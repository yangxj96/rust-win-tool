//! 网络诊断功能：DNS 解析、TCP 端口检查和 ICMP Ping。
//!
//! DNS 解析和端口检查使用标准库；标准库不支持 ICMP，因此 Ping 通过 `IcmpSendEcho` 实现。

use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct PingSummary {
    pub sent: u32,
    pub received: u32,
    pub avg_ms: u32,
}

pub fn resolve(host: &str) -> Result<Vec<IpAddr>, String> {
    let addresses: Vec<SocketAddr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .collect();
    let mut ips: Vec<IpAddr> = addresses.into_iter().map(|address| address.ip()).collect();
    ips.dedup();
    if ips.is_empty() {
        Err("no address resolved".to_string())
    } else {
        Ok(ips)
    }
}

pub fn format_addresses(ips: &[IpAddr]) -> String {
    ips.iter()
        .map(|ip| ip.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn check_port(host: &str, port: u16, timeout: Duration) -> Result<bool, String> {
    let addresses: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .collect();
    if addresses.is_empty() {
        return Err("no address resolved".to_string());
    }
    for address in addresses {
        if TcpStream::connect_timeout(&address, timeout).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(windows)]
pub fn ping(host: &str, count: u32, timeout_ms: u32) -> Result<PingSummary, String> {
    let ips = resolve(host)?;
    let ipv4 = ips
        .iter()
        .find_map(|ip| match ip {
            IpAddr::V4(v4) => Some(*v4),
            IpAddr::V6(_) => None,
        })
        .ok_or_else(|| "only IPv4 ping is supported".to_string())?;
    platform::ping_ipv4(ipv4, count, timeout_ms)
}

#[cfg(not(windows))]
pub fn ping(_host: &str, _count: u32, _timeout_ms: u32) -> Result<PingSummary, String> {
    Err("unsupported platform".to_string())
}

#[cfg(windows)]
mod platform {
    use super::PingSummary;
    use std::ffi::c_void;
    use std::net::Ipv4Addr;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY,
    };

    pub fn ping_ipv4(ip: Ipv4Addr, count: u32, timeout_ms: u32) -> Result<PingSummary, String> {
        let handle = unsafe { IcmpCreateFile() };
        if handle.is_null() {
            return Err("IcmpCreateFile failed".to_string());
        }

        let destination = u32::from_ne_bytes(ip.octets());
        let payload = [0u8; 32];
        let mut buffer = [0u8; 128];
        let mut received = 0u32;
        let mut total_ms = 0u32;

        for _ in 0..count {
            let replies = unsafe {
                IcmpSendEcho(
                    handle,
                    destination,
                    payload.as_ptr().cast::<c_void>(),
                    payload.len() as u16,
                    std::ptr::null(),
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer.len() as u32,
                    timeout_ms,
                )
            };
            if replies > 0 {
                let reply = unsafe { &*(buffer.as_ptr() as *const ICMP_ECHO_REPLY) };
                if reply.Status == 0 {
                    received += 1;
                    total_ms += reply.RoundTripTime;
                }
            }
        }

        unsafe { IcmpCloseHandle(handle) };
        Ok(PingSummary {
            sent: count,
            received,
            avg_ms: total_ms.checked_div(received).unwrap_or(0),
        })
    }
}

/// 使用 `ipconfig` 清空 Windows DNS 解析器缓存。
pub fn flush_dns() -> Result<String, crate::shared::BackendError> {
    crate::features::scripts::runtime::run_command("ipconfig /flushdns")
}
