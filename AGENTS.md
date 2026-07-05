# AGENTS.md

## Project

TUI (Terminal User Interface) desktop app — Rust with ratatui frontend. Windows system tool.

## Build Commands

```bash
cargo build
cargo run

cargo clippy -- -D warnings
cargo test
```

## Project Structure

```
├── Cargo.toml                  # Root crate, all deps declared here
├── src/
│   ├── main.rs                 # TUI entrypoint, event loop, rendering
│   ├── app.rs                  # Application state (App, View, Theme, AddDialogState), business logic
│   ├── i18n.rs                 # Internationalization (Language, Translations, ZH/EN)
│   ├── theme.rs                # Theme colors (ThemeColors, DARK/LIGHT)
│   └── ui/                     # UI modules
│       ├── mod.rs              # UI module declarations
│       ├── service.rs          # Service management view + add dialog
│       ├── tools.rs            # Tools list + system info detail view
│       ├── scripts.rs          # Scripts list + Navicat cleanup (winreg)
│       └── settings.rs         # Settings view (language/theme switching)
└── lib/
    └── service/                # Windows service management via PowerShell (app-service crate)
```

## Architecture

- TUI frontend (`src/ui/`) → App state (`src/app.rs`) → lib crate (`lib/service/`)
- Uses ratatui for terminal UI rendering, crossterm for terminal input
- i18n: `src/i18n.rs` defines `Translations` struct with `ZH`/`EN` static instances; access via `app.t()`
- Theme: `app.rs` defines `Theme` enum (`Dark`/`Light`); `src/theme.rs` defines `ThemeColors` with `DARK`/`LIGHT` static instances; access via `app.theme_colors()`
- Scripts: `src/ui/scripts.rs` uses `winreg` crate for direct registry operations; accepts `Language` for i18n error messages
- System info: PowerShell `Get-CimInstance` with UTF8 encoding

## Key Types

- `App` — main application state struct (`src/app.rs`)
- `View` — enum for tab views (`Service`, `Tools`, `Scripts`, `Settings`)
- `Theme` — enum for theme selection (`Dark`, `Light`), serialized via serde
- `AddDialogState` — sub-struct for add-service dialog state
- `PendingAction` — enum for deferred service operations (start/stop/refresh)
- `SystemInfo` — struct holding OS, CPU, RAM info
- `ServiceInfo` / `ManagedService` — from `app-service` crate

## TUI Navigation

### Tab Views
- ←/→ or h/l: Switch view (服务管理 | 系统工具 | 执行脚本 | 设置)

### 服务管理 (Service)
- ↑/↓ or k/j: Select service
- a: Add service (open dialog)
- d: Delete selected service
- s: Start selected service
- S: Start all services
- p: Stop selected service
- P: Stop all services
- r: Refresh all service statuses
- Enter: Confirm (in dialog)

### 系统工具 (Tools)
- ↑/↓: Select tool
- Enter: Open tool detail (system info)
- Esc: Return to tools list

### 执行脚本 (Scripts)
- ↑/↓: Select script
- Enter: Execute selected script

### 设置 (Settings)
- ↑/↓: Select setting
- Enter: Toggle language (index 0) / Toggle theme (index 1)

### Global
- q: Quit application
- Esc: Close dialog / Return from detail view

## Key Dependencies

- `ratatui` 0.30 + `crossterm` 0.29 — TUI rendering and input
- `winreg` 0.55 — Windows registry operations (scripts)
- `unicode-width` 0.2 — CJK character width calculation for alignment
- `serde` / `serde_json` — JSON serialization (settings, PowerShell output)
- `app-service` — PowerShell-based Windows service management (local crate)
