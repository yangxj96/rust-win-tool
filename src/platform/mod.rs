//! 应用级平台接入和原生文件对话框。

pub mod file_dialog;

#[cfg(target_os = "windows")]
pub mod windows;
