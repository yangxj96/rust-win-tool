use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub start_type: String,
    /// Currently unused; kept for JSON backward compatibility
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedService {
    pub name: String,
    pub display_name: String,
    /// Currently unused; kept for JSON backward compatibility
    pub enabled: bool,
}

/// Service backend errors.
///
/// The `Display` text intentionally contains the English keywords that the UI
/// error mapper (`map_error` in `src/app.rs`) already recognizes, so the user
/// facing messages stay identical after moving off PowerShell.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("access is denied")]
    AccessDenied,
    #[error("service not found")]
    NotFound,
    #[error("already running")]
    AlreadyRunning,
    #[error("not started")]
    NotStarted,
    #[error("cannot be stopped")]
    CannotStop,
    #[error("service disabled")]
    Disabled,
    #[error("timeout")]
    Timeout,
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("命令失败")]
    CommandFailed(String),
    #[error("启动失败")]
    StartFailed(String),
    #[error("停止失败")]
    StopFailed(String),
    #[error("解析失败")]
    ParseFailed(String),
}

pub fn load_managed_services(path: &std::path::Path) -> Vec<ManagedService> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_managed_services(path: &std::path::Path, services: &[ManagedService]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(services) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(windows)]
mod native {
    use super::{ServiceError, ServiceInfo};
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_ACCESS_DENIED, ERROR_MORE_DATA, ERROR_SERVICE_ALREADY_RUNNING,
        ERROR_SERVICE_CANNOT_ACCEPT_CTRL, ERROR_SERVICE_DISABLED, ERROR_SERVICE_DOES_NOT_EXIST,
        ERROR_SERVICE_NOT_ACTIVE, ERROR_SERVICE_REQUEST_TIMEOUT,
    };
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, ControlService, EnumServicesStatusExW, OpenSCManagerW, OpenServiceW,
        QueryServiceStatusEx, StartServiceW, ENUM_SERVICE_STATUS_PROCESSW, SC_ENUM_PROCESS_INFO,
        SC_HANDLE, SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SC_STATUS_PROCESS_INFO,
        SERVICE_CONTINUE_PENDING, SERVICE_CONTROL_STOP, SERVICE_PAUSE_PENDING, SERVICE_PAUSED,
        SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING,
        SERVICE_STATE_ALL, SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
        SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_WIN32,
    };

    /// RAII wrapper so a service handle is always released.
    struct Handle(SC_HANDLE);

    impl Handle {
        fn new(handle: SC_HANDLE) -> Result<Self, ServiceError> {
            if handle.is_null() {
                Err(last_error())
            } else {
                Ok(Handle(handle))
            }
        }

        fn raw(&self) -> SC_HANDLE {
            self.0
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseServiceHandle(self.0) };
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error() -> ServiceError {
        classify(unsafe { GetLastError() })
    }

    /// Map a Win32 error code onto the user-facing error variants.
    pub(super) fn classify(code: u32) -> ServiceError {
        match code {
            ERROR_ACCESS_DENIED => ServiceError::AccessDenied,
            ERROR_SERVICE_DOES_NOT_EXIST => ServiceError::NotFound,
            ERROR_SERVICE_ALREADY_RUNNING => ServiceError::AlreadyRunning,
            ERROR_SERVICE_NOT_ACTIVE => ServiceError::NotStarted,
            ERROR_SERVICE_CANNOT_ACCEPT_CTRL => ServiceError::CannotStop,
            ERROR_SERVICE_DISABLED => ServiceError::Disabled,
            ERROR_SERVICE_REQUEST_TIMEOUT => ServiceError::Timeout,
            other => ServiceError::CommandFailed(format!("windows error {other}")),
        }
    }

    /// Translate `SERVICE_STATUS.dwCurrentState` into the strings the UI parses.
    pub(super) fn state_to_status(state: u32) -> &'static str {
        match state {
            SERVICE_RUNNING => "Running",
            SERVICE_STOPPED => "Stopped",
            SERVICE_START_PENDING => "Start Pending",
            SERVICE_STOP_PENDING => "Stop Pending",
            SERVICE_PAUSED => "Paused",
            SERVICE_PAUSE_PENDING => "Pause Pending",
            SERVICE_CONTINUE_PENDING => "Continue Pending",
            _ => "Unknown",
        }
    }

    fn open_manager(access: u32) -> Result<Handle, ServiceError> {
        Handle::new(unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), access) })
    }

    fn open_service(manager: SC_HANDLE, name: &str, access: u32) -> Result<Handle, ServiceError> {
        let name = wide(name);
        Handle::new(unsafe { OpenServiceW(manager, name.as_ptr(), access) })
    }

    fn query_state(service: SC_HANDLE) -> Result<u32, ServiceError> {
        let mut status: SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
        let mut needed = 0u32;
        let ok = unsafe {
            QueryServiceStatusEx(
                service,
                SC_STATUS_PROCESS_INFO,
                (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
                &mut needed,
            )
        };
        if ok == 0 {
            return Err(last_error());
        }
        Ok(status.dwCurrentState)
    }

    pub fn get_service_status(name: &str) -> Result<String, ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_QUERY_STATUS)?;
        Ok(state_to_status(query_state(service.raw())?).to_string())
    }

    pub fn start_service(name: &str) -> Result<(), ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_START | SERVICE_QUERY_STATUS)?;
        let started = unsafe { StartServiceW(service.raw(), 0, std::ptr::null()) };
        if started == 0 {
            let error = last_error();
            // Already running is a success from the caller's point of view.
            if matches!(error, ServiceError::AlreadyRunning) {
                return Ok(());
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn stop_service(name: &str) -> Result<(), ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
        let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
        let stopped = unsafe { ControlService(service.raw(), SERVICE_CONTROL_STOP, &mut status) };
        if stopped == 0 {
            let error = last_error();
            // Already stopped is a success from the caller's point of view.
            if matches!(error, ServiceError::NotStarted) {
                return Ok(());
            }
            return Err(error);
        }
        Ok(())
    }

    fn pwstr_to_string(pointer: *const u16) -> String {
        if pointer.is_null() {
            return String::new();
        }
        let mut length = 0usize;
        unsafe {
            while *pointer.add(length) != 0 {
                length += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length))
        }
    }

    pub fn list_all_services() -> Result<Vec<ServiceInfo>, ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE)?;

        // `Vec<usize>` gives the 8-byte alignment `ENUM_SERVICE_STATUS_PROCESSW`
        // requires, which a plain `Vec<u8>` would not guarantee.
        let mut buffer: Vec<usize> = vec![0; 8 * 1024];
        let mut returned = 0u32;

        loop {
            let mut needed = 0u32;
            let mut resume = 0u32;
            let ok = unsafe {
                EnumServicesStatusExW(
                    manager.raw(),
                    SC_ENUM_PROCESS_INFO,
                    SERVICE_WIN32,
                    SERVICE_STATE_ALL,
                    buffer.as_mut_ptr().cast::<u8>(),
                    (buffer.len() * std::mem::size_of::<usize>()) as u32,
                    &mut needed,
                    &mut returned,
                    &mut resume,
                    std::ptr::null(),
                )
            };
            if ok != 0 {
                break;
            }
            let code = unsafe { GetLastError() };
            if code == ERROR_MORE_DATA {
                let units = (needed as usize + std::mem::size_of::<usize>() - 1)
                    / std::mem::size_of::<usize>();
                buffer.resize(units.max(8 * 1024), 0);
                continue;
            }
            return Err(classify(code));
        }

        let entries = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
        let mut services = Vec::with_capacity(returned as usize);
        for index in 0..returned as usize {
            let entry = unsafe { &*entries.add(index) };
            services.push(ServiceInfo {
                name: pwstr_to_string(entry.lpServiceName),
                display_name: pwstr_to_string(entry.lpDisplayName),
                status: state_to_status(entry.ServiceStatusProcess.dwCurrentState).to_string(),
                start_type: String::new(),
                description: String::new(),
            });
        }

        services.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
        Ok(services)
    }
}

