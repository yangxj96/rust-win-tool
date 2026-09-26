//! Windows 服务控制管理器的原生访问接口及跨平台服务数据类型。
//!
//! Win32 句柄通过包装类型确保及时释放；查询缓冲区使用按 `usize` 对齐的存储，
//! 再转换为 Windows API 返回的结构体。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 服务信息字段，与旧版服务 JSON 文件保持兼容。
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub start_type: String,
    /// 当前未使用；为兼容旧版 JSON 保留。
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// 应用保存的用户托管服务条目。
pub struct ManagedService {
    pub name: String,
    pub display_name: String,
    /// 当前未使用；为兼容旧版 JSON 保留。
    pub enabled: bool,
}

/// 服务启动类型，对应 Win32 的 `SERVICE_*_START` 数值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartType {
    Automatic,
    Manual,
    Disabled,
    Unknown,
}

impl StartType {
    /// 转换为服务控制管理器要求的数值。
    pub fn code(self) -> u32 {
        match self {
            Self::Automatic => 2,
            Self::Manual => 3,
            Self::Disabled => 4,
            Self::Unknown => 0,
        }
    }

    /// 将服务控制管理器的启动数值映射为界面使用的类型。
    pub fn from_code(code: u32) -> Self {
        match code {
            2 => Self::Automatic,
            3 => Self::Manual,
            4 => Self::Disabled,
            _ => Self::Unknown,
        }
    }
}

/// 单个服务的详细配置和运行时信息。
#[derive(Debug, Clone)]
pub struct ServiceDetails {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub status: String,
    pub start_type: StartType,
    pub binary_path: String,
    pub account: String,
    pub process_id: u32,
    /// 当前服务依赖的其他服务。
    pub depends_on: Vec<String>,
    /// 依赖当前服务的其他服务；停止当前服务时这些服务也会受影响。
    pub dependents: Vec<String>,
}

/// 服务后端错误。
///
/// `Display` 文本保留界面错误映射器（`src/app.rs` 中的 `map_error`）识别的英文
/// 关键词，确保替换 PowerShell 实现后用户看到的提示不变。
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
    #[cfg(not(windows))]
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("命令失败")]
    CommandFailed(String),
}

