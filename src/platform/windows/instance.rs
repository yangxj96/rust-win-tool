//! 使用命名互斥体保持单实例，并将重复启动引导到现有窗口。

use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

pub fn already_running() -> bool {
    let name: Vec<u16> = "Local\\rust-win-tool-single-instance\0"
        .encode_utf16()
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return false;
    }
    // 有意保留句柄，使互斥体在进程整个生命周期内持续有效；进程退出后系统会释放该名称。
    unsafe { GetLastError() == ERROR_ALREADY_EXISTS }
}

pub fn focus_existing() {
    let title: Vec<u16> = "Rust Win Tool\0".encode_utf16().collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if !window.is_null() {
        unsafe {
            ShowWindow(window, SW_RESTORE);
            SetForegroundWindow(window);
        }
    }
}
