# Element Plus 风格 GPUI 窗口壳层 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 GPUI 应用重排为固定宽度左侧导航、右侧卡片内容区和完全自绘无边框标题栏，并保持现有业务功能与配置兼容性。

**Architecture:** `MainWindow` 继续作为唯一根 Entity，负责组合自绘标题栏、固定侧栏和可伸缩内容区。窗口按钮通过 GPUI 0.2.2 的 `WindowControlArea::{Drag, Min, Max, Close}` 注册为 Windows 原生 hit-test 区域；主题 token 和可复用视觉组件分别收敛到 `theme.rs` 与 `ui/components.rs`，页面业务回调和后台任务保持现有边界。

**Tech Stack:** Rust 2021, GPUI `=0.2.2`, serde/serde_json, local `app-service`, winreg, winresource, Windows native window hit-testing.

**Spec:** `docs/superpowers/specs/2026-08-28-element-plus-window-shell-design.md`

## Global Constraints

- 左侧导航宽度固定为 `220px`，不随右侧内容滚动。
- `WindowOptions.titlebar.appears_transparent = true` 隐藏 Windows 默认标题栏。
- 标题栏空白区使用 `WindowControlArea::Drag`，按钮使用 `Min`、`Max`、`Close`。
- 保持 `is_movable = true`、`is_resizable = true`、`is_minimizable = true`。
- 页面导航和业务操作只使用可见鼠标控件；不恢复全局快捷键。
- 添加服务搜索框保留文字输入和退格编辑能力。
- 不改变 `config/managed_services.json`、`config/settings.json` 的路径、字段名和序列化值。
- 不引入 `gpui-component` 或其他第三方 UI 组件库。
- 所有状态变化继续通过 `MainWindow` 更新并调用 `cx.notify()`；后台服务、PowerShell 和注册表操作继续使用 GPUI background task。
- 保留当前工作区已有未提交 GPUI 重构改动，不使用 `git reset`、`git checkout` 或 `git clean` 覆盖它们。

## File Map

- Modify `src/main.rs`: 配置无边框标题栏、窗口可移动/可缩放属性，并继续触发初始服务刷新。
- Modify `src/theme.rs`: 增加 Element Plus 语义色 token，包括侧栏背景、成功色、警告色和危险色。
- Modify `src/i18n.rs`: 增加应用副标题和标题栏/页面所需文案，删除不再显示的底部快捷键提示文案。
- Modify `src/ui/components.rs`: 提供窗口控制按钮、侧栏导航项、页面 header、卡片和状态徽章的统一样式。
- Modify `src/ui/main_window.rs`: 组装标题栏/侧栏/内容区，重排四个页面并注册窗口控制区域；不改变后台操作逻辑。
- Modify `README.md`: 更新布局、窗口控制和 Element Plus 风格说明。
- Modify `AGENTS.md`: 更新项目结构和窗口交互规则。
- Modify `docs/superpowers/plans/2026-08-28-element-plus-window-shell.md`: 按执行进度勾选步骤。

---

### Task 1: Add failing tests for the new visual contract

**Files:**
- Modify: `src/theme.rs`
- Modify: `src/ui/main_window.rs`
- Test: existing inline test modules in the same files

**Interfaces:**
- Produces theme token assertions for later component work.
- Produces a GPUI minimum-size render smoke test that later shell changes must keep passing.

- [x] **Step 1: Write the failing theme token test**

Add this test to `src/theme.rs` before adding the fields:

```rust
#[cfg(test)]
mod tests {
    use super::{DARK, LIGHT};

    #[test]
    fn element_plus_tokens_keep_status_roles_distinct() {
        assert_eq!(LIGHT.primary, 0x409eff);
        assert_eq!(LIGHT.success, 0x67c23a);
        assert_eq!(LIGHT.warning, 0xe6a23c);
        assert_eq!(LIGHT.danger, 0xf56c6c);
        assert_ne!(DARK.bg_sidebar, DARK.bg_window);
        assert_ne!(LIGHT.bg_sidebar, LIGHT.bg_window);
    }
}
```

