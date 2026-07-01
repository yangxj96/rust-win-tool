# rust-win-tool

Windows 系统管理工具，基于 Tauri v2 + Vue 3 + Element Plus 构建。

## 功能

- **服务管理** — 查看、启动、停止 Windows 服务，支持持久化管理列表
- **系统设置** — 主题切换、语言切换、以管理员身份重启

## 技术栈

| 层 | 技术 |
|---|------|
| 前端 | Vue 3 + Element Plus + Vue Router |
| 后端 | Rust (Tauri v2, 多 crate 架构) |
| 构建 | pnpm + Vite + Cargo |

## 开发

```bash
pnpm install           # 安装依赖
pnpm tauri dev         # 启动开发模式（前端 + Tauri 热重载）
```

## 构建

```bash
pnpm tauri build       # 生产构建
```

## 项目结构

```
├── src/                        # Vue 3 前端
│   ├── layouts/MainLayout.vue  # 侧边栏布局
│   ├── views/
│   │   ├── ServiceManagement.vue  # 服务管理
│   │   └── Settings.vue           # 系统设置
│   └── router/index.ts        # 路由
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # 入口
│   │   └── lib.rs              # Tauri 命令 + AppState
│   └── lib/                    # 功能 crate
│       ├── core/               # 应用配置、错误类型
│       ├── utils/              # UUID、时间戳
│       ├── db/                 # 数据库（桩）
│       └── service/            # Windows 服务管理
└── AGENTS.md                   # AI 代理开发指引
```

## License

MIT
