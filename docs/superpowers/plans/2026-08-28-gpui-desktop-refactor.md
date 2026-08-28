# GPUI Desktop Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有 ratatui/crossterm Windows 控制台应用直接重构为 GPUI 桌面应用，并保留现有已实现功能和 JSON 配置兼容性；页面级全局快捷键按后续需求移除，搜索框保留文字编辑能力。

**Architecture:** `MainWindow` 是唯一的 GPUI 根 Entity，直接持有重命名后的 `AppState`。UI 组件只负责渲染和发出事件，`backend` 集中封装 PowerShell、Windows 服务、系统信息解析、文件系统和注册表操作；所有外部系统操作由 GPUI 后台任务执行，再回到根 Entity 更新状态。

**Tech Stack:** Rust 2021, GPUI 0.2.2, serde/serde_json, winreg, local `app-service` crate, winresource, Windows Win32/DirectWrite backend。

**Spec:** `docs/superpowers/specs/2026-08-28-gpui-desktop-refactor-design.md`

## Global Constraints

- 使用 `gpui = "=0.2.2"`，不引入非官方 GPUI fork。
- 使用 GPUI `Application::new()`，第一阶段不增加 `gpui_platform` companion dependency。
- 直接移除 ratatui、crossterm 和 unicode-width，不保留 TUI fallback 或双前端 feature。
- 保留服务管理、系统信息、Navicat 清理、语言切换、主题切换和现有 JSON 配置格式。
- 不实现当前尚未实现的磁盘清理、网络诊断、进程管理、注册表编辑器和事件查看器。
- PowerShell、Windows 服务、系统信息和注册表操作不得在 GPUI Render 函数或前台事件回调中同步执行。
- 使用具体文件进行 Git 暂存；未经用户明确确认不执行 commit 或 push。
- 每个任务先增加/更新测试，再实现最小代码并运行该任务的验证命令。

---

### Task 1: Replace the console bootstrap with a minimal GPUI Windows application

**Files:**
- Modify: `Cargo.toml`
- Modify: `build.rs`
- Modify: `src/main.rs`
- Test: `cargo check`

**Interfaces:**
- Produces a compiling `main()` that starts `gpui::Application`, creates a window, and renders a temporary root view.
- Leaves `app.rs`, `i18n.rs`, `theme.rs`, and the existing UI modules disconnected only for this bootstrap task; they are reconnected by later tasks.

- [ ] **Step 1: Update the dependency manifest**

Change the root dependency section to remove the TUI-only crates and add the exact GPUI version while retaining the business dependencies:

```toml
[dependencies]
gpui = "=0.2.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
app-service = { path = "lib/service" }
winreg = "0.55"
```

Keep `winresource = "0.1"` under `[build-dependencies]`. Do not add `gpui_platform` unless `cargo check` proves that GPUI 0.2.2 cannot create a Windows application through `Application::new()`.

- [ ] **Step 2: Change the Windows subsystem**

In `build.rs`, keep the existing icon, product name and file description configuration, but change:

```rust
res.set("Subsystem", "windows");
```

This makes the produced executable a GUI application instead of a console application.

- [ ] **Step 3: Replace the ratatui event loop with a GPUI smoke root**

Replace the ratatui imports and `run_app` loop in `src/main.rs` with a minimal GPUI root that has the same startup shape as:

```rust
use gpui::{div, prelude::*, px, size, Application, Bounds, Context, Render, Window,
    WindowBounds, WindowOptions};

struct BootstrapView;

impl Render for BootstrapView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Rust Win Tool")
    }
}

fn main() {
    Application::new().run(|cx| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| BootstrapView),
        )
        .expect("failed to open main window");
        cx.activate(true);
    });
}
```

Use the exact imports exposed by GPUI 0.2.2 if the compiler reports an import path difference; do not restore ratatui or crossterm to solve the error.

- [ ] **Step 4: Run the bootstrap check**

Run: `cargo check`

