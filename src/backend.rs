//! 供 GPUI 后台任务调用的阻塞式平台操作。
//!
//! 本模块将原生服务、注册表和系统信息结果转换为应用数据类型；调用方必须把
//! 阻塞操作放在界面事件循环之外执行。

use crate::service::{ManagedService, ServiceError, ServiceInfo};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

use crate::scripts::ScriptKind;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 阻塞式后端操作返回的错误类型。
///
/// 服务错误保留稳定的原始关键词，`AppState` 会据此映射为本地化界面提示。
#[derive(Debug, Clone, thiserror::Error)]
pub enum BackendError {
    #[error("service operation failed: {0}")]
    Service(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("response parse failed: {0}")]
    Parse(String),
    #[error("registry operation failed: {0}")]
    Registry(String),
    #[cfg(not(windows))]
    #[error("this operation is only supported on Windows")]
    UnsupportedPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 系统信息工具展示的系统字段快照。
pub struct SystemInfo {
    pub os: String,
    pub version: String,
    pub build: String,
    pub computer: String,
    pub user: String,
    pub cpu: String,
    pub cores: String,
    pub ram: String,
}

/// 通过 Windows 服务控制管理器枚举服务。
pub fn list_services() -> Result<Vec<ServiceInfo>, BackendError> {
    crate::service::list_all_services().map_err(service_error)
}

/// 根据服务内部名称查询当前状态。
pub fn service_status(name: &str) -> Result<String, BackendError> {
    crate::service::get_service_status(name).map_err(service_error)
}

/// 启动服务；权限不足或服务状态不允许时返回错误。
pub fn start_service(name: &str) -> Result<(), BackendError> {
    crate::service::start_service(name).map_err(service_error)
}

/// 停止服务；权限不足或依赖关系不允许时返回错误。
pub fn stop_service(name: &str) -> Result<(), BackendError> {
    crate::service::stop_service(name).map_err(service_error)
}

/// 读取服务配置和运行时信息，供服务详情对话框展示。
pub fn service_details(name: &str) -> Result<crate::service::ServiceDetails, BackendError> {
    crate::service::get_service_details(name).map_err(service_error)
}

/// 通过服务控制管理器修改服务启动类型。
pub fn set_service_start_type(
    name: &str,
    start_type: crate::service::StartType,
) -> Result<(), BackendError> {
    crate::service::set_service_start_type(name, start_type).map_err(service_error)
}

/// 按现有 JSON 结构写出指定的托管服务列表。
pub fn export_managed_services(
    path: &Path,
    services: &[ManagedService],
) -> Result<(), BackendError> {
    let json = serde_json::to_string_pretty(services)
        .map_err(|error| BackendError::Parse(error.to_string()))?;
    std::fs::write(path, json).map_err(|error| BackendError::Command(error.to_string()))
}

/// 从导出文件读取托管服务；调用方负责用结果替换当前列表。
pub fn import_managed_services(path: &Path) -> Result<Vec<ManagedService>, BackendError> {
    let text =
        std::fs::read_to_string(path).map_err(|error| BackendError::Command(error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| BackendError::Parse(error.to_string()))
}

/// 枚举进程管理工具所需的运行中进程。
pub fn list_processes() -> Result<Vec<crate::process::ProcessInfo>, BackendError> {
    Ok(crate::process::list())
}

/// 根据 PID 结束进程；调用方必须沿用现有的二次确认流程。
pub fn terminate_process(pid: u32) -> Result<(), BackendError> {
    crate::process::terminate(pid).map_err(BackendError::Command)
}

/// 查找占用指定本地 TCP/UDP 端口的进程和网络端点。
pub fn lookup_port(port: u16) -> Result<Vec<crate::port::PortEntry>, BackendError> {
    Ok(crate::port::lookup(port))
}

/// 为实时监视器采集一份系统指标数据。
pub fn sample_metrics() -> Result<crate::monitor::Metrics, BackendError> {
    Ok(crate::monitor::sample())
}

/// 从当前用户和计算机范围内的受支持位置枚举启动项。
pub fn list_startup() -> Result<Vec<crate::startup::StartupItem>, BackendError> {
    Ok(crate::startup::list())
}

/// 通过 `cmd.exe` 运行命令并捕获标准输出和标准错误。
///
/// 在 Windows 上隐藏子进程控制台窗口。此调用会阻塞，必须放在 GPUI 后台任务中。
pub fn run_command(command: &str) -> Result<String, BackendError> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|error| BackendError::Command(error.to_string()))?;
    let mut text = decode_console(&output.stdout);
    let stderr = decode_console(&output.stderr);
    if !stderr.trim().is_empty() {
        text.push('\n');
        text.push_str(&stderr);
    }
    if !output.status.success() {
        return Err(BackendError::Command(text.trim().to_string()));
    }
    Ok(text.trim().to_string())
}

