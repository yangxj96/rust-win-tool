#![windows_subsystem = "windows"]

use gpui::{
    px, size, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

mod app;
mod backend;
mod cleanup;
mod i18n;
mod theme;
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

fn main() {
    #[cfg(target_os = "windows")]
    if elevation::relaunch_elevated() {
        return;
    }

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
            })
            .expect("failed to initialize service status refresh");
        cx.activate(true);
    });
}
