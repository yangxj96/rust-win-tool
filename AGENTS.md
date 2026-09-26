# AGENTS.md

## Project

GPUI desktop app — Rust Windows system tool.

## Build Commands

```text
cargo build
cargo run
cargo clippy -- -D warnings
cargo test
```

## Project Structure

```text
├── Cargo.toml                  # Root crate, all deps declared here
├── src/
│   ├── main.rs                 # GPUI entrypoint, elevation, single instance, tray
│   ├── app.rs                  # AppState, navigation, persistence-facing state
│   ├── backend.rs              # Async-task targets: native services, system info, registry
│   ├── cleanup.rs              # Junk file scan/clean (native, recycle bin)
│   ├── file_dialog.rs          # Native open/save file dialogs
│   ├── logging.rs              # Append-only file logger (config/logs)
│   ├── monitor.rs              # Live CPU/memory/network/disk sampling
│   ├── network.rs              # DNS, TCP port checks and ICMP ping
│   ├── port.rs                 # Port occupancy lookup via IP Helper
│   ├── process.rs              # Process listing, termination and PID name lookup
│   ├── scripts.rs              # Custom script library persistence
│   ├── startup.rs              # Startup item listing and toggling
│   ├── tray.rs                 # System tray icon and menu (Shell_NotifyIcon)
│   ├── i18n.rs                 # Internationalization (Language, Translations, ZH/EN)
│   ├── theme.rs                # Framework-neutral theme colors
│   └── ui/
│       ├── mod.rs              # UI module declarations
│       ├── main_window.rs      # GPUI root entity, pages and event handlers
│       ├── components.rs       # Shared GPUI visual components
│       └── input.rs            # Keyboard input policy
└── lib/service/                # Windows service management via native Win32 API
```

## Architecture

- GPUI `MainWindow` renders the `AppState` entity.
- `AppState` owns navigation, selection, status, settings, and dialog state.
- `MainWindow` owns the fixed 220px sidebar, custom titlebar, window controls, and right-side content shell.
- `src/ui/components.rs` contains shared Element Plus-inspired visual primitives: cards, buttons, sidebar items, status badges, and window control hit areas.
- Blocking service, system-information, and registry operations run through GPUI background tasks.
- `src/i18n.rs` defines `Translations` static instances; access via `state.t()`.
- `src/theme.rs` stores RGB values; the GPUI layer converts them to `Rgba`.
- Configuration JSON and paths remain compatible with the console version.

## Key Types

- `AppState` — application state and persistence-backed behavior
- `View` — `Service`, `Tools`, `Cleanup`, `Scripts`, `Settings`
- `Theme` — `Dark`/`Light`, serialized in `settings.json`
- `ServiceStatus` — typed service status used by the UI
- `OperationState` — typed loading/operation state
- `AddDialogState` — add-service search dialog state
- `SystemInfo` — typed system information model

## Interaction Rules

- All navigation and actions use the visible mouse controls and clickable rows.
- The titlebar is custom rendered by GPUI; `WindowControlArea::Drag` provides native Windows dragging, while visible GPUI controls invoke minimize, maximize, and close through `Window` APIs.
- No global keyboard shortcuts are registered. Keyboard input is only interpreted as text editing in the add-service search field.
- Only the system-information tool and Navicat cleanup are implemented because those were the implemented features in the original version.
- PowerShell processes are hidden on Windows and must not block the GPUI event loop.

## Dependencies

- `gpui = "=0.2.2"`
- `serde` / `serde_json` — settings and service data compatibility
- `thiserror` — backend error model
- `winreg` — Windows registry cleanup
- `app-service` — local Windows service management crate
