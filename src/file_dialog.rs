//! Windows 原生打开和保存文件对话框。

use std::path::PathBuf;

/// 打开文件选择对话框，请用户选择一个已存在的文件。
pub fn open_file() -> Option<PathBuf> {
    platform::dialog(false, "")
}

/// 打开文件保存对话框，并将 `default_name` 预填为目标文件名。
pub fn save_file(default_name: &str) -> Option<PathBuf> {
    platform::dialog(true, default_name)
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;

    pub fn dialog(save: bool, default_name: &str) -> Option<PathBuf> {
        use windows_sys::Win32::UI::Controls::Dialogs::{
            GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
            OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        };

        let filter: Vec<u16> = "JSON (*.json)\0*.json\0All files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let default_extension: Vec<u16> = "json\0".encode_utf16().collect();

        let mut buffer = vec![0u16; 4096];
        if !default_name.is_empty() {
            let name: Vec<u16> = default_name.encode_utf16().collect();
            let length = name.len().min(buffer.len() - 1);
            buffer[..length].copy_from_slice(&name[..length]);
        }

        let mut flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST;
        flags |= if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };

        let mut options = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            lpstrFilter: filter.as_ptr(),
            lpstrFile: buffer.as_mut_ptr(),
            nMaxFile: buffer.len() as u32,
            lpstrDefExt: default_extension.as_ptr(),
            Flags: flags,
            ..OPENFILENAMEW::default()
        };

        let accepted = unsafe {
            if save {
                GetSaveFileNameW(&mut options)
            } else {
                GetOpenFileNameW(&mut options)
            }
        };
        if accepted == 0 {
            return None;
        }

        let end = buffer
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(buffer.len());
        Some(PathBuf::from(String::from_utf16_lossy(&buffer[..end])))
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    pub fn dialog(_save: bool, _default_name: &str) -> Option<PathBuf> {
        None
    }
}