Expected: the crate resolves GPUI 0.2.2 and compiles a Windows GUI bootstrap. If Cargo needs to populate the Rust registry cache, request the required command permission and rerun the same command.

### Task 2: Extract backend operations and make system-info parsing typed

**Files:**
- Create: `src/backend.rs`
- Modify: `src/app.rs`
- Modify: `src/ui/scripts.rs`
- Test: `src/backend.rs`

**Interfaces:**
- Produces `pub(crate) fn parse_system_info(json: &str) -> Result<SystemInfo, BackendError>`.
- Produces `pub(crate) fn fetch_system_info() -> Result<SystemInfo, BackendError>`.
- Produces `pub(crate) fn reset_navicat(language: Language) -> Result<String, String>`.
- Keeps `app-service` as the source for `list_all_services`, `get_service_status`, `start_service`, and `stop_service`.

- [ ] **Step 1: Add failing parser tests**

Add tests in `src/backend.rs` for numeric JSON fields:

```rust
#[test]
fn parses_numeric_system_info_fields() {
    let json = r#"{
        "OS":"Windows 11","Version":"10.0","Build":"26100",
        "Computer":"DESKTOP","User":"tester","CPU":"Test CPU",
        "Cores":16,"RAM":31.5
    }"#;

    let info = parse_system_info(json).expect("valid system info");
    assert_eq!(info.cores, "16");
    assert_eq!(info.ram, "31.5");
}

#[test]
fn missing_system_info_fields_use_na() {
    let info = parse_system_info(r#"{"OS":"Windows"}"#).expect("valid partial info");
    assert_eq!(info.os, "Windows");
    assert_eq!(info.cores, "N/A");
    assert_eq!(info.ram, "N/A");
}
```

- [ ] **Step 2: Run the parser tests to verify they fail**

Run: `cargo test backend::tests::parses_numeric_system_info_fields`

Expected: FAIL because `backend.rs` and `parse_system_info` do not exist yet.

- [ ] **Step 3: Move `SystemInfo` and implement typed parsing**

Keep `SystemInfo` as a public crate-local data type, either in `app.rs` or a dedicated data section imported by `backend.rs`. Implement parsing with numeric fallbacks:

```rust
fn json_text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or("N/A").to_string()
}

fn json_integer(value: &serde_json::Value) -> String {
    value.as_u64()
        .map(|number| number.to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

fn json_number(value: &serde_json::Value) -> String {
    value.as_f64()
        .map(|number| number.to_string())
        .unwrap_or_else(|| "N/A".to_string())
}
```

Define the parser error used by the public backend functions before implementing the parser:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    Command(String),
    Parse(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(message) => write!(f, "command failed: {message}"),
            Self::Parse(message) => write!(f, "parse failed: {message}"),
        }
    }
}