/// 将用户脚本写入临时文件，再调用对应解释器执行。
///
/// 使用临时文件可以完整保留多行内容，避免 shell 引号转义问题。子进程继承应用
/// 当前用户身份和提权上下文。
pub fn run_script(kind: ScriptKind, content: &str) -> Result<String, BackendError> {
    #[cfg(windows)]
    {
        use std::io::Write;

        let (extension, program, args): (&str, &str, &[&str]) = match kind {
            ScriptKind::Cmd => (".cmd", "cmd", &["/C"]),
            ScriptKind::PowerShell => (
                ".ps1",
                "powershell",
                &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"],
            ),
        };
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        path.push(format!(
            "rust-win-tool-{}-{}{}",
            std::process::id(),
            stamp,
            extension
        ));
        {
            let mut file = std::fs::File::create(&path)
                .map_err(|error| BackendError::Command(error.to_string()))?;
            file.write_all(content.as_bytes())
                .map_err(|error| BackendError::Command(error.to_string()))?;
        }
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.arg(&path);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output();
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|error| BackendError::Command(error.to_string()))?;
        let mut text = decode_console(&output.stdout);
        let stderr = decode_console(&output.stderr);
        if !stderr.trim().is_empty() {
            text.push('\n');
            text.push_str(&stderr);
        }
        if !output.status.success() {
            return Err(BackendError::Command(text.trim().to_string()));
        }
        Ok(text.trim().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = kind;
        run_command(content)
    }
}

/// 控制台程序输出使用 OEM 代码页，而不是 UTF-8。
#[cfg(windows)]
fn decode_console(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::MultiByteToWideChar;

    const CP_OEMCP: u32 = 1;
    if bytes.is_empty() {
        return String::new();
    }
    let length = unsafe {
        MultiByteToWideChar(
            CP_OEMCP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    };
    if length <= 0 {
        return String::from_utf8_lossy(bytes).to_string();
    }
    let mut wide = vec![0u16; length as usize];
    let written = unsafe {
        MultiByteToWideChar(
            CP_OEMCP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            length,
        )
    };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).to_string();
    }
    wide.truncate(written as usize);
    String::from_utf16_lossy(&wide)
}

#[cfg(not(windows))]
fn decode_console(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// 使用 `ipconfig` 清空 Windows DNS 解析器缓存。
pub fn flush_dns() -> Result<String, BackendError> {
    run_command("ipconfig /flushdns")
}

/// 在原有注册表或文件位置范围内启用或禁用一个受支持的启动项。
pub fn set_startup_enabled(
    location: crate::startup::StartupLocation,
    value_name: &str,
    enabled: bool,
) -> Result<(), BackendError> {
    crate::startup::set_enabled(location, value_name, enabled).map_err(BackendError::Command)
}

#[derive(Debug, Clone)]
/// 同步服务操作请求；批量操作按输入顺序处理。
pub enum ServiceOperation {
    Start(String),
    Stop(String),
    StartAll(Vec<String>),
    StopAll(Vec<String>),
    RefreshAll(Vec<String>),
}

#[derive(Debug)]
/// 批量操作中单个服务对应的状态或错误。
pub struct ServiceOperationResult {
    pub name: String,
    pub status: Result<String, BackendError>,
}

/// 执行单项或批量服务操作，并返回每个服务的最终状态。
///
/// 此函数同步调用 Windows API，必须从后台任务调用，不能阻塞 GPUI 事件循环。
pub fn execute_service_operation(operation: ServiceOperation) -> Vec<ServiceOperationResult> {
    match operation {
        ServiceOperation::Start(name) => vec![execute_one(&name, start_service)],
        ServiceOperation::Stop(name) => vec![execute_one(&name, stop_service)],
        ServiceOperation::StartAll(names) => names
            .iter()
            .map(|name| execute_one(name, start_service))
            .collect(),
        ServiceOperation::StopAll(names) => names
            .iter()
            .map(|name| execute_one(name, stop_service))
            .collect(),
        ServiceOperation::RefreshAll(names) => names
            .iter()
            .map(|name| {
                let status = service_status(name);
                ServiceOperationResult {
                    name: name.clone(),
                    status,
                }
            })
            .collect(),
    }
}

fn execute_one(name: &str, action: fn(&str) -> Result<(), BackendError>) -> ServiceOperationResult {
    let action_result = action(name);
    let status = match action_result {
        Ok(()) => service_status(name),
        Err(error) => Err(error),
    };
    ServiceOperationResult {
        name: name.to_string(),
        status,
    }
}

fn service_error(error: ServiceError) -> BackendError {
    BackendError::Service(error.to_string())
}

/// 通过隐藏的 PowerShell 进程获取系统信息快照。
///
/// 获取 CIM 数据期间会阻塞，必须在界面事件循环之外执行。
pub fn fetch_system_info() -> Result<SystemInfo, BackendError> {
    let script = r#"
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
        $os = Get-CimInstance Win32_OperatingSystem
        $cpu = Get-CimInstance Win32_Processor
        $sys = Get-CimInstance Win32_ComputerSystem
        @{
            OS = $os.Caption
            Version = $os.Version
            Build = $os.BuildNumber
            Computer = $env:COMPUTERNAME
            User = $env:USERNAME
            CPU = $cpu.Name
            Cores = $cpu.NumberOfCores
            RAM = [math]::Round($sys.TotalPhysicalMemory / 1GB, 2)
        } | ConvertTo-Json -Compress
    "#;

    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-Command", script]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|error| BackendError::Command(error.to_string()))?;
    if !output.status.success() {
        return Err(BackendError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    parse_system_info_json(&String::from_utf8_lossy(&output.stdout))
}

/// 解析系统信息脚本输出的标准化 JSON 对象。
pub fn parse_system_info_json(input: &str) -> Result<SystemInfo, BackendError> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| BackendError::Parse(error.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| BackendError::Parse("expected a JSON object".to_string()))?;

    Ok(SystemInfo {
        os: value_as_text(object.get("OS")),
        version: value_as_text(object.get("Version")),
        build: value_as_text(object.get("Build")),
        computer: value_as_text(object.get("Computer")),
        user: value_as_text(object.get("User")),
        cpu: value_as_text(object.get("CPU")),
        cores: value_as_text(object.get("Cores")),
        ram: value_as_text(object.get("RAM")),
    })
}

fn value_as_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => "N/A".to_string(),
    }
}

