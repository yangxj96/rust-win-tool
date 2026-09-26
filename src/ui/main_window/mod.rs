//! GPUI 主窗口实体和文本输入接入。
//!
//! 主窗口实体保存窗口级状态，并组合窗口外壳、各页面和浮层；按职责拆分的渲染逻辑
//! 与回调放在同级子模块中。

use std::ops::Range;

use gpui::{
    canvas, div, point, prelude::*, px, relative, rgba, Animation, AnimationExt, AnyElement,
    AppContext, Bounds, ClickEvent, Context, Div, DragMoveEvent, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, FontWeight, KeyDownEvent, Pixels, Point, Render, ScrollHandle,
    Timer, UTF16Selection, Window, WindowControlArea,
};
use std::time::{Duration, Instant};

use crate::app::{
    AppState, HistoryAction, HistoryEntry, OperationState, PendingAction, ProcessSort,
    ServiceFilter, ServiceSort, ServiceStatus, Theme, View,
};
use crate::features::cleanup::{CleanupCategory, DeleteMode};
use crate::features::scripts::ScriptKind;
use crate::features::services::operations::{self as service_backend, ServiceOperation};
use crate::features::services::StartType;
use crate::features::tools::startup::StartupLocation;
use crate::features::tools::{monitor, network, processes, startup, system_info};
use crate::ui::i18n::Language;
use crate::ui::theme::{StatusColors, ThemeColors};
use crate::ui::{components, input};

const SCRIPTS: [(&str, &str); 2] = [
    ("script_reset_navicat", "script_reset_navicat_desc"),
    ("script_flush_dns", "script_flush_dns_desc"),
];

const TOOLS: [(&str, &str); 5] = [
    ("tool_sysinfo", "tool_sysinfo_desc"),
    ("tool_processes", "tool_processes_desc"),
    ("tool_monitor", "tool_monitor_desc"),
    ("tool_network", "tool_network_desc"),
    ("tool_startup", "tool_startup_desc"),
];

const SERVICE_NAME_COLUMN_WIDTH: f32 = 180.;
const SERVICE_DISPLAY_COLUMN_WIDTH: f32 = 200.;
const SERVICE_STATUS_COLUMN_WIDTH: f32 = 96.;
const SERVICE_ACTIONS_COLUMN_WIDTH: f32 = 210.;

const ICON_SERVICE: &str = "▤";
const ICON_TOOLS: &str = "▦";
const ICON_CLEANUP: &str = "▧";
const ICON_SCRIPTS: &str = "⟳";
const ICON_SETTINGS: &str = "⚙︎";

pub struct MainWindow {
    pub(crate) state: Entity<AppState>,
    pub(crate) focus_handle: FocusHandle,
    search_focus: FocusHandle,
    service_search_focus: FocusHandle,
    net_host_focus: FocusHandle,
    port_lookup_focus: FocusHandle,
    script_name_focus: FocusHandle,
    script_content_focus: FocusHandle,
    service_scroll: ScrollHandle,
    dialog_scroll: ScrollHandle,
    sysinfo_scroll: ScrollHandle,
    process_scroll: ScrollHandle,
    startup_scroll: ScrollHandle,
    cleanup_scroll: ScrollHandle,
    script_scroll: ScrollHandle,
    port_scroll: ScrollHandle,
    /// 正在等待删除确认的托管服务。
    pending_delete: Option<String>,
    /// 清理操作的确认对话框状态。
    pending_clean: bool,
    pending_empty_bin: bool,
    /// 会话操作历史浮层是否打开。
    history_open: bool,
    /// 等待结束确认的进程，包含 PID 和进程名。
    pending_kill: Option<(u32, String)>,
    /// 是否已经启动监视器采样循环。
    monitor_started: bool,
    /// 是否已完成首次刷新，避免启动时的首次刷新显示完成提示。
    refresh_notice_ready: bool,
}

/// 拖动自定义滚动条滑块时保存的数据。
#[derive(Clone)]
struct ScrollbarDrag {
    handle: ScrollHandle,
}

/// 用于滚动条拖动交互的不可见预览视图。
struct ScrollbarDragPreview;

/// 当前由哪个可编辑文本框接收平台输入事件。
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextTarget {
    Dialog,
    ServiceSearch,
    NetHost,
    PortLookup,
    ScriptName,
    ScriptContent,
}