impl std::error::Error for BackendError {}
```

`parse_system_info` must parse all eight existing fields and use `json_integer` for `Cores` and `json_number` for `RAM`. A syntactically valid object with missing fields returns an `N/A` value; invalid JSON returns `BackendError`.

- [ ] **Step 4: Move external operation code out of UI modules**

Move the PowerShell body from `App::fetch_system_info` to `backend::fetch_system_info`. Move `reset_navicat` and `should_delete_key` from `src/ui/scripts.rs` to `backend.rs`. Keep localized messages by passing `Language` into the registry operation.

The UI script module must retain only script metadata and rendering code; it must call the backend through a callback supplied by `MainWindow`.

- [ ] **Step 5: Run backend tests**

Run: `cargo test backend::tests`

Expected: PASS, including numeric cores/RAM parsing and missing-field fallback.

### Task 3: Convert the state model from `App` to `AppState`

**Files:**
- Modify: `src/app.rs`
- Modify: `src/i18n.rs`
- Modify: `src/theme.rs`
- Test: `src/app.rs`

**Interfaces:**
- `AppState::new() -> Self` loads the existing configuration and managed services.
- `AppState::current_view() -> View` returns the current view by value.
- `AppState::select_service()`, `select_tool()`, `select_setting()` and `select_add_dialog()` apply mouse selections.
- `AppState::cycle_language()` and `AppState::cycle_theme()` update and persist settings.
- `AppState::service_status(name: &str) -> ServiceStatus` returns the typed current status for a service.
- `AppState::apply_service_status(name: &str, status: ServiceStatus)` updates one service without running external commands.
- `AppState::apply_system_info(result: Result<SystemInfo, String>)` updates system-info loading/error state.

- [ ] **Step 1: Add state-transition tests**

Extend `src/app.rs` tests with pure transition coverage:

```rust
#[test]
fn service_filter_matches_name_and_display_name() {
    let mut state = AppState::for_test(Vec::new());
    state.set_add_dialog_services(vec![
        ServiceInfo { name: "Spooler".into(), display_name: "Print Service".into(), status: "Stopped".into(), start_type: "Manual".into(), description: String::new() },
    ]);
    state.set_add_dialog_search("print");
    assert_eq!(state.add_dialog_filtered(), &[0]);
}
```

Add a test-only constructor that accepts services and a temporary settings path without calling Windows service APIs.

- [ ] **Step 2: Run the new state tests to verify they fail**

Run: `cargo test app::tests::service_selection_wraps_after_state_migration`

Expected: FAIL because `AppState`, `for_test`, and the pure state methods do not exist yet.

- [ ] **Step 3: Rename and simplify the state type**

Rename `App` to `AppState` throughout the crate. Keep `View`, `Theme`, `AddDialogState`, `SystemInfo`, language state, theme state, selected indices and configuration paths, but remove ratatui-specific APIs and the `PendingAction` render-loop protocol.

Use explicit state types for external operations:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationState {
    Idle,
    Running,
    Succeeded,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Refreshing,
    Unknown,
}
```

Store operation state separately from localized display text. Keep `translations_for`, `map_error`, JSON settings serialization, and existing config filenames.

- [ ] **Step 4: Remove ratatui types from theme and state code**

Change `ThemeColors` to store framework-independent `u32` RGB values. Keep the conversion helper in `src/ui/components.rs`, so `theme.rs` does not depend on GPUI:

```rust
fn theme_color(value: u32) -> gpui::Hsla {
    gpui::rgb(value).into()
}
```

Keep the existing dark/light semantic colors and translate all UI text through `AppState::t()`.

- [ ] **Step 5: Run all state tests**

Run: `cargo test app::tests`

Expected: PASS, including the existing nine tests migrated to `AppState` and the new filter/selection tests.

### Task 4: Build the GPUI root window and shared components

**Files:**
- Modify: `src/main.rs`
- Modify: `src/ui/mod.rs`
- Create: `src/ui/main_window.rs`
- Create: `src/ui/components.rs`
- Test: `src/ui/main_window.rs`

**Interfaces:**
- `MainWindow::new() -> MainWindow` creates `AppState` and initial UI state.
- `impl Render for MainWindow` returns the complete window element tree.
- `MainWindow::switch_to(view: View, cx: &mut Context<Self>)` changes the active view and notifies GPUI.
- Shared components provide GPUI elements for buttons, cards, status badges, toolbars and inline messages.

- [ ] **Step 1: Add a root Entity smoke test**

Add `gpui = { version = "=0.2.2", features = ["test-support"] }` under `[dev-dependencies]` so the normal GPUI dependency remains exact `0.2.2` while tests can use `TestAppContext`. Then add a test that creates the root view:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn main_window_entity_can_be_created(cx: &mut TestAppContext) {
        let window = cx.new(|_| super::MainWindow::new());
        cx.read_entity(&window, |view, _| {
            assert_eq!(view.state().current_view(), View::Service);
        });
    }
}
```

If the GPUI 0.2.2 test harness requires a window context, create the test window with the documented `TestAppContext::run` helper and assert the same initial state.

- [ ] **Step 2: Run the smoke test to verify it fails**

Run: `cargo test ui::main_window::tests::main_window_entity_can_be_created`

Expected: FAIL because `MainWindow` and the GPUI test feature are not defined.

- [ ] **Step 3: Implement `MainWindow` and GPUI actions**

Declare the UI modules in `src/ui/mod.rs` and implement `MainWindow` with an `AppState` field. Use mouse handlers for navigation and actions; interpret keyboard events only for add-service search text editing.

    The root layout must include:

```rust
let theme = self.state.theme_colors();
div()
    .size_full()
    .flex()
    .flex_col()
    .bg(gpui::rgb(theme.bg_terminal))
    .child(self.render_header())
    .child(self.render_content(window, cx))
    .child(self.render_status_bar())
