# rust-win-tool

Windows 系统管理桌面应用，基于 Rust + GPUI 构建。应用保留原控制台版本的已实现功能：Windows 服务管理、系统信息、Navicat 注册表清理，以及语言和主题设置。

## 功能

- 服务管理：加载持久化服务列表，查看状态，启动/停止单个或全部服务，添加和删除管理项
- 系统信息：异步读取 Windows 版本、CPU、核心数和内存
- 执行脚本：异步执行 Navicat 注册表清理
- 设置：中文/英文切换、深色/浅色主题切换
- 窗口壳层：GPUI 自绘无边框标题栏，支持拖拽、最小化、最大化和关闭
- 布局：固定宽度左侧导航栏、右侧内容区，采用现代化卡片和状态徽标
- 配置：继续使用可执行文件旁的 `config/managed_services.json` 和 `config/settings.json`

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | GPUI 0.2.2 |
| 状态 | Rust Entity + `AppState` |
| 后端 | PowerShell、Windows Registry、`app-service` |
| 构建 | Cargo + `winresource` |

## 开发

```text
cargo build
cargo run
cargo test
cargo clippy -- -D warnings
```

## 界面操作

- 鼠标：点击左侧导航、服务行、工具项、操作按钮和对话框按钮
- 窗口：拖拽自绘标题栏移动窗口，使用右上角控件最小化、最大化或关闭
- 添加服务对话框：直接输入文字搜索，点击列表项选择服务，再点击“添加”或“取消”
- 应用不再注册全局键盘快捷键；键盘输入仅用于添加服务搜索框的文字编辑

## 项目结构

```text
├── Cargo.toml
├── src/
│   ├── main.rs              # GPUI 启动与窗口配置
│   ├── app.rs               # AppState、导航和业务状态
│   ├── backend.rs           # PowerShell/服务/注册表后端
│   ├── i18n.rs              # 中英文翻译
│   ├── theme.rs             # 与前端无关的主题色
│   └── ui/
│       ├── main_window.rs   # GPUI 主窗口与页面
│       ├── components.rs    # 通用 GPUI 组件
│       └── input.rs         # 键盘输入规则
└── lib/service/             # Windows 服务 PowerShell 封装
```

## License

MIT
