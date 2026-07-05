# rust-win-tool

Windows 系统管理工具，基于 Rust + ratatui 构建的 TUI (终端用户界面) 应用。

## 功能

- **服务管理** — 查看、启动、停止 Windows 服务，支持持久化管理列表
- **系统设置** — 主题切换、语言切换
- **工具集合** — 系统信息、磁盘清理、网络诊断等工具

## 技术栈

| 层 | 技术 |
|---|------|
| 前端 | ratatui + crossterm (TUI) |
| 后端 | Rust (多 crate 架构) |
| 构建 | Cargo |

## 开发

```bash
cargo build
cargo run
```

## 构建

```bash
cargo build --release
```

## 项目结构

```
├── Cargo.toml              # Workspace root
├── src/
│   ├── main.rs             # TUI 入口
│   ├── app.rs              # 应用状态管理
│   └── ui/                 # UI 模块
│       ├── service.rs      # 服务管理界面
│       ├── settings.rs     # 设置界面
│       └── tools.rs        # 工具集合界面
└── lib/                    # 功能 crate
    ├── core/               # 应用配置、错误类型
    ├── utils/              # UUID、时间戳
    └── service/            # Windows 服务管理
```

## TUI 导航

- 按 1: 服务管理视图
- 按 2: 设置视图
- 按 3: 工具视图
- 按 q: 退出应用

## License

MIT