/// 网络诊断操作类型。
#[derive(Clone, Copy, PartialEq, Eq)]
enum NetCheck {
    Ping,
    Dns,
    Port,
}

/// 通用确认对话框所需的标题、按钮和回调参数。
struct ConfirmDialog {
    title: &'static str,
    message: String,
    ok_label: &'static str,
    ok_id: &'static str,
    cancel_id: &'static str,
    danger: bool,
    on_ok: fn(&mut MainWindow, &ClickEvent, &mut Window, &mut Context<MainWindow>),
    on_cancel: fn(&mut MainWindow, &ClickEvent, &mut Window, &mut Context<MainWindow>),
}

impl Render for ScrollbarDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

mod cleanup;
mod dialogs;
mod scripts;
mod service;
mod settings;
mod shared;
mod shell;
mod tools;

use self::shared::toggle_window_zoom;

impl MainWindow {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            focus_handle: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            service_search_focus: cx.focus_handle(),
            net_host_focus: cx.focus_handle(),
            port_lookup_focus: cx.focus_handle(),
            script_name_focus: cx.focus_handle(),
            script_content_focus: cx.focus_handle(),
            service_scroll: ScrollHandle::new(),
            dialog_scroll: ScrollHandle::new(),
            sysinfo_scroll: ScrollHandle::new(),
            process_scroll: ScrollHandle::new(),
            startup_scroll: ScrollHandle::new(),
            cleanup_scroll: ScrollHandle::new(),
            script_scroll: ScrollHandle::new(),
            port_scroll: ScrollHandle::new(),
            pending_delete: None,
            pending_clean: false,
            pending_empty_bin: false,
            history_open: false,
            pending_kill: None,
            monitor_started: false,
            refresh_notice_ready: false,
        }
    }

    fn text_target(&self, window: &Window) -> TextTarget {
        if self.service_search_focus.is_focused(window) {
            TextTarget::ServiceSearch
        } else if self.net_host_focus.is_focused(window) {
            TextTarget::NetHost
        } else if self.port_lookup_focus.is_focused(window) {
            TextTarget::PortLookup
        } else if self.script_name_focus.is_focused(window) {
            TextTarget::ScriptName
        } else if self.script_content_focus.is_focused(window) {
            TextTarget::ScriptContent
        } else {
            TextTarget::Dialog
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // 文本插入由平台输入处理器（WM_CHAR / 输入法）负责。WM_CHAR 会过滤退格键，
        // 因此这里根据当前焦点文本框处理退格。脚本正文还需要将回车作为换行，而不是
        // 激活按钮。
        let target = self.text_target(window);
        if event.keystroke.key == "enter" {
            match target {
                TextTarget::ScriptContent => {
                    self.update_state(cx, |state| state.script_dialog_content_newline());
                }
                TextTarget::PortLookup => {
                    self.run_port_lookup(cx);
                }
                _ => {}
            }
            return;
        }
        if input::text_editing_input(event) != Some(input::TextEditingInput::Backspace) {
            return;
        }
        match target {
            TextTarget::ServiceSearch => {
                self.update_state(cx, |state| state.service_search_backspace());
            }
            TextTarget::NetHost => {
                self.update_state(cx, |state| state.net_host_backspace());
            }
            TextTarget::PortLookup => {
                self.update_state(cx, |state| state.port_lookup_query_backspace());
            }
            TextTarget::ScriptName => {
                self.update_state(cx, |state| state.script_dialog_name_backspace());
            }
            TextTarget::ScriptContent => {
                self.update_state(cx, |state| state.script_dialog_content_backspace());
            }
            TextTarget::Dialog => {
                if self.state.read(cx).show_add_dialog() {
                    self.update_state(cx, |state| state.add_dialog_backspace());
                }
            }
        }
    }

    fn minimize_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        window.minimize_window();
    }

    fn maximize_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        toggle_window_zoom(window);
    }

    fn close_window_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // 窗口关闭后继续驻留托盘；用户可以从托盘菜单退出程序。
        #[cfg(target_os = "windows")]
        {
            let _ = &window;
            crate::platform::windows::tray::hide_main_window();
        }
        #[cfg(not(target_os = "windows"))]
        {
            window.remove_window();
        }
    }

    fn update_state(&self, cx: &mut Context<Self>, update: impl FnOnce(&mut AppState) + 'static) {
        self.state.update(cx, |state, cx| {
            update(state);
            cx.notify();
        });
    }

    fn select_view(
        &mut self,
        view: View,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.set_view(view));
    }

    fn select_tool(
        &mut self,
        index: usize,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.select_tool(index));
    }

    fn select_dialog_service(
        &mut self,
        index: usize,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, move |state| state.select_add_dialog(index));
    }

    fn set_language_zh(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_language(Language::Chinese));
    }

    fn set_language_en(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_language(Language::English));
    }

    fn set_theme_dark(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_theme(Theme::Dark));
    }

    fn set_theme_light(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.set_theme(Theme::Light));
    }

    fn begin_add_dialog(&mut self, cx: &mut Context<Self>) {
        // 使用新的滚动句柄，使自定义滚动条能针对当前对话框重新测量。
        self.dialog_scroll = ScrollHandle::new();
        self.update_state(cx, |state| state.begin_add_dialog());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let services = cx
                .background_spawn(async { service_backend::list_services() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_add_dialog_services(services);
                    cx.notify();
                })
                .ok();
            // 自定义滚动条要等列表完成布局后才能读取尺寸，因此还需要再绘制一帧才会显示。
            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();
        })
        .detach();
    }

    pub(crate) fn refresh_services(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).operation_state() == OperationState::Refreshing {
            return;
        }
        let action = self.state.update(cx, |state, cx| {
            let action = state.request_refresh();
            cx.notify();
            action
        });
        // 手动刷新显示完成提示，启动时的自动刷新不显示提示。
        let announce = self.refresh_notice_ready;
        self.refresh_notice_ready = true;
        self.spawn_service_operation(action, announce, cx);
    }

    fn start_service(&mut self, name: String, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() != View::Service {
            return;
        }
        let action = self.state.update(cx, move |state, cx| {
            let action = state.request_start_service(&name);
            cx.notify();
            action
        });
        if let Some(action) = action {
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn stop_service(&mut self, name: String, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() != View::Service {
            return;
        }
        let action = self.state.update(cx, move |state, cx| {
            let action = state.request_stop_service(&name);
            cx.notify();
            action
        });
        if let Some(action) = action {
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn start_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_start_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn stop_all(&mut self, cx: &mut Context<Self>) {
        if self.state.read(cx).current_view() == View::Service {
            let action = self.state.update(cx, |state, cx| {
                let action = state.request_stop_all();
                cx.notify();
                action
            });
            self.spawn_service_operation(action, false, cx);
        }
    }

    fn spawn_service_operation(
        &mut self,
        action: PendingAction,
        announce: bool,
        cx: &mut Context<Self>,
    ) {
        let is_refresh = matches!(action, PendingAction::RefreshAll(_));
        let history_action = match &action {
            PendingAction::StartService(_) => HistoryAction::Start,
            PendingAction::StopService(_) => HistoryAction::Stop,
            PendingAction::StartAll(_) => HistoryAction::StartAll,
            PendingAction::StopAll(_) => HistoryAction::StopAll,
            PendingAction::RefreshAll(_) => HistoryAction::Refresh,
        };
        let operation = match action {
            PendingAction::StartService(name) => ServiceOperation::Start(name),
            PendingAction::StopService(name) => ServiceOperation::Stop(name),
            PendingAction::StartAll(names) => ServiceOperation::StartAll(names),
            PendingAction::StopAll(names) => ServiceOperation::StopAll(names),
            PendingAction::RefreshAll(names) => ServiceOperation::RefreshAll(names),
        };
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let started = Instant::now();
            let results = cx
                .background_spawn(
                    async move { service_backend::execute_service_operation(operation) },
                )
                .await;

            // 原生后端可能在几毫秒内完成刷新；延长提示显示时间，确保用户能注意到反馈。
            if is_refresh {
                let minimum = Duration::from_millis(700);
                let elapsed = started.elapsed();
                if elapsed < minimum {
                    Timer::after(minimum - elapsed).await;
                }
            }

            let refreshed = results.len();
            let pending = state
                .update(cx, |state, cx| {
                    for result in results {
                        let ok = result.status.is_ok();
                        let message = result
                            .status
                            .as_ref()
                            .err()
                            .map(|error| error.to_string())
                            .unwrap_or_default();
                        state.record_history(HistoryEntry {
                            action: history_action,
                            service: result.name.clone(),
                            ok,
                            message: message.clone(),
                        });
                        crate::support::logging::log(&format!(
                            "{:?} {} -> {}",
                            history_action,
                            result.name,
                            if ok {
                                "ok".to_string()
                            } else {
                                message.clone()
                            }
                        ));
                        match result.status {
                            Ok(status) => state.apply_service_success(&result.name, &status),
                            Err(error) => state.apply_service_error(&result.name, &error),
                        }
                    }
                    if state.operation_state() != OperationState::Error {
                        state.set_operation_state(OperationState::Idle);
                    }
                    if is_refresh && announce {
                        let notice = state.t().refresh_done.replace("{}", &refreshed.to_string());
                        state.set_refresh_notice(notice);
                    }
                    let pending: Vec<String> = state
                        .managed_services()
                        .iter()
                        .map(|service| service.name.clone())
                        .filter(|name| {
                            matches!(
                                state.service_status(name),
                                ServiceStatus::Starting | ServiceStatus::Stopping
                            )
                        })
                        .collect();
                    cx.notify();
                    pending
                })
                .unwrap_or_default();

            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();

            if is_refresh && announce {
                Timer::after(Duration::from_millis(1100)).await;
                state
                    .update(cx, |state, cx| {
                        state.clear_refresh_notice();
                        cx.notify();
                    })
                    .ok();
            }

            if pending.is_empty() {
                return;
            }

            // 原生服务 API 在接受请求后立即返回，因此需要在后台轮询，并在服务状态稳定后
            // 更新界面。
            for _ in 0..75 {
                Timer::after(Duration::from_millis(400)).await;
                let names = pending.clone();
                let statuses = cx
                    .background_spawn(async move {
                        names
                            .into_iter()
                            .map(|name| {
                                let status = service_backend::service_status(&name);
                                (name, status)
                            })
                            .collect::<Vec<_>>()
                    })
                    .await;
                let settled = state
                    .update(cx, |state, cx| {
                        let mut settled = true;
                        for (name, status) in statuses {
                            match status {
                                Ok(status) => state.apply_service_success(&name, &status),
                                Err(error) => state.apply_service_error(&name, &error),
                            }
                            if matches!(
                                state.service_status(&name),
                                ServiceStatus::Starting | ServiceStatus::Stopping
                            ) {
                                settled = false;
                            }
                        }
                        cx.notify();
                        settled
                    })
                    .unwrap_or(true);
                if settled {
                    break;
                }
            }
        })
        .detach();
    }

    fn open_tool(&mut self, cx: &mut Context<Self>) {
        let opened = self.state.update(cx, |state, cx| {
            let opened = state.open_tool_detail();
            cx.notify();
            opened
        });
        if !opened {
            return;
        }
        let state = self.state.clone();
        if self.state.read(cx).system_info_loading() {
            cx.spawn(async move |_this, cx| {
                let result = cx
                    .background_spawn(async { system_info::fetch_system_info() })
                    .await;
                state
                    .update(cx, |state, cx| {
                        state.set_system_info(result);
                        cx.notify();
                    })
                    .ok();
                Timer::after(Duration::from_millis(100)).await;
                state.update(cx, |_state, cx| cx.notify()).ok();
            })
            .detach();
        } else if self.state.read(cx).processes_loading() {
            cx.spawn(async move |_this, cx| {
                let result = cx
                    .background_spawn(async { processes::list_processes() })
                    .await;
                state
                    .update(cx, |state, cx| {
                        state.set_processes(result);
                        cx.notify();
                    })
                    .ok();
            })
            .detach();
        } else if self.state.read(cx).monitor_active() {
            self.ensure_monitor_loop(cx);
        } else if self.state.read(cx).startup().loading {
            cx.spawn(async move |_this, cx| {
                let result = cx.background_spawn(async { startup::list_startup() }).await;
                state
                    .update(cx, |state, cx| {
                        state.set_startup(result);
                        cx.notify();
                    })
                    .ok();
            })
            .detach();
        }
    }

    fn ensure_monitor_loop(&mut self, cx: &mut Context<Self>) {
        if self.monitor_started {
            return;
        }
        self.monitor_started = true;
        let state = self.state.clone();
        cx.spawn(async move |this, cx| loop {
            let active = state
                .read_with(cx, |state, _| state.monitor_active())
                .unwrap_or(false);
            if !active {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, _cx| {
                        this.monitor_started = false;
                    })
                    .ok();
                }
                break;
            }
            let result = cx
                .background_spawn(async { monitor::sample_metrics() })
                .await;
            state
                .update(cx, |state, cx| {
                    if let Ok(metrics) = result {
                        state.push_metrics(metrics);
                    }
                    cx.notify();
                })
                .ok();
            Timer::after(Duration::from_millis(1000)).await;
        })
        .detach();
    }

    fn refresh_processes(&mut self, cx: &mut Context<Self>) {
        self.update_state(cx, |state| state.begin_process_refresh());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async { processes::list_processes() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_processes(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn run_script(&mut self, cx: &mut Context<Self>) {
        let (selected, translations) = {
            let state = self.state.read(cx);
            (state.scripts_selected(), state.t())
        };
        let custom = if selected >= 2 {
            self.state
                .read(cx)
                .custom_script(selected - 2)
                .map(|script| (script.kind, script.command.clone()))
        } else {
            None
        };
        self.update_state(cx, |state| state.begin_script());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    match selected {
                        0 => crate::features::cleanup::navicat::reset_navicat().map(|deleted| {
                            translations
                                .script_result_cleanup
                                .replace("{}", &deleted.to_string())
                        }),
                        1 => network::flush_dns(),
                        _ => match custom {
                            Some((kind, command)) => {
                                crate::features::scripts::runtime::run_script(kind, &command)
                            }
                            None => Ok(String::new()),
                        },
                    }
                })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_script_output(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let page = match state.current_view() {
            View::Service => self.render_service_page(window, state, cx),
            View::Tools => self.render_tools_page(window, state, cx),
            View::Cleanup => self.render_cleanup_page(state, cx),
            View::Scripts => self.render_scripts_page(state, cx),
            View::Settings => self.render_settings_page(state, cx),
        };
        let mut content = div()
            .flex()
            .flex_1()
            .w_full()
            .min_h(px(0.))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .child(self.render_sidebar(state, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .child(self.render_content_header(state))
                    .child(div().flex_1().w_full().min_h(px(0.)).child(page)),
            );
        if state.show_add_dialog() {
            content = content.child(self.render_add_dialog(window, state, cx));
        }
        if state.show_port_lookup() {
            content = content.child(self.render_port_lookup_dialog(window, state, cx));
        }
        if self.pending_delete.is_some() {
            content = content.child(self.render_confirm_delete(state, cx));
        }
        if self.pending_clean {
            content = content.child(self.render_confirm_clean(state, cx));
        }
        if self.pending_empty_bin {
            content = content.child(self.render_confirm_empty_bin(state, cx));
        }
        if state.service_detail_name().is_some() {
            content = content.child(self.render_service_detail(state, cx));
        }
        if self.history_open {
            content = content.child(self.render_history(state, cx));
        }
        if self.pending_kill.is_some() {
            content = content.child(self.render_confirm_kill(state, cx));
        }
        if state.show_script_dialog() {
            content = content.child(self.render_add_script_dialog(window, state, cx));
        }
        if state.script_output().is_some() {
            content = content.child(self.render_script_output(state, cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(components::color(state.theme_colors().bg.canvas))
            .text_color(components::color(state.theme_colors().fg.default))
            .child(self.render_titlebar(window, state, cx))
            .child(content)
    }
}

impl EntityInputHandler for MainWindow {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let target = self.text_target(window);
        let state = self.state.read(cx);
        let text = match target {
            TextTarget::ServiceSearch => state.service_search_text_range(range_utf16.clone()),
            TextTarget::NetHost => state.net_host_text_range(range_utf16.clone()),
            TextTarget::PortLookup => state.port_lookup_query_text_range(range_utf16.clone()),
            TextTarget::ScriptName => state.script_dialog_name_text_range(range_utf16.clone()),
            TextTarget::ScriptContent => {
                state.script_dialog_content_text_range(range_utf16.clone())
            }
            TextTarget::Dialog => state.add_dialog_text_range(range_utf16.clone()),
        };
        *adjusted_range = Some(range_utf16);
        Some(text)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let target = self.text_target(window);
        let state = self.state.read(cx);
        let cursor = match target {
            TextTarget::ServiceSearch => state.service_search_text_utf16_len(),
            TextTarget::NetHost => state.net_host_text_utf16_len(),
            TextTarget::PortLookup => state.port_lookup_query_utf16_len(),
            TextTarget::ScriptName => state.script_dialog_name_utf16_len(),
            TextTarget::ScriptContent => state.script_dialog_content_utf16_len(),
            TextTarget::Dialog => state.add_dialog_text_utf16_len(),
        };
        Some(UTF16Selection {
            range: cursor..cursor,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let target = self.text_target(window);
        let state = self.state.read(cx);
        match target {
            TextTarget::ServiceSearch => state.service_search_marked_range(),
            TextTarget::NetHost => state.net_host_marked_range(),
            TextTarget::PortLookup => state.port_lookup_query_marked_range(),
            TextTarget::ScriptName => state.script_dialog_name_marked_range(),
            TextTarget::ScriptContent => state.script_dialog_content_marked_range(),
            TextTarget::Dialog => state.add_dialog_marked_range(),
        }
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = self.text_target(window);
        self.update_state(cx, move |state| match target {
            TextTarget::ServiceSearch => state.service_search_unmark(),
            TextTarget::NetHost => state.net_host_unmark(),
            TextTarget::PortLookup => state.port_lookup_query_unmark(),
            TextTarget::ScriptName => state.script_dialog_name_unmark(),
            TextTarget::ScriptContent => state.script_dialog_content_unmark(),
            TextTarget::Dialog => state.add_dialog_unmark(),
        });
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.to_string();
        let target = self.text_target(window);
        self.update_state(cx, move |state| match target {
            TextTarget::ServiceSearch => match replacement_range {
                Some(range) => state.service_search_replace_range(range, &text),
                None => state.service_search_commit_text(&text),
            },
            TextTarget::NetHost => match replacement_range {
                Some(range) => state.net_host_replace_range(range, &text),
                None => state.net_host_commit_text(&text),
            },
            TextTarget::PortLookup => match replacement_range {
                Some(range) => state.port_lookup_query_replace_range(range, &text),
                None => state.port_lookup_query_commit_text(&text),
            },
            TextTarget::ScriptName => match replacement_range {
                Some(range) => state.script_dialog_name_replace_range(range, &text),
                None => state.script_dialog_name_commit_text(&text),
            },
            TextTarget::ScriptContent => match replacement_range {
                Some(range) => state.script_dialog_content_replace_range(range, &text),
                None => state.script_dialog_content_commit_text(&text),
            },
            TextTarget::Dialog => match replacement_range {
                Some(range) => state.add_dialog_replace_range(range, &text),
                None => state.add_dialog_commit_text(&text),
            },
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = new_text.to_string();
        let target = self.text_target(window);
        self.update_state(cx, move |state| match target {
            TextTarget::ServiceSearch => {
                if let Some(range) = range_utf16 {
                    state.service_search_replace_range(range, "");
                }
                state.service_search_set_marked(&text);
            }
            TextTarget::NetHost => {
                if let Some(range) = range_utf16 {
                    state.net_host_replace_range(range, "");
                }
                state.net_host_set_marked(&text);
            }
            TextTarget::PortLookup => {
                if let Some(range) = range_utf16 {
                    state.port_lookup_query_replace_range(range, "");
                }
                state.port_lookup_query_set_marked(&text);
            }
            TextTarget::ScriptName => {
                if let Some(range) = range_utf16 {
                    state.script_dialog_name_replace_range(range, "");
                }
                state.script_dialog_name_set_marked(&text);
            }
            TextTarget::ScriptContent => {
                if let Some(range) = range_utf16 {
                    state.script_dialog_content_replace_range(range, "");
                }
                state.script_dialog_content_set_marked(&text);
            }
            TextTarget::Dialog => {
                if let Some(range) = range_utf16 {
                    state.add_dialog_replace_range(range, "");
                }
                state.add_dialog_set_marked(&text);
            }
        });
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests;
