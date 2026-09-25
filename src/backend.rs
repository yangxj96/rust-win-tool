use app_service::{ManagedService, ServiceError, ServiceInfo};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

pub fn list_services() -> Result<Vec<ServiceInfo>, BackendError> {
    app_service::list_all_services().map_err(service_error)
}

pub fn service_status(name: &str) -> Result<String, BackendError> {
    app_service::get_service_status(name).map_err(service_error)
}

pub fn start_service(name: &str) -> Result<(), BackendError> {
    app_service::start_service(name).map_err(service_error)
}

pub fn stop_service(name: &str) -> Result<(), BackendError> {
    app_service::stop_service(name).map_err(service_error)
}

pub fn service_details(name: &str) -> Result<app_service::ServiceDetails, BackendError> {
    app_service::get_service_details(name).map_err(service_error)
}

pub fn set_service_start_type(
    name: &str,
    start_type: app_service::StartType,
) -> Result<(), BackendError> {
    app_service::set_service_start_type(name, start_type).map_err(service_error)
}

pub fn export_managed_services(
    path: &Path,
    services: &[ManagedService],
) -> Result<(), BackendError> {
    let json = serde_json::to_string_pretty(services)
        .map_err(|error| BackendError::Parse(error.to_string()))?;
    std::fs::write(path, json).map_err(|error| BackendError::Command(error.to_string()))
}

pub fn import_managed_services(path: &Path) -> Result<Vec<ManagedService>, BackendError> {
    let text =
        std::fs::read_to_string(path).map_err(|error| BackendError::Command(error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| BackendError::Parse(error.to_string()))
}

#[derive(Debug, Clone)]
pub enum ServiceOperation {
    Start(String),
    Stop(String),
    StartAll(Vec<String>),
    StopAll(Vec<String>),
    RefreshAll(Vec<String>),
}

#[derive(Debug)]
pub struct ServiceOperationResult {
    pub name: String,
    pub status: Result<String, BackendError>,
}

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
pub fn reset_navicat() -> Result<u32, BackendError> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hku = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Registration17XCS");
    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Update");

    let clsid_path = r"Software\Classes\CLSID";
    let clsid = hku
        .open_subkey_with_flags(clsid_path, KEY_READ)
        .map_err(|error| BackendError::Registry(error.to_string()))?;

    let mut deleted = 0;
    for key_name in clsid.enum_keys().filter_map(Result::ok) {
        let full_path = format!(r"{}\{}", clsid_path, key_name);
        if should_delete_key(&hku, &full_path) {
            hku.delete_subkey_all(&full_path)
                .map_err(|error| BackendError::Registry(error.to_string()))?;
            deleted += 1;
        }
    }

    Ok(deleted)
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
