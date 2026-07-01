# AGENTS.md

## Project

Tauri v2 desktop app — Vue 3 + Element Plus frontend, Rust multi-crate backend. Windows system tool.

## Build Commands

```bash
pnpm install              # install deps
pnpm tauri dev            # dev (frontend + Tauri hot-reload)
pnpm tauri build          # production build
pnpm build                # vue-tsc + vite (frontend only)

cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project Structure

```
├── src/                        # Vue 3 frontend
│   ├── App.vue                 # Root, renders MainLayout
│   ├── main.ts                 # Vue + Element Plus + Router
│   ├── layouts/MainLayout.vue  # Sidebar menu + router-view
│   ├── views/
│   │   ├── ServiceManagement.vue  # Service CRUD + start/stop
│   │   └── Settings.vue           # Theme, lang, admin restart
│   └── router/index.ts        # Routes
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml              # All deps declared here
│   ├── src/
│   │   ├── main.rs             # Entrypoint
│   │   └── lib.rs              # Tauri commands + AppState
│   ├── lib/                    # Feature crates
│   │   ├── core/               # App config, error types
│   │   ├── utils/              # UUID, timestamps
│   │   ├── db/                 # DB traits (stub)
│   │   └── service/            # Windows service management via PowerShell
│   └── tauri.conf.json
├── package.json
├── vite.config.ts
└── mise.toml                   # pnpm version pinning
```

## Architecture

- Frontend → `invoke("command_name")` → Rust commands (`src-tauri/src/lib.rs`) → lib crates (`src-tauri/lib/`)
- Register new Tauri commands in `invoke_handler`
- Add new lib crates in `src-tauri/lib/`, add path dep in `src-tauri/Cargo.toml`

## Windows Service Management

- `app-service` crate uses hidden PowerShell (`CREATE_NO_WINDOW`) to call `Get-Service`/`Start-Service`/`Stop-Service`
- No UAC elevation by default — if permission denied, user can restart app as admin via Settings page
- `restart_as_admin` command uses `Start-Process -Verb RunAs` then `app.exit(0)`
- Managed services persisted to `%APPDATA%/rust-win-tool/managed_services.json`

## Frontend Notes

- Element Plus + icons imported globally (`@element-plus/icons-vue`)
- Tauri v2 API: `@tauri-apps/api` (not v1's `@tauri-apps/api/tauri`)
- Sidebar: 200px, collapsible via `Fold`/`Expand` icons
- Table action columns: icon-only buttons with `el-tooltip`, fixed 160px width
- TitleBar z-index: 100 (not 9999) — avoids blocking ElMessage popups
- ElMessage global offset: 40px in `main.ts` to display below TitleBar (32px height)
- Service status/start_type i18n: use `statusText()` and `startTypeText()` helper functions to translate backend values (Running/Stopped/Unknown, Automatic/Manual/Disabled) via `services.statusType.*` and `services.startTypeValue.*` keys

## Rust Notes

- `src-tauri/Cargo.toml` is the workspace root — all deps declared there
- Lib crates use `thiserror` for error types, `serde` for serialization
- `windows_subsystem = "windows"` hides console in release builds
- `app-service` depends on `base64` crate for `-EncodedCommand` encoding
