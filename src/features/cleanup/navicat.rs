//! Navicat 注册表清理及可撤销备份。

use crate::shared::BackendError;

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
            let backup_path = format!(r"{}\{}", backup_root, key_name);
            let _ = copy_key_tree(&hku, &full_path, &backup_path);
            hku.delete_subkey_all(&full_path)
                .map_err(|error| BackendError::Registry(error.to_string()))?;
            deleted += 1;
        }
    }

    if deleted > 0 {
        crate::support::logging::log(&format!(
            "navicat cleanup removed {deleted} CLSID keys, backup at HKCU\\{backup_root}"
        ));
    }
    Ok(deleted)
}

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
