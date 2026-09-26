//! 通过 PowerShell 采集并解析系统信息。

use serde_json::Value;
use std::process::Command;

use crate::shared::BackendError;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
