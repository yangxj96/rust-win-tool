//! 主窗口组合结构和界面选择器的回归测试。

use super::{MainWindow, SERVICE_STATUS_COLUMN_WIDTH, TOOLS};
use crate::app::{AppState, View};
use gpui::{point, px, size, AppContext, Modifiers, MouseButton, TestAppContext};

#[gpui::test]
fn main_window_entity_smoke(cx: &mut TestAppContext) {
    let state = cx.new(|_| AppState::new());
    let window = cx.new(|cx| MainWindow::new(state.clone(), cx));
    let state_id = cx.read(|app| window.read(app).state.entity_id());
    assert_eq!(state_id, state.entity_id());
}

#[gpui::test]
fn main_window_render_smoke(cx: &mut TestAppContext) {
    let state = cx.new(|_| AppState::new());
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
        view.clone()
    });
}

#[gpui::test]
fn main_window_renders_minimum_shell_size(cx: &mut TestAppContext) {
    let state = cx.new(|_| AppState::new());
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(900.), px(600.)), |_, _| {
        view.clone()
    });
    assert!(visual_cx.debug_bounds("service-add").is_some());
}

#[gpui::test]
fn cleanup_page_renders(cx: &mut TestAppContext) {
    let state = cx.new(|_| {
        let mut state = AppState::new();
        state.set_view(View::Cleanup);
        state
    });
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
        view.clone()
    });
    assert!(visual_cx.debug_bounds("cleanup-scan").is_some());
}

#[gpui::test]
fn settings_page_renders(cx: &mut TestAppContext) {
    let state = cx.new(|_| {
        let mut state = AppState::new();
        state.set_view(View::Settings);
        state
    });
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
        view.clone()
    });
    assert!(visual_cx.debug_bounds("setting-open-logs").is_some());
}

#[gpui::test]
fn service_detail_renders(cx: &mut TestAppContext) {
    let state = cx.new(|_| {
        let mut state = AppState::new();
        state.begin_service_detail("Demo");
        state.set_service_detail(Ok(crate::features::services::ServiceDetails {
            name: "Demo".into(),
            display_name: "Demo Service".into(),
            description: "A demo service".into(),
            status: "Running".into(),
            start_type: crate::features::services::StartType::Automatic,
            binary_path: r"C:\demo.exe".into(),
            account: "LocalSystem".into(),
            process_id: 1234,
            depends_on: vec!["Dep".into()],
            dependents: Vec::new(),
        }));
        state
    });
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
        view.clone()
    });
    assert!(visual_cx.debug_bounds("detail-close").is_some());
}

#[gpui::test]
fn titlebar_mouse_down_does_not_prevent_native_window_drag(cx: &mut TestAppContext) {
    let state = cx.new(|_| AppState::new());
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(1100.), px(720.)), |_, _| {
        view.clone()
    });

    visual_cx.simulate_mouse_move(point(px(500.), px(24.)), None, Modifiers::default());
    visual_cx.simulate_mouse_down(
        point(px(500.), px(24.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    let default_prevented = visual_cx.update(|window, _| window.default_prevented());

    assert!(!default_prevented);
}

#[test]
fn service_status_column_is_compact() {
    assert_eq!(SERVICE_STATUS_COLUMN_WIDTH, 96.);
}

#[test]
fn tools_catalog_lists_all_tools() {
    assert_eq!(TOOLS.len(), 5);
    assert_eq!(TOOLS[0], ("tool_sysinfo", "tool_sysinfo_desc"));
    assert_eq!(TOOLS[1], ("tool_processes", "tool_processes_desc"));
    assert_eq!(TOOLS[2], ("tool_monitor", "tool_monitor_desc"));
    assert_eq!(TOOLS[3], ("tool_network", "tool_network_desc"));
    assert_eq!(TOOLS[4], ("tool_startup", "tool_startup_desc"));
}
