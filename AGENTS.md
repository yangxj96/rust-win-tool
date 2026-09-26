# AGENTS.md

## 项目

使用 GPUI 构建的 Rust Windows 桌面系统工具。

## 构建命令

```text
cargo build
cargo run
cargo clippy -- -D warnings
cargo test
```

## 项目结构

```text
├── Cargo.toml                  # 根 crate，所有依赖均在此声明
├── src/
│   ├── main.rs                 # 精简入口，只声明顶层模块并调用 app::run()
│   ├── app.rs                  # 应用门面，导出状态类型并启动 GPUI
│   ├── app/
│   │   ├── bootstrap.rs        # 提权、单实例、日志、窗口和托盘启动顺序
│   │   ├── state.rs            # AppState、状态模型和通用状态逻辑
│   │   └── state/
│   │       ├── services.rs     # 服务列表、筛选、详情、操作和添加对话框状态
│   │       ├── cleanup.rs      # 清理页面状态
│   │       ├── scripts.rs      # 脚本页面、脚本对话框和执行结果状态
│   │       ├── tools.rs        # 工具页面、进程、端口、监控、网络和启动项状态
│   │       ├── settings.rs     # 语言、主题与设置持久化
│   │       └── tests.rs        # AppState 单元测试
│   ├── features/               # 按服务、清理、脚本和工具功能域组织的业务实现
│   │   ├── services/           # 服务数据、Windows 服务操作
│   │   ├── cleanup/            # 垃圾文件扫描、清理和 Navicat 规则
│   │   ├── scripts/            # 自定义脚本数据和命令执行
│   │   └── tools/              # 系统信息、进程、端口、监控、网络和启动项
│   ├── platform/               # Windows 平台集成
│   │   ├── file_dialog.rs      # 原生文件打开和保存对话框
│   │   └── windows/            # 提权、单实例和系统托盘
│   ├── shared/                 # 跨功能域共享的错误类型
│   ├── support/                # 日志等支撑能力
│   └── ui/
│       ├── mod.rs              # UI 模块声明
│       ├── main_window/         # GPUI 根实体、页面和事件处理
│       ├── components.rs       # GPUI 共享视觉组件
│       ├── i18n.rs             # 界面语言和中英文文案
│       ├── theme.rs            # 界面主题颜色
│       └── input.rs            # 键盘输入策略
```

## 架构

- GPUI 的 `MainWindow` 负责渲染 `AppState` 实体。
- `AppState` 管理页面导航、选择、状态、设置和对话框状态。
- `MainWindow` 管理固定宽度为 220px 的侧边栏、自定义标题栏、窗口控件和右侧内容区域。
- `src/ui/components.rs` 存放受 Element Plus 启发的共享视觉组件，包括卡片、按钮、侧边栏项、状态徽标和窗口控制点击区域。
- 服务、系统信息和注册表等阻塞操作通过 GPUI 后台任务执行。
- `src/ui/i18n.rs` 定义 `Translations` 静态实例；通过 `state.t()` 访问。
- `src/ui/theme.rs` 保存 RGB 颜色值，再由 GPUI 层转换为 `Rgba`。
- 配置 JSON 格式和路径与控制台版本保持兼容。

## 主要类型

- `AppState` — 应用状态以及依赖持久化的行为
- `View` — `Service`、`Tools`、`Cleanup`、`Scripts`、`Settings` 页面
- `Theme` — `Dark` / `Light`，序列化后存入 `settings.json`
- `ServiceStatus` — UI 使用的服务状态类型
- `OperationState` — 操作和加载状态类型
- `AddDialogState` — 添加服务搜索对话框状态
- `SystemInfo` — 系统信息数据模型

## 交互规则

- 所有导航和操作均通过界面上可见的鼠标控件和可点击行完成。
- 标题栏由 GPUI 自定义绘制；`WindowControlArea::Drag` 提供 Windows 原生窗口拖动，可见的最小化、最大化和关闭控件通过 `Window` API 执行。
- 不注册全局键盘快捷键。键盘输入只在“添加服务”搜索框中作为文本编辑处理。
- 目前只实现系统信息工具和 Navicat 清理；这两项也是原始版本中已实现的功能。
- Windows 上的 PowerShell 进程以隐藏窗口方式运行，且不得阻塞 GPUI 事件循环。

## 依赖

- `gpui = "=0.2.2"`
- `serde` / `serde_json` — 设置和服务数据兼容性
- `thiserror` — 后端错误类型
- `winreg` — Windows 注册表清理
- `windows-sys` — Windows 原生 API 绑定，包括服务控制管理器 API