- [x] **Step 2: Run the focused test and confirm the expected failure**

Run `cargo test theme::tests::element_plus_tokens_keep_status_roles_distinct`.

Expected: compile failure because `ThemeColors` does not yet expose `success`, `warning`, `danger`, or `bg_sidebar`.

- [x] **Step 3: Add minimum-size GPUI render coverage**

Add this test beside the existing `main_window_render_smoke` test in `src/ui/main_window.rs`:

```rust
#[gpui::test]
fn main_window_renders_minimum_shell_size(cx: &mut TestAppContext) {
    let state = cx.new(|_| AppState::new());
    let (view, visual_cx) = cx.add_window_view(|_, cx| MainWindow::new(state.clone(), cx));
    let _ = visual_cx.draw(point(px(0.), px(0.)), size(px(900.), px(600.)), |_, _| {
        view.clone()
    });
}
```

Run `cargo test ui::main_window::tests::main_window_renders_minimum_shell_size`; it may pass against the current layout and remains as the shell regression test.

### Task 2: Implement the Element Plus theme and shared visual primitives

**Files:**
- Modify: `src/theme.rs`
- Modify: `src/ui/components.rs`
- Modify: `src/i18n.rs`
- Test: `src/theme.rs`

**Interfaces:**
- `ThemeColors` exposes `primary`, `success`, `warning`, `danger`, `bg_window`, `bg_sidebar`, `bg_surface`, `bg_select`, `border`, `fg_default`, `inactive`, and `error` as framework-neutral `u32` values.
- `components::button` keeps its current call signature for existing business pages.
- Add `components::sidebar_item(label, colors, selected) -> Div`.
- Add `components::window_control(label, area, colors, danger) -> Div`; the returned element calls `.window_control_area(area)`.
- Add `components::card(colors) -> Div` and `components::status_badge(label, color) -> Div`.

- [x] **Step 1: Add the missing theme fields and make the focused test pass**

Extend `ThemeColors` and both constants with:

```rust
pub success: u32,
pub warning: u32,
pub danger: u32,
pub bg_sidebar: u32,
```

Use light `#409EFF/#67C23A/#E6A23C/#F56C6C/#FFFFFF` and dark `#66B1FF/#85CE61/#EBB563/#F78989/#111827` for primary, success, warning, danger, and sidebar respectively. Run the focused theme test.

- [x] **Step 2: Implement shared primitives with concrete styles**

Keep the existing `button` signature but use the new semantic colors. Add these concrete helpers:

```rust
pub fn card(colors: &ThemeColors) -> Div {
    div()
        .w_full()
        .rounded_lg()
        .border_1()
        .border_color(color(colors.border))
        .bg(color(colors.bg_surface))
}

pub fn sidebar_item(
    label: impl Into<SharedString>,
    colors: &ThemeColors,
    selected: bool,
) -> Div {
    let foreground = if selected { colors.primary } else { colors.fg_default };
    div()
        .flex()
        .items_center()
        .h(px(42.))
        .px_3()
        .rounded_md()
        .border_l_3()
        .border_color(color(if selected { colors.primary } else { colors.bg_sidebar }))
        .bg(color(if selected { colors.bg_select } else { colors.bg_sidebar }))
        .text_color(color(foreground))
        .cursor_pointer()
        .hover(|style| style.bg(color(colors.bg_select)))
        .child(label.into())
}
```

Implement `window_control` as a 46px-wide, 48px-high flex-centered `Div` with `id` assigned by the caller, `window_control_area(area)`, muted hover styling, and `colors.danger` hover background when `danger` is true. Implement `status_badge` as a compact rounded pill using the supplied semantic color. Do not add an icon dependency.

- [x] **Step 3: Add required localized chrome text**

Add `app_subtitle` to `Translations`, using Chinese `Windows 系统工具` and English `Windows System Utility`. Remove `hint_mouse` because the footer is removed; retain labels used by visible page buttons and use existing `tab_*` values for navigation.