#[cfg(windows)]
pub use native::{get_service_status, list_all_services, start_service, stop_service};

#[cfg(not(windows))]
pub fn list_all_services() -> Result<Vec<ServiceInfo>, ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn get_service_status(_name: &str) -> Result<String, ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn start_service(_name: &str) -> Result<(), ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn stop_service(_name: &str) -> Result<(), ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(all(test, windows))]
mod tests {
    use super::native::{classify, state_to_status};
    use super::ServiceError;
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST,
        ERROR_SERVICE_NOT_ACTIVE,
    };
    use windows_sys::Win32::System::Services::{
        SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STOPPED,
    };

    #[test]
    fn maps_error_codes_to_semantic_variants() {
        assert!(matches!(classify(ERROR_ACCESS_DENIED), ServiceError::AccessDenied));
        assert!(matches!(
            classify(ERROR_SERVICE_DOES_NOT_EXIST),
            ServiceError::NotFound
        ));
        assert!(matches!(
            classify(ERROR_SERVICE_ALREADY_RUNNING),
            ServiceError::AlreadyRunning
        ));
        assert!(matches!(
            classify(ERROR_SERVICE_NOT_ACTIVE),
            ServiceError::NotStarted
        ));
    }

    #[test]
    fn maps_state_codes_to_ui_status_strings() {
        assert_eq!(state_to_status(SERVICE_RUNNING), "Running");
        assert_eq!(state_to_status(SERVICE_STOPPED), "Stopped");
        assert_eq!(state_to_status(SERVICE_START_PENDING), "Start Pending");
    }
}
