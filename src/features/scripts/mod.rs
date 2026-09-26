//! 与其他配置一同持久化保存的用户自定义脚本。

pub mod runtime;

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 运行自定义脚本时使用的解释器。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScriptKind {
    #[default]
    Cmd,
    PowerShell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomScript {
    pub name: String,
    #[serde(default)]
    pub kind: ScriptKind,
    /// 脚本正文，可以包含多行内容。
    pub command: String,
}

pub fn load(path: &Path) -> Vec<CustomScript> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, scripts: &[CustomScript]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(scripts) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_legacy_scripts_without_kind() {
        let json = r#"[{"name":"legacy","command":"echo hi"}]"#;
        let scripts: Vec<CustomScript> = serde_json::from_str(json).unwrap();
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].kind, ScriptKind::Cmd);
        assert_eq!(scripts[0].command, "echo hi");
    }

    #[test]
    fn round_trips_script_kind() {
        let script = CustomScript {
            name: "ps".into(),
            kind: ScriptKind::PowerShell,
            command: "Write-Output 'hi'".into(),
        };
        let json = serde_json::to_string(&script).unwrap();
        assert!(json.contains("\"kind\":\"powershell\""));
        let back: CustomScript = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, ScriptKind::PowerShell);
    }
}