- [x] **Step 4: Run theme/component checks**

Run:

```text
cargo test theme::tests::element_plus_tokens_keep_status_roles_distinct
cargo clippy -- -D warnings
```

Expected: focused test passes and Clippy has no warnings.

---

### Task 3: Configure the fully custom Windows titlebar

**Files:**
- Modify: `src/main.rs:1-35`
- Modify: `src/ui/main_window.rs:1-40, 880-915`
- Test: `src/ui/main_window.rs` render smoke tests

**Interfaces:**
- `MainWindow::render_titlebar(&self, window: &Window, state: &AppState) -> Div` returns the 48px custom titlebar.
- The titlebar registers one `Drag`, one `Min`, one `Max`, and one `Close` hitbox.

- [x] **Step 1: Configure WindowOptions for transparent titlebar rendering**

Import `TitlebarOptions` and set the window options as follows while preserving current bounds:

```rust
WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    window_min_size: Some(size(px(900.), px(600.))),
    titlebar: Some(TitlebarOptions {
        title: Some("Rust Win Tool".into()),
        appears_transparent: true,
        traffic_light_position: None,
    }),
    is_movable: true,
    is_resizable: true,
    is_minimizable: true,
    ..Default::default()
}
```

- [x] **Step 2: Add structural render coverage for the titlebar**

Keep the minimum-size GPUI smoke test from Task 1 and add stable ids to the titlebar: `window-titlebar`, `window-minimize`, `window-maximize`, and `window-close`. GPUI 0.2.2's headless test window does not expose a reliable Windows hit-test assertion, so compile coverage plus stable structure is the test boundary for `WindowControlArea`.

- [x] **Step 3: Implement the titlebar layout and native hitboxes**

In `MainWindow::render_titlebar`, render a 48px row with app mark/title, a flexible drag region, and three controls. Use `components::window_control` for the buttons and pass `window.is_maximized()` to choose the maximize/restore glyph:

```rust
div()
    .id("window-titlebar")
    .h(px(48.))
    .w_full()
    .flex()
    .items_center()
    .border_b_1()
    .border_color(components::color(colors.border))
    .bg(components::color(colors.bg_surface))
    .child(app_mark_and_title)
    .child(div().flex_1().window_control_area(WindowControlArea::Drag))
    .child(components::window_control("—", WindowControlArea::Min, colors, false))
    .child(components::window_control(
        if window.is_maximized() { "❐" } else { "□" },
        WindowControlArea::Max,
        colors,
        false,
    ))
    .child(components::window_control("×", WindowControlArea::Close, colors, true))
```

Ensure the drag element does not overlap the three control hitboxes. Do not attach business `on_click` handlers to these controls and do not add window state to `AppState`; GPUI/Windows owns the actual operation.

- [x] **Step 4: Replace the old root header/footer composition**

Change `Render::render` to accept `window: &mut Window`, call `render_titlebar(window, state)`, and remove the old top tab header and footer from the root. Keep page rendering reachable through the new content region so this task remains buildable.

- [x] **Step 5: Run shell smoke tests and build**

Run:

```text
cargo test ui::main_window::tests
cargo clippy -- -D warnings
cargo build --release
```

Expected: the executable still builds as a Windows GUI, and the test window renders at `900 × 600`.

### Task 4: Add the fixed sidebar and route page navigation through it

**Files:**
- Modify: `src/ui/main_window.rs`
- Modify: `src/ui/components.rs`
- Test: `src/ui/main_window.rs` render smoke tests

**Interfaces:**
- `MainWindow::render_sidebar(&self, state: &AppState, cx: &Context<Self>) -> Div` returns a non-scrolling `220px` sidebar.
- Existing `select_view(view, event, window, cx)` remains the sole navigation callback.

- [x] **Step 1: Compose the shell body**

Compose the root body after the titlebar as:

```rust
div()
    .flex_1()
    .flex()
    .child(self.render_sidebar(state, cx))
    .child(
        div()
            .flex_1()
            .flex_col()
            .child(self.render_content_header(state))
            .child(div().flex_1().child(page)),
    )
```

The sidebar must not have `flex_1` or a scroll container; the content column owns flexible width and page scroll regions.

- [x] **Step 2: Implement the sidebar identity block**

Render a 220px column using `bg_sidebar`, a right border, 20px horizontal padding, and a 52px identity area. Show a compact blue app mark, `app_title`, and `app_subtitle`; use existing localized `tab_*` labels for navigation.

- [x] **Step 3: Implement four full-row navigation items**

Map `(View::Service, tab_service)`, `(View::Tools, tab_tools)`, `(View::Scripts, tab_scripts)`, and `(View::Settings, tab_settings)` into `components::sidebar_item`. Assign stable ids `sidebar-service`, `sidebar-tools`, `sidebar-scripts`, and `sidebar-settings`; each callback calls `select_view` with its captured `View`.

The active item uses `bg_select`, `primary`, and a visible left accent; inactive items use `fg_default`/`inactive` and a hover background. The page title no longer depends on the old top Tab rendering.

- [x] **Step 4: Replace the footer with a content header**

Remove `render_footer` and `hint_mouse`. Add `render_content_header` that shows the current page title, a concise localized description, and spacing for page actions. Keep service toolbar actions in the service page so page-specific callbacks remain local.

- [x] **Step 5: Run navigation/render checks**

Run:

```text
cargo test ui::main_window::tests
cargo clippy -- -D warnings
```

Expected: all smoke tests pass, and no keyboard shortcut handler or footer hint is reintroduced.

---

### Task 5: Restyle the service page and add-service dialog without changing business callbacks

**Files:**
- Modify: `src/ui/main_window.rs`
- Modify: `src/ui/components.rs`
- Modify: `src/i18n.rs` for service page copy
- Test: existing `src/app.rs` dialog tests, `src/ui/input.rs` tests, and GPUI render smoke tests

**Interfaces:**
- Existing callbacks remain: `add_clicked`, `delete_clicked`, `start_clicked`, `stop_clicked`, `start_all_clicked`, `stop_all_clicked`, `refresh_clicked`, `select_service`, `select_dialog_service`, `confirm_dialog_clicked`, and `cancel_dialog_clicked`.
- `AppState` service operation and search methods remain unchanged.

- [x] **Step 1: Wrap the service toolbar in a page header/card**

Render the service title and a compact managed-service count in the right content header. Put the existing Add/Delete/Start/Stop/Start All/Stop All/Refresh buttons in a bordered toolbar card with consistent 8px gaps. Keep every existing button id and callback.

- [x] **Step 2: Restyle the service table as a surface card**

Wrap the table header and rows in `components::card(colors)`, use a subtle header background, 44–48px row height, 12px horizontal padding, and Element Plus status colors. Keep row-click selection; map Running to `success`, Stopped/Unknown to `danger`, and pending/refreshing to `warning`.

- [x] **Step 3: Improve empty, error, and loading states**

Keep existing state strings and error mapping, but center each state inside the table card with muted or danger colors. The empty state must instruct the user to click the visible Add button and must not mention a keyboard shortcut.

- [x] **Step 4: Restyle the add-service modal while preserving text editing**

Keep the absolute overlay and root focus handling. Use a centered surface card, bordered search field, selectable rows with hover/active styles, and visible Cancel/Add buttons. Keep `input::text_editing_input` as the only interpreted key input; do not add Enter/Esc/arrows or page-level shortcuts. Keep mouse row selection through `select_dialog_service`.

- [x] **Step 5: Run service-focused tests**

Run:

```text
cargo test app::tests
cargo test ui::input::tests
cargo test ui::main_window::tests
```

Expected: service filtering, selection, input policy, and shell render tests pass, with no backend calls added to Render.

### Task 6: Restyle tools, system information, scripts, and settings pages

