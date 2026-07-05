use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub start_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedService {
    pub name: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("命令失败: {0}")]
    CommandFailed(String),
    #[error("服务不存在")]
    NotFound(String),
    #[error("启动失败: {0}")]
    StartFailed(String),
    #[error("停止失败: {0}")]
    StopFailed(String),
    #[error("解析失败: {0}")]
    ParseFailed(String),
}

fn hidden_cmd() -> Command {
    let mut cmd = Command::new("powershell");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

fn ps_args(script: &str) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-Command".into(),
        format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}",
            script
        ),
    ]
}

pub fn list_all_services() -> Result<Vec<ServiceInfo>, ServiceError> {
    let output = hidden_cmd()
        .args(ps_args(
            "Get-Service | Select-Object Name, DisplayName, Status, StartType | ConvertTo-Json -Compress",
        ))
        .output()
        .map_err(|e| ServiceError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(ServiceError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| ServiceError::ParseFailed(e.to_string()))?;

    let mut services = Vec::new();
    let items = match &raw {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(_) => vec![raw.clone()],
        _ => return Err(ServiceError::ParseFailed("unexpected json shape".into())),
    };

    for item in &items {
        let name = item["Name"].as_str().unwrap_or_default().to_string();
        let display_name = item["DisplayName"].as_str().unwrap_or_default().to_string();
        let status = item["Status"].as_str().unwrap_or_default().to_string();
        let start_type = item["StartType"].as_str().unwrap_or_default().to_string();
        services.push(ServiceInfo {
            name,
            display_name,
            status,
            start_type,
            description: String::new(),
        });
    }

    services.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(services)
}

pub fn get_service_status(name: &str) -> Result<String, ServiceError> {
    let output = hidden_cmd()
        .args(ps_args(
            &format!("(Get-Service -Name '{}').Status", name.replace('\'', "''")),
        ))
        .output()
        .map_err(|e| ServiceError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        return Err(ServiceError::NotFound(name.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn start_service(name: &str) -> Result<(), ServiceError> {
    let output = hidden_cmd()
        .args(ps_args(
            &format!("Start-Service -Name '{}'", name.replace('\'', "''")),
        ))
        .output()
        .map_err(|e| ServiceError::StartFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ServiceError::StartFailed(
            if stderr.is_empty() { String::from_utf8_lossy(&output.stdout).to_string() } else { stderr.to_string() }
        ));
    }
    Ok(())
}

pub fn stop_service(name: &str) -> Result<(), ServiceError> {
    let output = hidden_cmd()
        .args(ps_args(
            &format!("Stop-Service -Name '{}' -Force", name.replace('\'', "''")),
        ))
        .output()
        .map_err(|e| ServiceError::StopFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ServiceError::StopFailed(
            if stderr.is_empty() { String::from_utf8_lossy(&output.stdout).to_string() } else { stderr.to_string() }
        ));
    }
    Ok(())
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
