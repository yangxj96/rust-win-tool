# System Tools and Service UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove placeholder system tools while preserving system information, and redesign the service management page as a modern table without changing service operations or persistence.

**Architecture:** Keep `AppState`, `backend`, service persistence, and existing click handlers as the behavior boundary. Narrow the tools catalog to one real entry, then refactor only the GPUI presentation into a summary toolbar plus a card-based table with the existing row-selection and operation callbacks.

**Tech Stack:** Rust 2021, GPUI 0.2.2, `AppState`, `components`, static Chinese/English translations, Cargo test and Clippy.

**Spec:** User-approved in-chat design: keep the service list as a table, remove every placeholder tool except system information, and preserve all existing service actions.

## Global Constraints

- Keep `gpui = "=0.2.2"` and the existing Windows backend dependencies.
- Do not change `AppState` service persistence, `PendingAction`, backend operations, or configuration file formats.
- Preserve add, delete, start, stop, start-all, stop-all, refresh, selection, status, error, and add-dialog behavior.
- Keep the system information async loading/detail flow and the existing navigation views.
- Do not restore global keyboard shortcuts.

### Task 1: Lock the reduced system-tools contract

**Files:**
- Modify: `src/app.rs` (`TOOLS_COUNT` and tool selection test)
- Modify: `src/ui/main_window.rs` (`TOOLS` catalog and catalog test)
- Modify: `src/i18n.rs` (remove placeholder tool translations and update tools description)

**Interfaces:**
- Produces a one-entry `TOOLS` catalog containing `tool_sysinfo` and `tool_sysinfo_desc`.
- `AppState::select_tool` accepts only index `0`; `open_tool_detail` continues to open system information.

- [ ] **Step 1: Write failing tests**

Add an `AppState` test asserting that index `1` cannot be selected and index `0` can open the detail flow. Add a `MainWindow` test asserting `TOOLS.len() == 1` and that its only key is `tool_sysinfo`.

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```text
cargo test only_system_tool_can_be_selected_and_opened
cargo test tools_catalog_contains_only_system_info
```

Expected: the catalog test fails because six entries still exist; the state test fails because `TOOLS_COUNT` is still `6`.

- [ ] **Step 3: Implement the reduced catalog**

Change `TOOLS_COUNT` to `1`, reduce `TOOLS` to one tuple, remove the five placeholder fields and values from both `ZH` and `EN`, update `page_tools_desc`, and keep only the two system-information arms in `tool_text`.

- [ ] **Step 4: Run the focused tests and verify they pass**

Run:

```text
cargo test only_system_tool_can_be_selected_and_opened
cargo test tools_catalog_contains_only_system_info
```

Expected: both tests pass.

### Task 2: Add a tested service summary model for the new table header

**Files:**
- Modify: `src/ui/main_window.rs` (pure summary helper and tests)
- Modify: `src/i18n.rs` (summary labels)
- Modify: `src/ui/components.rs` (compact summary card primitive)

**Interfaces:**
- Produces `ServiceSummary { total, running, stopped, pending }` for the service page.
- `service_summary` counts `Running`, `Stopped`, and `Starting`/`Stopping`/`Refreshing` separately; `Unknown` contributes only to `total`.
- `components::stat_card(label, value, accent, colors)` returns a GPUI `Div` and has no state or side effects.

- [ ] **Step 1: Write the failing summary test**

Add a unit test using statuses `[Running, Stopped, Starting, Stopping, Refreshing, Unknown]` and assert:

```text
ServiceSummary { total: 6, running: 1, stopped: 1, pending: 3 }
```

- [ ] **Step 2: Run the summary test and verify it fails**

Run:

```text
cargo test service_summary_counts_statuses
```

Expected: compilation/test failure because `ServiceSummary` and `service_summary` do not exist.

- [ ] **Step 3: Implement the summary helper and primitive**

Add the small value type and pure counting function, add Chinese/English labels for total/running/stopped/pending, and implement a compact stat card using the existing theme colors and GPUI primitives.

- [ ] **Step 4: Run the summary test and verify it passes**

Run:

```text
cargo test service_summary_counts_statuses
```

Expected: PASS.

### Task 3: Redesign service management presentation while preserving callbacks

**Files:**
- Modify: `src/ui/main_window.rs` (`render_service_page` and table helpers)
- Modify: `src/ui/components.rs` only if the stat-card styling needs shared adjustments

**Interfaces:**
- Keeps the existing IDs and listeners: `service-add`, `service-delete`, `service-start`, `service-stop`, `service-start-all`, `service-stop-all`, `service-refresh`, `service-row`, `service-empty`, and `service-list`.
- Keeps each existing callback mapped to the same `AppState` request method.
- Adds a summary row and preserves the table column widths, including the compact 96px status column.

- [ ] **Step 1: Add a render smoke assertion for the redesigned service surface**

Extend the GPUI render test to render the minimum shell after the service page refactor and retain the existing `service_status_column_is_compact` assertion. The test must exercise empty-state rendering without requiring a Windows service backend.

- [ ] **Step 2: Run the focused render tests and verify the baseline behavior is covered**

Run:

```text
cargo test main_window_render_smoke
cargo test main_window_renders_minimum_shell_size
```

Expected: PASS before the visual refactor, establishing a safe baseline.

- [ ] **Step 3: Refactor the service page layout**

Keep the existing toolbar actions, reorganize them into primary add/refresh controls, selected-service controls, and batch controls; add four summary cards from `service_summary`; render the existing services in a card table with a clear header, compact status badges, selected-row highlight, hover state, message column, scroll behavior, and the existing empty state. Do not add new service operations.

- [ ] **Step 4: Run the focused render tests and verify they pass**

Run:

```text
cargo test main_window_render_smoke
cargo test main_window_renders_minimum_shell_size
cargo test service_status_column_is_compact
```

Expected: all three tests pass.

### Task 4: Full verification and handoff

**Files:**
- Verify: `Cargo.toml`, `Cargo.lock`, `src/app.rs`, `src/i18n.rs`, `src/ui/components.rs`, `src/ui/main_window.rs`

- [ ] **Step 1: Run the complete Rust verification suite**

Run:

```text
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo build --release
git diff --check
```

- [ ] **Step 2: Inspect the diff for behavior preservation**

Confirm that service callbacks still call the original `start_selected`, `stop_selected`, `start_all`, `stop_all`, `refresh_services`, `begin_add_dialog`, and `remove_selected_service` paths, and that only the system-information tool remains in the tools catalog.

- [ ] **Step 3: Report results without committing**

Report changed files, test counts, and the pre-existing dirty worktree state. Do not stage or commit unless the user separately requests it.