**Files:**
- Modify: `src/ui/main_window.rs`
- Modify: `src/ui/components.rs` for shared card/badge styles
- Modify: `src/i18n.rs` for concise page descriptions and detail labels
- Test: existing `src/app.rs` and `src/ui/main_window.rs` tests

**Interfaces:**
- Existing mouse callbacks and async operations remain the business entry points.
- The system information Back button remains visible and calls `state.close_tool_detail()` through `MainWindow`.
- Settings rows continue to call `cycle_language`/`cycle_theme` and persist the same settings JSON.

- [x] **Step 1: Convert the tools list to modern cards**

Render each tool as a bordered card with a compact marker, title, description, and selected/hover state. Keep only system information actionable; clicking it calls `select_tool` and `open_tool`. Keep the five placeholder entries visible and non-executing.

- [x] **Step 2: Restyle the system-information detail page**

Use a page header with a visible Back button and a two-column key/value card. Keep the eight existing fields and loading/error states. Retain asynchronous `backend::fetch_system_info()` and do not move it into Render.

- [x] **Step 3: Convert the scripts page to an action card**

Use a card with title, description, result area, and visible action button. Preserve row/button click behavior and `backend::reset_navicat()` background execution.

- [x] **Step 4: Convert settings to clickable setting cards**

Use one card per setting with label, current value, and a right-side visual affordance. Clicking a row selects it and toggles the matching setting. Keep language/theme persistence and repaint all shell colors after a theme change.

- [x] **Step 5: Run page and persistence tests**

Run:

```text
cargo test
cargo clippy -- -D warnings
```

Expected: all existing tests pass, including typed system-information parsing and dialog behavior.

### Task 7: Update documentation and verify the final Windows shell

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/plans/2026-08-28-element-plus-window-shell.md`

**Interfaces:**
- Documentation describes the actual fixed sidebar, custom titlebar, native window controls, mouse-only page interaction, and preserved business features.

- [x] **Step 1: Update README**

Document the four-item left navigation, right content cards, custom titlebar controls, minimum window size, light/dark Element Plus palette, and absence of global keyboard shortcuts. Keep the search field text-editing note.

- [x] **Step 2: Update AGENTS.md**

Replace top-tab/footer assumptions with the shell structure. State that `WindowControlArea` owns native titlebar hit-testing and page actions must remain mouse-visible.

- [x] **Step 3: Run automated verification**

Run from the repository root:

```text
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo build --release
git diff --check
```

Expected: every command exits 0; tests report no failures and Clippy reports no warnings.

- [x] **Step 4: Run consistency searches**

Run:

```text
rg -n "ratatui|crossterm|handle_main_key|hint_mouse|q.*退出|按 a|Enter.*打开|Esc.*返回|hint_switch|hint_select|hint_quit" src README.md AGENTS.md
```

Expected: no stale console UI or global shortcut references remain in current source/docs. Historical migration wording may remain only in committed design history where explicitly contextual.

- [x] **Step 5: Run Windows GUI acceptance checks**

Use the release executable to verify:

1. No default Windows titlebar is visible.
2. Dragging titlebar blank space moves the window.
3. Minimize, maximize/restore, and close buttons work through native Windows behavior.
4. The sidebar remains exactly 220px while the right content expands after maximize.
5. All four pages, service operations, add-service search/selection, system-info Back, script action, language, and theme remain usable with the mouse.
6. The window stays responsive while service, system-information, and script background tasks run.

- [x] **Step 6: Review the final worktree before any commit**

Run:

```text
git status --short --branch
git diff --stat
git diff --check
```

Confirm no `config/`, credentials, generated binaries, or local settings are staged. Ask for explicit commit confirmation before staging or committing implementation changes.

## Execution Notes

- Execute one checkbox step at a time and keep the focused test green before moving to the next task.
- Edit repository files with `apply_patch`; do not use shell redirection or ad-hoc file writers.
- Preserve unrelated existing changes. Stage exact files only if a commit is explicitly requested.
- The design spec is committed as `2df2500`; implementation changes remain separate until the user requests a commit.
