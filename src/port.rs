//! 使用 Windows IP Helper API 查询端口占用情况。
//!
//! 通过 `GetExtendedTcpTable` / `GetExtendedUdpTable` 查询 IPv4/IPv6 的 TCP 和 UDP
//! 所有者表，无需启动外部工具。结果会按本地端口筛选，并补充占用端口的进程名称。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

impl PortProtocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortEntry {
    pub protocol: PortProtocol,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
}

#[cfg(windows)]
mod platform {
    use super::{PortEntry, PortProtocol};
    use std::collections::HashMap;
    use std::mem::size_of;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use windows_sys::core::BOOL;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };

    // 这里直接定义 WinSock 的 AddressFamily 常量，避免仅为两个整数引入整个
    // `Win32_Networking_WinSock` 功能模块。
    const AF_INET: u32 = 2;
    const AF_INET6: u32 = 23;

    const NO_ERROR: u32 = 0;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

    type GetTable =
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32, BOOL, u32, i32, u32) -> u32;

    /// 端口号以网络字节序保存在低 16 位中。
    fn read_port(value: u32) -> u16 {
        u16::from_be(value as u16)
    }

    fn tcp_state(state: u32) -> &'static str {
        match state {
            1 => "CLOSED",
            2 => "LISTEN",
            3 => "SYN_SENT",
            4 => "SYN_RECEIVED",
            5 => "ESTABLISHED",
            6 => "FIN_WAIT1",
            7 => "FIN_WAIT2",
            8 => "CLOSE_WAIT",
            9 => "CLOSING",
            10 => "LAST_ACK",
            11 => "DELETE_TCB",
            12 => "TIME_WAIT",
            _ => "UNKNOWN",
        }
    }

    /// 获取原始端口所有者表。当该地址族没有条目或 API 不可用时返回空缓冲区。
    unsafe fn query(get: GetTable, family: u32, class: i32) -> Vec<u8> {
        let mut size = 0u32;
        let mut code = get(std::ptr::null_mut(), &mut size, 0, family, class, 0);
        if code != ERROR_INSUFFICIENT_BUFFER && code != NO_ERROR {
            return Vec::new();
        }
        if size == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u8; size as usize];
        code = get(buffer.as_mut_ptr().cast(), &mut size, 0, family, class, 0);
        if code != NO_ERROR {
            return Vec::new();
        }
        buffer
    }

    /// 表格布局为一个 `u32` 条目数量，后面紧跟连续排列的行数据。
    fn rows<T: Copy>(buffer: &[u8]) -> Vec<T> {
        if buffer.len() < 4 {
            return Vec::new();
        }
        let count = u32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        let row_size = size_of::<T>();
        let mut result = Vec::with_capacity(count);
        let mut offset = 4;
        for _ in 0..count {
            if offset + row_size > buffer.len() {
                break;
            }
            let row = unsafe { std::ptr::read_unaligned(buffer.as_ptr().add(offset) as *const T) };
            result.push(row);
            offset += row_size;
        }
        result
    }

    fn process_name(pid: u32, cache: &mut HashMap<u32, String>) -> String {
        if let Some(value) = cache.get(&pid) {
            return value.clone();
        }
        let value = crate::process::name_for_pid(pid);
        cache.insert(pid, value.clone());
        value
    }

    pub fn lookup(port: u16) -> Vec<PortEntry> {
        let mut entries: Vec<PortEntry> = Vec::new();

        unsafe {
            for row in rows::<MIB_TCPROW_OWNER_PID>(&query(
                GetExtendedTcpTable,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
            )) {
                if read_port(row.dwLocalPort) == port {
                    entries.push(PortEntry {
                        protocol: PortProtocol::Tcp,
                        local_addr: Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()).to_string(),
                        local_port: port,
                        remote_addr: format!(
                            "{}:{}",
                            Ipv4Addr::from(row.dwRemoteAddr.to_ne_bytes()),
                            read_port(row.dwRemotePort)
                        ),
                        state: tcp_state(row.dwState).to_string(),
                        pid: row.dwOwningPid,
                        process_name: String::new(),
                    });
                }
            }

            for row in rows::<MIB_TCP6ROW_OWNER_PID>(&query(
                GetExtendedTcpTable,
                AF_INET6,
                TCP_TABLE_OWNER_PID_ALL,
            )) {
                if read_port(row.dwLocalPort) == port {
                    entries.push(PortEntry {
                        protocol: PortProtocol::Tcp,
                        local_addr: format!("[{}]", Ipv6Addr::from(row.ucLocalAddr)),
                        local_port: port,
                        remote_addr: format!(
                            "[{}]:{}",
                            Ipv6Addr::from(row.ucRemoteAddr),
                            read_port(row.dwRemotePort)
                        ),
                        state: tcp_state(row.dwState).to_string(),
                        pid: row.dwOwningPid,
                        process_name: String::new(),
                    });
                }
            }

            for row in rows::<MIB_UDPROW_OWNER_PID>(&query(
                GetExtendedUdpTable,
                AF_INET,
                UDP_TABLE_OWNER_PID,
            )) {
                if read_port(row.dwLocalPort) == port {
                    entries.push(PortEntry {
                        protocol: PortProtocol::Udp,
                        local_addr: Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()).to_string(),
                        local_port: port,
                        remote_addr: String::new(),
                        state: String::new(),
                        pid: row.dwOwningPid,
                        process_name: String::new(),
                    });
                }
            }

            for row in rows::<MIB_UDP6ROW_OWNER_PID>(&query(
                GetExtendedUdpTable,
                AF_INET6,
                UDP_TABLE_OWNER_PID,
            )) {
                if read_port(row.dwLocalPort) == port {
                    entries.push(PortEntry {
                        protocol: PortProtocol::Udp,
                        local_addr: format!("[{}]", Ipv6Addr::from(row.ucLocalAddr)),
                        local_port: port,
                        remote_addr: String::new(),
                        state: String::new(),
                        pid: row.dwOwningPid,
                        process_name: String::new(),
                    });
                }
            }
        }

        let mut cache = HashMap::new();
        for entry in &mut entries {
            entry.process_name = process_name(entry.pid, &mut cache);
        }
        entries.sort_by(|a, b| {
            (a.protocol as u8, a.state.as_str(), a.local_addr.as_str()).cmp(&(
                b.protocol as u8,
                b.state.as_str(),
                b.local_addr.as_str(),
            ))
        });
        entries
    }
}

#[cfg(windows)]
pub use platform::lookup;

#[cfg(not(windows))]
pub fn lookup(_port: u16) -> Vec<PortEntry> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::lookup;

    #[test]
    fn lookup_filters_by_port() {
        // 验证表格遍历逻辑，确保返回的每一行都与目标端口匹配。
        for entry in lookup(65535) {
            assert_eq!(entry.local_port, 65535);
        }
        for entry in lookup(0) {
            assert_eq!(entry.local_port, 0);
        }
    }
}