```

Use `cx.notify()` after every state-changing event. Route page actions through `MainWindow` mouse handlers so components never mutate `AppState` through an untracked reference.

- [ ] **Step 4: Implement shared GPUI components**

Create reusable button, toolbar, card, status badge and message elements in `components.rs`. Each interactive component must set an id, cursor style, hover/active/focus styling and an `on_click` callback.

- [ ] **Step 5: Reconnect `main.rs` to `MainWindow`**

Replace `BootstrapView` with the real root:

```rust
Application::new().run(|cx| {
    let bounds = Bounds::centered(None, size(px(1100.0), px(720.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(900.0), px(600.0))),
            ..Default::default()
        },
        |_, cx| cx.new(|_| MainWindow::new()),
    )
    .expect("failed to open main window");
    cx.activate(true);
});
```

- [ ] **Step 6: Run root tests and check**

Run: `cargo test ui::main_window::tests`

Expected: PASS, and the root application compiles without ratatui/crossterm imports.

### Task 5: Implement the GPUI service page, asynchronous operations and add-service editor

**Files:**
- Modify: `src/backend.rs`
- Modify: `src/app.rs`
- Modify: `src/ui/main_window.rs`
- Modify: `src/ui/components.rs`
- Modify: `src/ui/service.rs`
- Create: `src/ui/input.rs`
- Test: `src/app.rs`, `src/ui/service.rs`

**Interfaces:**
- `MainWindow::refresh_statuses(&mut self, cx: &mut Context<Self>)` starts a background refresh.
- `MainWindow::start_selected_service(&mut self, cx: &mut Context<Self>)` and `MainWindow::stop_selected_service(&mut self, cx: &mut Context<Self>)` start one operation.
- `MainWindow::start_all_services(&mut self, cx: &mut Context<Self>)` and `MainWindow::stop_all_services(&mut self, cx: &mut Context<Self>)` start batch operations.
- `MainWindow::open_add_dialog(&mut self, cx: &mut Context<Self>)` loads all services asynchronously.
- `ServicePanel` renders the table and calls callbacks supplied by `MainWindow`.
- `SearchInput` implements `Focusable` and `EntityInputHandler` using the GPUI 0.2.2 input pattern.

- [ ] **Step 1: Add service-state tests**

Test that marking a service as starting does not call an external API and that applying a completed status clears the operation message:

```rust
#[test]
fn service_action_state_is_applied_without_external_io() {
    let mut state = AppState::for_test(vec![
        ManagedService { name: "Spooler".into(), display_name: "Print Spooler".into(), enabled: true },
    ]);
    state.mark_service_starting("Spooler");
    assert_eq!(state.service_status("Spooler"), ServiceStatus::Starting);
    state.apply_service_status("Spooler", ServiceStatus::Running);
    assert_eq!(state.service_status("Spooler"), ServiceStatus::Running);
}
```

- [ ] **Step 2: Run the service-state test to verify it fails**

Run: `cargo test app::tests::service_action_state_is_applied_without_external_io`

Expected: FAIL because the typed service-state methods are not implemented.

- [ ] **Step 3: Implement backend operation result types**

Define result types that are `Send + 'static` and contain only owned data:

```rust
pub struct ServiceActionResult {
    pub name: String,
    pub status: ServiceStatus,
    pub message: Option<String>,
}

pub struct RefreshResult {
    pub statuses: Vec<(String, ServiceStatus)>,
}
```

