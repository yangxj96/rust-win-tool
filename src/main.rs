#![windows_subsystem = "windows"]

use gpui::{
    px, size, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

mod app;
mod backend;
mod cleanup;
mod file_dialog;
mod i18n;
mod logging;
mod theme;
mod tray;
mod ui;

use app::AppState;
use ui::main_window::MainWindow;

/// Re-launch with administrator rights on Windows. gpui ships its own
/// application manifest, so elevation cannot be requested through a manifest
/// resource without conflicting with it; request it at startup instead.
#[cfg(target_os = "windows")]
mod elevation {
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn is_elevated() -> bool {
        let mut token: HANDLE = std::ptr::null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast::<c_void>(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )
        };
        unsafe { CloseHandle(token) };
        ok != 0 && elevation.TokenIsElevated != 0
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Returns `true` when this process should exit because an elevated copy
    /// was launched in its place.
    pub fn relaunch_elevated() -> bool {
        if is_elevated() {
            return false;
        }
        let Ok(executable) = std::env::current_exe() else {
            return false;
        };
        let file = wide(executable.as_os_str());
        let verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
        // A result of 32 or less means the request failed (for example the user
        // dismissed the UAC prompt); this process stops in either case so the
        // app never runs without administrator rights.
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
        }
        true
    }
}

/// Keep a single running instance; a second launch focuses the existing window.
#[cfg(target_os = "windows")]
mod instance {
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
        // The handle is intentionally leaked so the mutex lives for the whole
        // process; when this process exits the name is released.
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
}

fn main() {
    #[cfg(target_os = "windows")]
    if elevation::relaunch_elevated() {
        return;
    }
    #[cfg(target_os = "windows")]
    if instance::already_running() {
        instance::focus_existing();
        return;
    }

    logging::init();

    Application::new().run(|cx| {
        let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
        let state = cx.new(|_| AppState::new());
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(900.), px(600.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Rust Win Tool".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    is_movable: true,
                    is_resizable: true,
                    is_minimizable: true,
                    ..Default::default()
                },
                |_window, cx| cx.new(|cx| MainWindow::new(state.clone(), cx)),
            )
            .expect("failed to open the main window");
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.focus_handle);
                view.refresh_services(cx);

                #[cfg(target_os = "windows")]
                {
                    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    if let Ok(handle) = <gpui::Window as HasWindowHandle>::window_handle(window) {
                        if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                            let translations = view.state.read(cx).t();
                            tray::install(
                                tray::TrayLabels {
                                    show: translations.tray_show.to_string(),
                                    hide: translations.tray_hide.to_string(),
                                    exit: translations.tray_exit.to_string(),
                                },
                                win32.hwnd.get() as *mut std::ffi::c_void,
                            );
                        }
                    }
                }
            })
            .expect("failed to initialize the main window");
        cx.activate(true);
    });
}
