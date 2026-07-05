# AGENTS.md

## Project

TUI (Terminal User Interface) desktop app — Rust multi-crate backend with ratatui frontend. Windows system tool.

## Build Commands

```bash
cargo build
cargo run

cargo clippy -- -D warnings
cargo test
```

## Project Structure

```
├── Cargo.toml                  # Workspace root, all deps declared here
├── src/
│   ├── main.rs                 # TUI entrypoint
│   ├── app.rs                  # Application state management
│   └── ui/                     # UI modules
│       ├── mod.rs              # UI module declarations
│       ├── service.rs          # Service management TUI
│       ├── settings.rs         # Settings TUI
│       └── tools.rs            # Tools collection TUI
└── lib/                        # Feature crates
    ├── core/                   # App config, error types
    ├── utils/                  # UUID, timestamps
    └── service/                # Windows service management via PowerShell
```

## Architecture

- TUI frontend (`src/ui/`) → App state (`src/app.rs`) → lib crates (`lib/`)
- Uses ratatui for terminal UI rendering
- Uses crossterm for terminal input handling
- Add new UI modules in `src/ui/`
- Add new lib crates in `lib/`, add path dep in `Cargo.toml`

## Windows Service Management

- `app-service` crate uses hidden PowerShell (`CREATE_NO_WINDOW`) to call `Get-Service`/`Start-Service`/`Stop-Service`
- No UAC elevation by default — if permission denied, user can restart app as admin
- Managed services persisted to `%APPDATA%/rust-win-tool/managed_services.json`

## TUI Navigation

- ←/→: Switch view (Service / Settings / Tools)
- ↑/↓: Select item
- a: Add service (open dialog)
- d: Delete selected service
- s: Start selected service
- p: Stop selected service
- r: Refresh service status
- Enter: Confirm / Execute
- Esc: Close dialog
- q: Quit application

## Rust Notes

- `Cargo.toml` is the workspace root — all deps declared there
- Lib crates use `thiserror` for error types, `serde` for serialization
- `app-service` depends on `base64` crate for `-EncodedCommand` encoding
- Uses ratatui 0.30 and crossterm 0.29 for TUI