Wrap the existing synchronous `app-service` calls in functions that return these values. Preserve error mapping through `map_error` and return localized messages only at the foreground update boundary.

- [ ] **Step 4: Implement background task orchestration**

For each operation, clone the required service names and language before spawning. Run the synchronous backend function inside `cx.background_spawn`. On completion, upgrade `WeakEntity<MainWindow>`, update `AppState`, clear the task handle and call `cx.notify()`.

Do not capture `&mut Context<Self>` or `&mut AppState` inside the background future. Disable duplicate buttons while the matching operation is `Running`.

- [ ] **Step 5: Implement the service table and toolbar**

Render a scrollable table with columns matching the existing TUI: service name, display name, status and message. Add toolbar buttons for Add, Delete, Start, Stop, Start All, Stop All and Refresh. A row click sets `selected_service`; the add-service dialog selects rows with the mouse.

The delete button removes the selected service, writes the unchanged JSON format, and repairs the selected index.

- [ ] **Step 6: Implement the add-service input and modal**

Create the add-service search input so IME text, Chinese search and backspace editing remain available without global shortcuts. The modal displays a scrollable service list, filters by service name or display name, and uses mouse selection plus visible Add/Cancel buttons.

- [ ] **Step 7: Run service tests and static checks**

Run:

```text
cargo test app::tests
cargo test ui::service::tests
cargo clippy -- -D warnings
```

Expected: PASS with no clippy warnings.

### Task 6: Implement tools, system-info detail, scripts and settings pages

**Files:**
- Modify: `src/ui/tools.rs`
- Modify: `src/ui/scripts.rs`
- Modify: `src/ui/settings.rs`
- Modify: `src/ui/main_window.rs`
- Modify: `src/app.rs`
- Test: `src/app.rs`, `src/ui/tools.rs`, `src/ui/settings.rs`

**Interfaces:**
- `ToolsPanel` displays six entries and enables detail navigation only for the system-info entry.
- `SystemInfoPanel` displays loading, error, retry and loaded states.
- `ScriptsPanel` starts the Navicat cleanup task and displays its result.
- `SettingsPanel` toggles language and theme through `MainWindow` callbacks.

- [ ] **Step 1: Add behavior tests**

Add tests for the implemented tool boundary and settings:

```rust
#[test]
fn only_system_info_tool_can_open_detail() {
    let mut state = AppState::for_test(Vec::new());
    state.current_view = View::Tools;
    state.tools_selected = 1;
    assert!(!state.can_open_tool_detail());
    state.tools_selected = 0;
    assert!(state.can_open_tool_detail());
}

#[test]
fn settings_changes_are_persisted_in_existing_format() {
    let path = std::env::temp_dir().join(format!(
        "rust-win-tool-settings-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut state = AppState::for_test_with_settings_path(path.clone());
    state.cycle_language();
    state.cycle_theme();
    let json = std::fs::read_to_string(&path).expect("settings");
    assert!(json.contains("\"language\""));
    assert!(json.contains("\"theme\""));
    let _ = std::fs::remove_file(path);
}
```

