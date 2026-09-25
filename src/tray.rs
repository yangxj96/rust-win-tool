//! System tray icon.
//!
//! GPUI has no tray support, so this creates a hidden message window that owns
//! a `Shell_NotifyIcon` icon. Left clicking the icon shows the main window;
//! right clicking opens a small menu (show / hide / exit).
//!
//! Only the visibility of the main window is driven from here; "exit" goes
//! through the normal `WM_CLOSE` handling so GPUI can shut down cleanly.

/// Tray menu labels, resolved from the current language.
pub struct TrayLabels {
    pub show: String,
    pub hide: String,
    pub exit: String,
}

#[cfg(windows)]
mod platform {
    use super::TrayLabels;
    use std::mem::size_of;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::{Mutex, OnceLock};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, GetCursorPos,
        LoadIconW, RegisterClassW, SetForegroundWindow, ShowWindow, TrackPopupMenu,
        IDI_APPLICATION, MF_STRING, SW_HIDE, SW_RESTORE, SW_SHOW, TPM_BOTTOMALIGN, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, WM_CLOSE, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
    };

    const TRAY_CALLBACK: u32 = 0x8000 + 1; // WM_APP + 1
    const COMMAND_SHOW: usize = 1;
    const COMMAND_HIDE: usize = 2;
    const COMMAND_EXIT: usize = 3;

    static MAIN_WINDOW: AtomicIsize = AtomicIsize::new(0);
    static TRAY_WINDOW: AtomicIsize = AtomicIsize::new(0);
    static LABELS: OnceLock<Mutex<TrayLabels>> = OnceLock::new();

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[allow(clippy::manual_dangling_ptr)] // MAKEINTRESOURCE-style icon resource id
    pub fn install(labels: TrayLabels, main_window: HWND) {
        MAIN_WINDOW.store(main_window as isize, Ordering::SeqCst);
        let _ = LABELS.set(Mutex::new(labels));

        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide("RustWinToolTrayWindow");
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&class);
            let tray_window = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if tray_window.is_null() {
                return;
            }
            TRAY_WINDOW.store(tray_window as isize, Ordering::SeqCst);

            // Resource id 1 is the icon embedded by the build script; fall back
            // to the generic application icon if it cannot be loaded.
            let mut icon = LoadIconW(instance, 1 as *const u16);
            if icon.is_null() {
                icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
            }

            let mut data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: tray_window,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: TRAY_CALLBACK,
                hIcon: icon,
                ..NOTIFYICONDATAW::default()
            };
            let tip: Vec<u16> = "Rust Win Tool".encode_utf16().collect();
            let length = tip.len().min(127);
            data.szTip[..length].copy_from_slice(&tip[..length]);
            Shell_NotifyIconW(NIM_ADD, &data);
        }
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            TRAY_CALLBACK => {
                match lparam as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => show_main(),
                    WM_RBUTTONUP => show_menu(window),
                    _ => {}
                }
                0
            }
            WM_CLOSE => {
                remove_icon();
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    fn main_window() -> HWND {
        MAIN_WINDOW.load(Ordering::SeqCst) as HWND
    }

    fn show_main() {
        let window = main_window();
        if !window.is_null() {
            unsafe {
                ShowWindow(window, SW_SHOW);
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
        }
    }

    fn hide_main() {
        let window = main_window();
        if !window.is_null() {
            unsafe { ShowWindow(window, SW_HIDE) };
        }
    }

    fn exit_app() {
        let window = main_window();
        if !window.is_null() {
            unsafe { PostMessageW(window, WM_CLOSE, 0, 0) };
        }
    }

    fn show_menu(owner: HWND) {
        let labels = LABELS.get().and_then(|labels| labels.lock().ok());
        let Some(labels) = labels else {
            return;
        };
        let show = wide(&labels.show);
        let hide = wide(&labels.hide);
        let exit = wide(&labels.exit);
        drop(labels);

        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }
            AppendMenuW(menu, MF_STRING, COMMAND_SHOW, show.as_ptr());
            AppendMenuW(menu, MF_STRING, COMMAND_HIDE, hide.as_ptr());
            AppendMenuW(menu, MF_STRING, COMMAND_EXIT, exit.as_ptr());

            let mut point = POINT { x: 0, y: 0 };
            GetCursorPos(&mut point);
            // Required so the menu closes when clicking elsewhere.
            SetForegroundWindow(owner);
            let command = TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_BOTTOMALIGN,
                point.x,
                point.y,
                0,
                owner,
                std::ptr::null(),
            );
            DestroyMenu(menu);

            match command as usize {
                COMMAND_SHOW => show_main(),
                COMMAND_HIDE => hide_main(),
                COMMAND_EXIT => exit_app(),
                _ => {}
            }
        }
    }

    fn remove_icon() {
        let tray_window = TRAY_WINDOW.load(Ordering::SeqCst) as HWND;
        if tray_window.is_null() {
            return;
        }
        let data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: tray_window,
            uID: 1,
            ..NOTIFYICONDATAW::default()
        };
        unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
    }
}

#[cfg(windows)]
pub use platform::install;

#[cfg(not(windows))]
pub fn install(_labels: TrayLabels, _main_window: *mut std::ffi::c_void) {}
