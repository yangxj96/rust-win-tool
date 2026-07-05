# AGENTS.md

## Project

TUI (Terminal User Interface) desktop app — Rust multi-crate backend with ratatui frontend. Windows system tool.

## Build Commands

```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo run --manifest-path src-tauri/Cargo.toml

cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project Structure

```
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml              # All deps declared here
│   ├── src/
│   │   ├── main.rs             # TUI entrypoint
│   │   ├── app.rs              # Application state management
│   │   ├── lib.rs              # Library functions
│   │   └── ui/                 # UI modules
│   │       ├── mod.rs          # UI module declarations
│   │       ├── service.rs      # Service management TUI
│   │       ├── settings.rs     # Settings TUI
│   │       └── tools.rs        # Tools collection TUI
│   └── lib/                    # Feature crates
│       ├── core/               # App config, error types
│       ├── utils/              # UUID, timestamps
│       ├── db/                 # DB traits (stub)
│       └── service/            # Windows service management via PowerShell
```

## Architecture

- TUI frontend (`src-tauri/src/ui/`) → App state (`src-tauri/src/app.rs`) → lib crates (`src-tauri/lib/`)
- Uses ratatui for terminal UI rendering
- Uses crossterm for terminal input handling
- Add new UI modules in `src-tauri/src/ui/`
- Add new lib crates in `src-tauri/lib/`, add path dep in `src-tauri/Cargo.toml`

## Windows Service Management

- `app-service` crate uses hidden PowerShell (`CREATE_NO_WINDOW`) to call `Get-Service`/`Start-Service`/`Stop-Service`
- No UAC elevation by default — if permission denied, user can restart app as admin
- Managed services persisted to `%APPDATA%/rust-win-tool/managed_services.json`

## TUI Navigation

- Press 1: Services view
- Press 2: Settings view
- Press 3: Tools view
- Press q: Quit application

## Rust Notes

- `src-tauri/Cargo.toml` is the workspace root — all deps declared there
- Lib crates use `thiserror` for error types, `serde` for serialization
- `app-service` depends on `base64` crate for `-EncodedCommand` encoding
- Uses ratatui 0.30 and crossterm 0.29 for TUI

## Git Rules

- 每次git提交的时候都需要包含新增和修改文件