Use a unique path and remove it at the end of the test; do not add a new dependency solely for this test.

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test app::tests::only_system_info_tool_can_open_detail`

Expected: FAIL until tool capability and settings test helpers are implemented.

- [ ] **Step 3: Implement the tools page**

Render the six existing localized tools. The first entry opens `SystemInfoPanel` when clicked; the remaining entries use disabled styling and do not start any backend operation. A visible Back button closes the detail view and returns to the tools list.

- [ ] **Step 4: Implement asynchronous system information**

When the detail view opens, set the state to `Running` and start `backend::fetch_system_info` in a background task. Render the loading message while running, the eight fields after success, and a localized error with a retry action after failure. Do not leave `system_info` in an indefinite `None` state after a failed command or parse.

- [ ] **Step 5: Implement the scripts page**

Keep the existing `SCRIPTS` metadata and Navicat localized labels. On row or button click, start `backend::reset_navicat` in a background task and update `script_result` when it finishes. Display success and failure messages in the page.

- [ ] **Step 6: Implement the settings page**

Render language and theme rows with their current localized values. Mouse clicks call `cycle_language` or `cycle_theme`, persist the existing JSON shape and call `cx.notify()` so the whole theme updates immediately.

- [ ] **Step 7: Run page tests**

Run:

```text
cargo test app::tests
cargo test ui::tools::tests
cargo test ui::settings::tests
```

Expected: PASS.

### Task 7: Remove obsolete TUI code, finalize Windows resources and update project documentation

**Files:**
- Modify: `src/ui/mod.rs`
- Delete or replace: `src/ui/service.rs` ratatui rendering implementation
- Delete or replace: `src/ui/tools.rs` ratatui rendering implementation
- Delete or replace: `src/ui/scripts.rs` ratatui rendering implementation
- Delete or replace: `src/ui/settings.rs` ratatui rendering implementation
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `build.rs`

**Interfaces:**
- All UI modules compile exclusively against GPUI 0.2.2.
- Documentation describes a Windows GPUI GUI, the four pages, actual implemented tools and mouse-only page interaction.
- The release executable uses the existing icon and Windows GUI subsystem.

- [ ] **Step 1: Remove obsolete imports and dependencies**

Run `rg -n "ratatui|crossterm|unicode_width|DefaultTerminal|Frame|render_widget" src Cargo.toml` and remove every remaining TUI reference. Keep `unicode-width` out of `Cargo.toml` unless a non-UI backend test genuinely requires it.

- [ ] **Step 2: Regenerate the lockfile**

Run: `cargo check`

Expected: `Cargo.lock` contains GPUI 0.2.2 and no ratatui/crossterm dependency tree.

- [ ] **Step 3: Verify Windows resource settings**

Run: `cargo build --release`

Expected: the release binary builds with `Subsystem = windows`, the existing `assets/app.ico`, product name `rust-win-tool`, and file description `Windows System Tool`.

- [ ] **Step 4: Update README and AGENTS**

Update the technology table and project structure to mention GPUI, `MainWindow`, `AppState`, `backend.rs` and the four GPUI pages. Remove stale TUI navigation instructions and describe the actual five unimplemented tool entries as disabled placeholders. Document mouse controls and the search field's text-editing behavior.

- [ ] **Step 5: Run documentation consistency search**

Run: `rg -n "ratatui|crossterm|TUI|console|disk cleanup|network diagnostics|process manager" README.md AGENTS.md src Cargo.toml build.rs`

Expected: no stale implementation claims remain; the only allowed TUI/console mentions are historical migration context in documentation.

### Task 8: Full verification and Windows GUI acceptance check

**Files:**
- Test: entire repository

**Interfaces:**
- Produces a verified Windows GUI binary and a clean test/lint/build result.

- [ ] **Step 1: Run all automated checks**

Run:

```text
cargo test
cargo clippy -- -D warnings
cargo build --release
```

Expected: all commands exit with code 0.

- [ ] **Step 2: Verify configuration compatibility**

Use an existing or temporary `config/managed_services.json` and `config/settings.json` beside the executable. Start the release binary and confirm it loads the managed services, selected language and selected theme without rewriting field names or enum values.

- [ ] **Step 3: Verify the service workflow**

In the GUI, verify: service refresh, row selection, add-service search by service name, add-service search by display name, add confirmation, cancel, delete confirmation, single start, single stop, start all, stop all, error message display, and non-blocking interaction while an operation is running.

- [ ] **Step 4: Verify the remaining pages**

Verify: four top tabs and mouse navigation, service operations, system-info loading/success/error/retry, dialog search and mouse selection, Navicat script result, language switch, theme switch, window close, and no global keyboard action handling.

- [ ] **Step 5: Inspect final repository state**

Run: `git status --short --branch`

Expected: only intended GPUI refactor files are modified; no generated secrets, local configuration, or unrelated user changes are included.
