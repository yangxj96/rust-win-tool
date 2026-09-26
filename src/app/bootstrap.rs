//! GPUI 应用、主窗口和启动期服务的装配。

use gpui::{
    px, size, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

use crate::app::AppState;
use crate::support::logging;
use crate::ui::main_window::MainWindow;

/// 启动并装配主窗口。
pub(super) fn run() {
    #[cfg(target_os = "windows")]
    if crate::platform::windows::elevation::relaunch_elevated() {
        return;
    }
    #[cfg(target_os = "windows")]
    if crate::platform::windows::instance::already_running() {
        crate::platform::windows::instance::focus_existing();
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
                            crate::platform::windows::tray::install(
                                crate::platform::windows::tray::TrayLabels {
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
