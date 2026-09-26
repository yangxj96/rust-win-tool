//! 注册表 Run 项和启动文件夹中的开机启动项。
//!
//! 禁用操作不会删除任何内容：注册表值会移动到配套的 `RunDisabled` 项，文件夹中的
//! 启动项会移动到 `disabled` 子目录，因此两类项目都可以恢复。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupLocation {
    RegistryHkcu,
    RegistryHklm,
    FolderUser,
    FolderCommon,
}

#[derive(Debug, Clone)]
pub struct StartupItem {
    pub location: StartupLocation,
    /// 注册表值名称；对于文件夹启动项，此字段保存文件名。
    pub value_name: String,
    pub name: String,
    pub command: String,
    pub enabled: bool,
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_DISABLED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\RunDisabled";

#[cfg(windows)]
pub fn list() -> Vec<StartupItem> {
    let mut items = Vec::new();
    items.extend(registry_items(
        StartupLocation::RegistryHkcu,
        winreg::enums::HKEY_CURRENT_USER,
    ));
    items.extend(registry_items(
        StartupLocation::RegistryHklm,
        winreg::enums::HKEY_LOCAL_MACHINE,
    ));
    items.extend(folder_items(
        StartupLocation::FolderUser,
        user_startup_dir(),
    ));
    items.extend(folder_items(
        StartupLocation::FolderCommon,
        common_startup_dir(),
    ));
    items
}

#[cfg(not(windows))]
pub fn list() -> Vec<StartupItem> {
    Vec::new()
}

#[cfg(windows)]
fn registry_items(location: StartupLocation, hive: winreg::HKEY) -> Vec<StartupItem> {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    let root = RegKey::predef(hive);
    let mut items = Vec::new();
    for (subkey, enabled) in [(RUN_KEY, true), (RUN_DISABLED_KEY, false)] {
        let Ok(key) = root.open_subkey_with_flags(subkey, KEY_READ) else {
            continue;
        };
        for entry in key.enum_values().flatten() {
            let (value_name, data) = entry;
            let command = data.to_string();
            items.push(StartupItem {
                location,
                name: value_name.clone(),
                value_name,
                command,
                enabled,
            });
        }
    }
    items
}

#[cfg(not(windows))]
fn registry_items(_location: StartupLocation, _hive: usize) -> Vec<StartupItem> {
    Vec::new()
}

fn folder_items(location: StartupLocation, dir: std::path::PathBuf) -> Vec<StartupItem> {
    let mut items = Vec::new();
    items.extend(read_folder(location, &dir, true));
    items.extend(read_folder(location, &dir.join("disabled"), false));
    items
}

fn read_folder(
    location: StartupLocation,
    dir: &std::path::Path,
    enabled: bool,
) -> Vec<StartupItem> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy() != "desktop.ini")
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            StartupItem {
                location,
                command: entry.path().display().to_string(),
                name: name.clone(),
                value_name: name,
                enabled,
            }
        })
        .collect()
}

#[cfg(windows)]
pub fn set_enabled(
    location: StartupLocation,
    value_name: &str,
    enabled: bool,
) -> Result<(), String> {
    match location {
        StartupLocation::RegistryHkcu => {
            set_registry(winreg::enums::HKEY_CURRENT_USER, value_name, enabled)
        }
        StartupLocation::RegistryHklm => {
            set_registry(winreg::enums::HKEY_LOCAL_MACHINE, value_name, enabled)
        }
        StartupLocation::FolderUser => set_folder(user_startup_dir(), value_name, enabled),
        StartupLocation::FolderCommon => set_folder(common_startup_dir(), value_name, enabled),
    }
}

#[cfg(not(windows))]
pub fn set_enabled(
    _location: StartupLocation,
    _value_name: &str,
    _enabled: bool,
) -> Result<(), String> {
    Err("unsupported platform".to_string())
}

#[cfg(windows)]
fn set_registry(hive: winreg::HKEY, value_name: &str, enabled: bool) -> Result<(), String> {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    let root = RegKey::predef(hive);
    if enabled {
        let disabled = root
            .open_subkey_with_flags(RUN_DISABLED_KEY, KEY_READ)
            .map_err(|error| error.to_string())?;
        let raw = disabled
            .get_raw_value(value_name)
            .map_err(|error| error.to_string())?;
        let (run, _) = root
            .create_subkey(RUN_KEY)
            .map_err(|error| error.to_string())?;
        run.set_raw_value(value_name, &raw)
            .map_err(|error| error.to_string())?;
        let (disabled, _) = root
            .create_subkey(RUN_DISABLED_KEY)
            .map_err(|error| error.to_string())?;
        let _ = disabled.delete_value(value_name);
    } else {
        let run = root
            .open_subkey_with_flags(RUN_KEY, KEY_READ)
            .map_err(|error| error.to_string())?;
        let raw = run
            .get_raw_value(value_name)
            .map_err(|error| error.to_string())?;
        let (disabled, _) = root
            .create_subkey(RUN_DISABLED_KEY)
            .map_err(|error| error.to_string())?;
        disabled
            .set_raw_value(value_name, &raw)
            .map_err(|error| error.to_string())?;
        let (run, _) = root
            .create_subkey(RUN_KEY)
            .map_err(|error| error.to_string())?;
        let _ = run.delete_value(value_name);
    }
    Ok(())
}

fn set_folder(dir: std::path::PathBuf, value_name: &str, enabled: bool) -> Result<(), String> {
    let disabled_dir = dir.join("disabled");
    let (from, to) = if enabled {
        (disabled_dir.join(value_name), dir.join(value_name))
    } else {
        (dir.join(value_name), disabled_dir.join(value_name))
    };
    if !to.parent().map(|parent| parent.exists()).unwrap_or(false) {
        std::fs::create_dir_all(to.parent().unwrap()).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&from, &to).map_err(|error| error.to_string())
}

fn user_startup_dir() -> std::path::PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    base.join(r"Microsoft\Windows\Start Menu\Programs\Startup")
}

fn common_startup_dir() -> std::path::PathBuf {
    let base = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    base.join(r"Microsoft\Windows\Start Menu\Programs\Startup")
}

/// 从当前用户和计算机范围内的受支持位置枚举启动项。
pub fn list_startup() -> Result<Vec<StartupItem>, crate::shared::BackendError> {
    Ok(list())
}

/// 在原有注册表或文件位置范围内启用或禁用一个受支持的启动项。
pub fn set_startup_enabled(
    location: StartupLocation,
    value_name: &str,
    enabled: bool,
) -> Result<(), crate::shared::BackendError> {
    set_enabled(location, value_name, enabled).map_err(crate::shared::BackendError::Command)
}