#[cfg(windows)]
/// 删除白名单中的 Navicat 注册表项以重置其状态。
pub fn reset_navicat() -> Result<u32, BackendError> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hku = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Registration17XCS");
    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Update");

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup_root = format!(r"Software\RustWinTool\Backup\{stamp}\CLSID");

    let clsid_path = r"Software\Classes\CLSID";
    let clsid = hku
        .open_subkey_with_flags(clsid_path, KEY_READ)
        .map_err(|error| BackendError::Registry(error.to_string()))?;

    let mut deleted = 0;
    for key_name in clsid.enum_keys().filter_map(Result::ok) {
        let full_path = format!(r"{}\{}", clsid_path, key_name);
        if should_delete_key(&hku, &full_path) {
            // 先备份注册表项，再执行删除，以便撤销这项更改。
            let backup_path = format!(r"{}\{}", backup_root, key_name);
            let _ = copy_key_tree(&hku, &full_path, &backup_path);
            hku.delete_subkey_all(&full_path)
                .map_err(|error| BackendError::Registry(error.to_string()))?;
            deleted += 1;
        }
    }

    if deleted > 0 {
        crate::logging::log(&format!(
            "navicat cleanup removed {deleted} CLSID keys, backup at HKCU\\{backup_root}"
        ));
    }
    Ok(deleted)
}

/// 递归复制注册表项，以便清理操作可以撤销。
#[cfg(windows)]
fn copy_key_tree(
    root: &winreg::RegKey,
    source_path: &str,
    destination_path: &str,
) -> std::io::Result<()> {
    use winreg::enums::KEY_READ;

    let source = root.open_subkey_with_flags(source_path, KEY_READ)?;
    let (destination, _) = root.create_subkey(destination_path)?;
    for (name, data) in source.enum_values().flatten() {
        let _ = destination.set_raw_value(name, &data);
    }
    for name in source.enum_keys().flatten() {
        let _ = copy_key_tree(
            root,
            &format!(r"{source_path}\{name}"),
            &format!(r"{destination_path}\{name}"),
        );
    }
    Ok(())
}

#[cfg(windows)]
fn should_delete_key(hku: &winreg::RegKey, path: &str) -> bool {
    use winreg::enums::KEY_READ;

    if let Ok(subkey) = hku.open_subkey_with_flags(path, KEY_READ) {
        if subkey
            .enum_values()
            .filter_map(Result::ok)
            .any(|value| value.0.contains("Info") || value.0.contains("ShellFolder"))
        {
            return true;
        }
        return subkey
            .enum_keys()
            .filter_map(Result::ok)
            .any(|name| name.contains("Info") || name.contains("ShellFolder"));
    }
    false
}

#[cfg(not(windows))]
/// 返回错误，说明 Navicat 注册表重置仅支持 Windows。
pub fn reset_navicat() -> Result<u32, BackendError> {
    Err(BackendError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::{parse_system_info_json, BackendError};

    #[test]
    fn parses_numeric_system_info_fields() {
        let info = parse_system_info_json(
            r#"{"OS":"Windows 11","Version":"10.0","Build":"26100","Computer":"DESKTOP","User":"alice","CPU":"Test CPU","Cores":12,"RAM":31.5}"#,
        )
        .expect("valid system info JSON");

        assert_eq!(info.os, "Windows 11");
        assert_eq!(info.cores, "12");
        assert_eq!(info.ram, "31.5");
    }

    #[test]
    fn reports_invalid_system_info_shape() {
        let error = parse_system_info_json("[]").expect_err("array must be rejected");
        assert!(matches!(error, BackendError::Parse(_)));
    }
}