/// 读取兼容旧版格式的托管服务 JSON 文件；读取或解析失败时返回空列表。
pub fn load_managed_services(path: &std::path::Path) -> Vec<ManagedService> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 使用现有的缩进 JSON 格式保存托管服务列表。
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
    use super::{ServiceDetails, ServiceError, ServiceInfo, StartType};
    use windows_sys::Win32::Foundation::{
        GetLastError, ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA,
        ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_CANNOT_ACCEPT_CTRL, ERROR_SERVICE_DISABLED,
        ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_NOT_ACTIVE, ERROR_SERVICE_REQUEST_TIMEOUT,
    };
    use windows_sys::Win32::System::Services::{
        ChangeServiceConfigW, CloseServiceHandle, ControlService, EnumDependentServicesW,
        EnumServicesStatusExW, OpenSCManagerW, OpenServiceW, QueryServiceConfig2W,
        QueryServiceConfigW, QueryServiceStatusEx, StartServiceW, ENUM_SERVICE_STATUSW,
        ENUM_SERVICE_STATUS_PROCESSW, QUERY_SERVICE_CONFIGW, SC_ENUM_PROCESS_INFO, SC_HANDLE,
        SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SC_STATUS_PROCESS_INFO,
        SERVICE_CHANGE_CONFIG, SERVICE_CONFIG_DESCRIPTION, SERVICE_CONTINUE_PENDING,
        SERVICE_CONTROL_STOP, SERVICE_DESCRIPTIONW, SERVICE_NO_CHANGE, SERVICE_PAUSED,
        SERVICE_PAUSE_PENDING, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
        SERVICE_START, SERVICE_START_PENDING, SERVICE_STATE_ALL, SERVICE_STATUS,
        SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOPPED, SERVICE_STOP_PENDING, SERVICE_WIN32,
    };

    /// RAII 句柄包装器，确保服务句柄在离开作用域时释放。
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

    /// 将 Win32 错误码映射为界面可识别的错误类型。
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

    /// 将 `SERVICE_STATUS.dwCurrentState` 转换为界面解析的状态文本。
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

    /// 在服务管理器句柄有效期间查询单个服务状态。
    pub fn get_service_status(name: &str) -> Result<String, ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_QUERY_STATUS)?;
        Ok(state_to_status(query_state(service.raw())?).to_string())
    }

    /// 请求服务控制管理器启动指定服务。
    pub fn start_service(name: &str) -> Result<(), ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_START | SERVICE_QUERY_STATUS)?;
        let started = unsafe { StartServiceW(service.raw(), 0, std::ptr::null()) };
        if started == 0 {
            let error = last_error();
            // 对调用方而言，服务已经在运行即视为启动成功。
            if matches!(error, ServiceError::AlreadyRunning) {
                return Ok(());
            }
            return Err(error);
        }
        Ok(())
    }

    /// 请求服务控制管理器停止指定服务。
    pub fn stop_service(name: &str) -> Result<(), ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(manager.raw(), name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
        let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
        let stopped = unsafe { ControlService(service.raw(), SERVICE_CONTROL_STOP, &mut status) };
        if stopped == 0 {
            let error = last_error();
            // 对调用方而言，服务已经停止即视为停止成功。
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

    /// 将双空字符结尾的多字符串拆分为独立字符串。
    fn multi_string(pointer: *const u16) -> Vec<String> {
        if pointer.is_null() {
            return Vec::new();
        }
        let mut values = Vec::new();
        let mut cursor = pointer;
        loop {
            let mut length = 0usize;
            unsafe {
                while *cursor.add(length) != 0 {
                    length += 1;
                }
            }
            if length == 0 {
                break;
            }
            values.push(unsafe {
                String::from_utf16_lossy(std::slice::from_raw_parts(cursor, length))
            });
            cursor = unsafe { cursor.add(length + 1) };
        }
        values
    }

    /// 为 Win32 服务结构体提供 8 字节对齐的临时缓冲区。
    fn aligned_buffer(bytes: usize) -> Vec<usize> {
        let unit = std::mem::size_of::<usize>();
        vec![0usize; bytes.div_ceil(unit)]
    }

    fn buffer_bytes(buffer: &[usize]) -> u32 {
        std::mem::size_of_val(buffer) as u32
    }

    struct RawConfig {
        start_type: StartType,
        binary_path: String,
        account: String,
        display_name: String,
        dependencies: Vec<String>,
    }

    fn query_config(service: SC_HANDLE) -> Result<RawConfig, ServiceError> {
        let mut buffer = aligned_buffer(8 * 1024);
        loop {
            let mut needed = 0u32;
            let ok = unsafe {
                QueryServiceConfigW(
                    service,
                    buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW,
                    buffer_bytes(&buffer),
                    &mut needed,
                )
            };
            if ok != 0 {
                break;
            }
            let code = unsafe { GetLastError() };
            if code == ERROR_INSUFFICIENT_BUFFER && needed as usize > buffer_bytes(&buffer) as usize
            {
                buffer = aligned_buffer(needed as usize);
                continue;
            }
            return Err(classify(code));
        }
        let config = unsafe { &*(buffer.as_ptr() as *const QUERY_SERVICE_CONFIGW) };
        Ok(RawConfig {
            start_type: StartType::from_code(config.dwStartType),
            binary_path: pwstr_to_string(config.lpBinaryPathName),
            account: pwstr_to_string(config.lpServiceStartName),
            display_name: pwstr_to_string(config.lpDisplayName),
            dependencies: multi_string(config.lpDependencies),
        })
    }

    fn query_description(service: SC_HANDLE) -> String {
        let mut buffer = aligned_buffer(4 * 1024);
        let mut needed = 0u32;
        let ok = unsafe {
            QueryServiceConfig2W(
                service,
                SERVICE_CONFIG_DESCRIPTION,
                buffer.as_mut_ptr().cast::<u8>(),
                buffer_bytes(&buffer),
                &mut needed,
            )
        };
        if ok == 0 {
            return String::new();
        }
        let description = unsafe { &*(buffer.as_ptr() as *const SERVICE_DESCRIPTIONW) };
        pwstr_to_string(description.lpDescription)
    }

    fn query_dependents(service: SC_HANDLE) -> Vec<String> {
        let mut buffer = aligned_buffer(8 * 1024);
        let mut returned = 0u32;
        loop {
            let mut needed = 0u32;
            let ok = unsafe {
                EnumDependentServicesW(
                    service,
                    SERVICE_STATE_ALL,
                    buffer.as_mut_ptr() as *mut ENUM_SERVICE_STATUSW,
                    buffer_bytes(&buffer),
                    &mut needed,
                    &mut returned,
                )
            };
            if ok != 0 {
                break;
            }
            let code = unsafe { GetLastError() };
            if code == ERROR_MORE_DATA && needed as usize > buffer_bytes(&buffer) as usize {
                buffer = aligned_buffer(needed as usize);
                continue;
            }
            return Vec::new();
        }
        let entries = buffer.as_ptr() as *const ENUM_SERVICE_STATUSW;
        (0..returned as usize)
            .map(|index| {
                let entry = unsafe { &*entries.add(index) };
                pwstr_to_string(entry.lpServiceName)
            })
            .collect()
    }

    /// 通过 Win32 API 查询服务配置和进程信息。
    pub fn get_service_details(name: &str) -> Result<ServiceDetails, ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(
            manager.raw(),
            name,
            SERVICE_QUERY_STATUS | SERVICE_QUERY_CONFIG,
        )?;
        let config = query_config(service.raw())?;
        let state = query_state(service.raw())?;

        let mut status: SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
        let mut needed = 0u32;
        let process_id = unsafe {
            QueryServiceStatusEx(
                service.raw(),
                SC_STATUS_PROCESS_INFO,
                (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
                &mut needed,
            )
        };
        let process_id = if process_id != 0 {
            status.dwProcessId
        } else {
            0
        };

        let display_name = if config.display_name.is_empty() {
            name.to_string()
        } else {
            config.display_name
        };
        Ok(ServiceDetails {
            name: name.to_string(),
            display_name,
            description: query_description(service.raw()),
            status: state_to_status(state).to_string(),
            start_type: config.start_type,
            binary_path: config.binary_path,
            account: config.account,
            process_id,
            depends_on: config.dependencies,
            dependents: query_dependents(service.raw()),
        })
    }

    /// 通过服务控制管理器配置指定服务的启动类型。
    pub fn set_service_start_type(name: &str, start_type: StartType) -> Result<(), ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(
            manager.raw(),
            name,
            SERVICE_CHANGE_CONFIG | SERVICE_QUERY_CONFIG,
        )?;
        let changed = unsafe {
            ChangeServiceConfigW(
                service.raw(),
                SERVICE_NO_CHANGE,
                start_type.code(),
                SERVICE_NO_CHANGE,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if changed == 0 {
            return Err(last_error());
        }
        Ok(())
    }

    /// 从服务控制管理器枚举服务元数据。
    pub fn list_all_services() -> Result<Vec<ServiceInfo>, ServiceError> {
        let manager = open_manager(SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE)?;

        // `Vec<usize>` 能满足 `ENUM_SERVICE_STATUS_PROCESSW` 所需的 8 字节对齐；
        // 普通的 `Vec<u8>` 不保证这一点。
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
                let units = (needed as usize).div_ceil(std::mem::size_of::<usize>());
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
pub use native::{
    get_service_details, get_service_status, list_all_services, set_service_start_type,
    start_service, stop_service,
};

#[cfg(not(windows))]
/// 查询 Windows 服务控制管理器中的服务配置和运行时信息。
pub fn get_service_details(name: &str) -> Result<ServiceDetails, ServiceError> {
    let _ = name;
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
/// 当前平台不支持修改服务配置时返回平台错误。
pub fn set_service_start_type(_name: &str, _start_type: StartType) -> Result<(), ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
/// 通过 Windows 服务控制管理器枚举服务。
pub fn list_all_services() -> Result<Vec<ServiceInfo>, ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
/// 根据服务内部名称返回当前状态文本。
pub fn get_service_status(_name: &str) -> Result<String, ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
/// 请求服务控制管理器启动指定服务。
pub fn start_service(_name: &str) -> Result<(), ServiceError> {
    Err(ServiceError::UnsupportedPlatform)
}

#[cfg(not(windows))]
/// 请求服务控制管理器停止指定服务。
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
        assert!(matches!(
            classify(ERROR_ACCESS_DENIED),
            ServiceError::AccessDenied
        ));
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
