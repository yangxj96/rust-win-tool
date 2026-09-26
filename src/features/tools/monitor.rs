//! 系统资源采样：CPU、内存、磁盘和网络吞吐量。
//!
//! CPU 和网络速率需要前一次采样数据。数据保存在进程级全局状态中，调用方只需定期轮询
//! `sample()`。

#[derive(Debug, Clone, Default)]
pub struct DiskUsage {
    pub name: String,
    pub free: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disks: Vec<DiskUsage>,
    /// 网络每秒接收和发送的字节数。
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
}

#[cfg(windows)]
mod platform {
    use super::{DiskUsage, Metrics};
    use std::sync::Mutex;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
    };
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    struct CpuSample {
        idle: u64,
        total: u64,
    }

    static PREVIOUS_CPU: Mutex<Option<CpuSample>> = Mutex::new(None);
    static PREVIOUS_NET: Mutex<Option<(u64, u64, Instant)>> = Mutex::new(None);

    fn filetime_to_u64(time: FILETIME) -> u64 {
        ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
    }

    fn cpu_percent() -> f32 {
        let mut idle: FILETIME = unsafe { std::mem::zeroed() };
        let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
        let mut user: FILETIME = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
        if ok == 0 {
            return 0.0;
        }
        let idle = filetime_to_u64(idle);
        let total = filetime_to_u64(kernel) + filetime_to_u64(user);

        let mut previous = PREVIOUS_CPU.lock().unwrap();
        let percent = if let Some(before) = previous.as_ref() {
            let idle_delta = idle.saturating_sub(before.idle) as f64;
            let total_delta = total.saturating_sub(before.total) as f64;
            if total_delta > 0.0 {
                ((total_delta - idle_delta) / total_delta * 100.0) as f32
            } else {
                0.0
            }
        } else {
            0.0
        };
        *previous = Some(CpuSample { idle, total });
        percent.clamp(0.0, 100.0)
    }

    fn memory() -> (f32, u64, u64) {
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..unsafe { std::mem::zeroed() }
        };
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok == 0 {
            return (0.0, 0, 0);
        }
        let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
        (status.dwMemoryLoad as f32, used, status.ullTotalPhys)
    }

    fn disks() -> Vec<DiskUsage> {
        let mut result = Vec::new();
        for letter in b'C'..=b'Z' {
            let name = format!("{}:", letter as char);
            let wide: Vec<u16> = format!("{name}\\")
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut free = 0u64;
            let mut total = 0u64;
            let ok = unsafe {
                GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, std::ptr::null_mut())
            };
            if ok != 0 && total > 0 {
                result.push(DiskUsage { name, free, total });
            }
        }
        result
    }

    fn network() -> (f64, f64) {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        let status = unsafe { GetIfTable2(&mut table) };
        if status != 0 || table.is_null() {
            return (0.0, 0.0);
        }
        let (mut rx, mut tx) = (0u64, 0u64);
        unsafe {
            let rows = (*table).Table.as_ptr();
            for index in 0..(*table).NumEntries as usize {
                let row = &*rows.add(index);
                // 跳过环回接口和当前未启用的网络接口。
                if row.Type == 24 || row.OperStatus != 1 {
                    continue;
                }
                rx += row.InOctets;
                tx += row.OutOctets;
            }
            FreeMibTable(table as *const core::ffi::c_void);
        }

        let now = Instant::now();
        let mut previous = PREVIOUS_NET.lock().unwrap();
        let (rx_bps, tx_bps) = if let Some((prev_rx, prev_tx, prev_at)) = previous.as_ref() {
            let seconds = now.duration_since(*prev_at).as_secs_f64().max(0.001);
            (
                rx.saturating_sub(*prev_rx) as f64 / seconds,
                tx.saturating_sub(*prev_tx) as f64 / seconds,
            )
        } else {
            (0.0, 0.0)
        };
        *previous = Some((rx, tx, now));
        (rx_bps, tx_bps)
    }

    pub fn sample() -> Metrics {
        let (memory_percent, memory_used, memory_total) = memory();
        let (net_rx_bps, net_tx_bps) = network();
        Metrics {
            cpu_percent: cpu_percent(),
            memory_percent,
            memory_used,
            memory_total,
            disks: disks(),
            net_rx_bps,
            net_tx_bps,
        }
    }
}

#[cfg(windows)]
pub use platform::sample;

#[cfg(not(windows))]
pub fn sample() -> Metrics {
    Metrics::default()
}

pub fn format_rate(bytes_per_second: f64) -> String {
    const KB: f64 = 1024.0;
    if bytes_per_second < KB {
        format!("{bytes_per_second:.0} B/s")
    } else if bytes_per_second < KB * KB {
        format!("{:.1} KB/s", bytes_per_second / KB)
    } else {
        format!("{:.2} MB/s", bytes_per_second / (KB * KB))
    }
}

/// 为实时监视器采集一份系统指标数据。
pub fn sample_metrics() -> Result<Metrics, crate::shared::BackendError> {
    Ok(sample())
}
