#![windows_subsystem = "windows"]

use gpui::{
    px, size, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};

mod app;
mod backend;
mod i18n;
mod theme;
mod ui;

use app::AppState;
use ui::main_window::MainWindow;

fn main() {
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
