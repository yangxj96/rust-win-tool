//! Process listing and termination.
//!
//! CPU usage is derived from two `GetProcessTimes` samples taken a short time
//! apart, so no long-lived sampling state is required.

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    /// Total CPU time in 100ns units; used while computing `cpu_percent`.
    pub cpu_time: u64,
}

#[cfg(windows)]
mod platform {
    use super::ProcessInfo;
    use std::mem::size_of;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
    use windows_sys::Win32::System::ProcessStatus::{
        EnumProcesses, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ,
    };

    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    fn filetime_to_u64(time: FILETIME) -> u64 {
        ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
    }

    fn open(pid: u32, access: u32) -> Option<Handle> {
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() {
            None
        } else {
            Some(Handle(handle))
        }
    }

    fn process_name(handle: HANDLE) -> String {
        let mut buffer = vec![0u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string()
    }

    fn process_cpu_time(handle: HANDLE) -> Option<u64> {
        let mut creation: FILETIME = unsafe { std::mem::zeroed() };
        let mut exit: FILETIME = unsafe { std::mem::zeroed() };
        let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
        let mut user: FILETIME = unsafe { std::mem::zeroed() };
        let ok =
            unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
        if ok == 0 {
            return None;
        }
        Some(filetime_to_u64(kernel) + filetime_to_u64(user))
    }

    fn process_memory(handle: HANDLE) -> u64 {
        let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = unsafe {
            GetProcessMemoryInfo(
                handle,
                &mut counters,
                size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            )
        };
        if ok == 0 {
            0
        } else {
            counters.WorkingSetSize as u64
        }
    }

    fn query(pid: u32) -> Option<ProcessInfo> {
        let handle = open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ)?;
        Some(ProcessInfo {
            pid,
            name: process_name(handle.0),
            memory_bytes: process_memory(handle.0),
            cpu_percent: 0.0,
            cpu_time: process_cpu_time(handle.0).unwrap_or(0),
        })
    }

    fn query_cpu_time(pid: u32) -> Option<u64> {
        let handle = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        process_cpu_time(handle.0)
    }

    fn enumerate_pids() -> Vec<u32> {
        let mut buffer = vec![0u32; 4096];
        loop {
            let mut needed = 0u32;
            let ok = unsafe {
                EnumProcesses(
                    buffer.as_mut_ptr(),
                    (buffer.len() * size_of::<u32>()) as u32,
                    &mut needed,
                )
            };
            if ok == 0 {
                return Vec::new();
            }
            let count = needed as usize / size_of::<u32>();
            if count < buffer.len() {
                buffer.truncate(count);
                return buffer;
            }
            buffer.resize(count + 1024, 0);
        }
    }

    pub fn list() -> Vec<ProcessInfo> {
        let cores = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1) as f64;
        let interval = Duration::from_millis(250);

        let first: Vec<ProcessInfo> = enumerate_pids()
            .into_iter()
            .filter(|&pid| pid != 0)
            .filter_map(query)
            .collect();

        std::thread::sleep(interval);

        let mut result: Vec<ProcessInfo> = first
            .into_iter()
            .map(|mut entry| {
                if let Some(second) = query_cpu_time(entry.pid) {
                    let delta = second.saturating_sub(entry.cpu_time);
                    entry.cpu_percent =
                        (delta as f64 / (interval.as_secs_f64() * 10_000_000.0) / cores * 100.0)
                            as f32;
                }
                entry
            })
            .collect();

        result.sort_by_key(|process| std::cmp::Reverse(process.memory_bytes));
        result
    }

    pub fn terminate(pid: u32) -> Result<(), String> {
        if pid == 0 || pid == 4 {
            return Err("protected process".to_string());
        }
        let handle = open(pid, PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION)
            .ok_or_else(|| "cannot open process".to_string())?;
        let ok = unsafe { TerminateProcess(handle.0, 1) };
        if ok == 0 {
            return Err("terminate failed".to_string());
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use platform::{list, terminate};

#[cfg(not(windows))]
pub fn list() -> Vec<ProcessInfo> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn terminate(_pid: u32) -> Result<(), String> {
    Err("unsupported platform".to_string())
}

pub fn format_memory(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    format!("{:.1} MB", bytes as f64 / MB)
}
