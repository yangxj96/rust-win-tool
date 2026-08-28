# GPUI Desktop Refactor Design

## Goal

将当前基于 ratatui/crossterm 的 Windows 控制台应用直接重构为 GPUI 桌面应用，保留现有已经实现的服务管理、系统信息、Navicat 注册表清理、语言切换、主题切换和配置兼容能力。

本次重构不保留 TUI 前端、不提供双前端模式，也不扩展当前尚未实现的磁盘清理、网络诊断、进程管理、注册表编辑器和事件查看器功能。

## Current Baseline

- 根 crate `rust-win-tool` 是 Windows 二进制应用，业务服务能力位于本地 crate `lib/service`。
- `src/app.rs` 当前同时承载应用状态、配置读写、PowerShell 操作调度、系统信息读取和 UI 导航状态。
- `src/ui/` 下有服务、工具、脚本、设置四个 ratatui 视图。
- `managed_services.json` 和 `settings.json` 保存在 exe 同级的 `config/` 目录中。
- 基线验证已通过：`cargo test` 的 9 个测试通过，`cargo clippy -- -D warnings` 通过。

## Scope and Compatibility

### Retained behavior

1. 服务管理
   - 读取并保存受管理的 Windows 服务列表。
   - 从 Windows 服务列表中搜索并添加服务。
   - 查看服务名称、显示名称、状态和错误消息。
   - 启动/停止选中服务。
   - 启动/停止全部受管理服务。
   - 刷新全部服务状态。
   - 删除受管理服务。
2. 工具
   - 展示现有六项工具条目。
   - 仅“系统信息”进入已实现详情页。
   - 其他五项继续展示，但不执行未实现操作。
3. 脚本
   - 执行 Navicat 注册信息和 CLSID 残留清理。
   - 展示成功或失败结果。
4. 设置
   - 中英文切换。
   - 深色/浅色主题切换。
   - 继续保存设置并在下次启动时恢复。
5. 操作方式
   - 保留现有快捷键：页面切换、列表选择、服务操作、Enter 确认、Esc 返回/关闭和 `q` 退出。
   - 增加桌面 GUI 所需的鼠标点击、按钮交互和可见焦点。

### Explicit non-goals

- 不实现当前尚未实现的五个系统工具。
- 不改变现有 JSON 字段名、枚举序列化值和配置目录位置。
- 不引入 TUI fallback 或 `tui`/`gpui` feature 双模式。
- 不引入 `gpui-component` 等第三方组件库作为第一阶段依赖。

## Architecture

### Runtime

使用 `gpui = "=0.2.2"`，通过 GPUI 0.2.2 的 `Application::new()` 启动应用和 Windows 平台运行时。依赖使用精确版本，避免 pre-1.0 GPUI 的兼容性漂移。

应用运行结构为：

```text
gpui::Application
    └── Window
        └── MainWindow Entity
            ├── AppState
            ├── current view
            ├── modal state
            └── background task handles
```

`MainWindow` 实现 GPUI `Render`，负责组装窗口级布局和页面组件。项目状态重命名为 `AppState`，避免与 GPUI 自身的 `App` 上下文冲突。

### Module boundaries

```text
src/
├── main.rs                  # GPUI Application、WindowOptions、根 Entity
├── app.rs                   # AppState、页面状态、选择状态、设置状态
├── backend.rs               # 外部系统操作和结果解析
├── i18n.rs                  # Language、Translations、ZH、EN
├── theme.rs                # 与 GPUI 无关的主题 token 和颜色值
└── ui/
    ├── mod.rs               # UI 模块声明
    ├── main_window.rs       # MainWindow、窗口壳层、导航、全局操作
    ├── components.rs        # 按钮、状态徽章、通用卡片和提示组件
    ├── service.rs           # 服务列表、服务工具栏、添加服务弹层
    ├── tools.rs             # 工具列表、系统信息详情
    ├── scripts.rs           # 脚本列表和执行结果
    └── settings.rs          # 设置列表
```

`lib/service` 继续提供 Windows 服务的同步底层 API。`backend.rs` 包装服务 crate、PowerShell、文件系统和 `winreg` 操作，使 UI 只依赖明确的后台操作结果。

### State ownership

`MainWindow` Entity 直接持有 `AppState`，页面组件只读取状态并通过事件回调请求 `MainWindow` 更新。页面组件不得直接调用 PowerShell、注册表或服务 crate。

`AppState` 保留以下状态类别：

- 当前页面和各页面选中索引。
- 受管理服务及其状态、错误消息。
- 添加服务弹层的搜索字符串、过滤结果和选中项。
- 当前语言和主题。
- 系统信息加载状态及结果。
- 脚本执行状态及结果。
- 当前模态弹层和全局错误提示。

服务状态和异步操作状态使用明确的枚举或结构表示，不再依赖把“启动中”“刷新中”等本地化文字写入服务状态 map。状态文字在 Render 阶段根据当前语言生成。

## Backend and Async Data Flow

所有外部系统操作遵循以下流程：

```text
GPUI click/key event
        ↓
MainWindow starts operation
        ↓
cx.background_spawn(sync backend operation)
        ↓
PowerShell / winreg / app-service / config IO
        ↓
typed operation result
        ↓
MainWindow Entity update on GPUI foreground context
        ↓
cx.notify()
        ↓
Render updated state
```

### Operations

后台操作至少覆盖：

- 初始服务状态刷新。
- 服务列表枚举，用于添加服务弹层。
- 单个服务启动和停止。
- 全部服务启动和停止。
- 全部服务状态刷新。
- 系统信息 PowerShell 查询和 JSON 解析。
- Navicat 注册表清理。

渲染函数中不得执行这些操作。操作进行期间，相关按钮显示忙碌状态并避免重复提交；窗口仍然可以切换页面和响应关闭操作。

配置文件很小，启动时读取可以同步完成；设置和受管理服务列表写入沿用现有格式，并将读写错误转为可见的应用状态。

后台任务通过 `WeakEntity<MainWindow>` 更新 UI，窗口关闭后任务不会继续持有根 Entity。任务句柄由 `MainWindow` 保存，以便在必要时取消正在运行的操作。

## UI Design

### Window shell

- 初始窗口约为 `1100 × 720`。
- 最小窗口尺寸约为 `900 × 600`。
- 使用 GPUI 默认 Windows 标题栏和系统窗口按钮。
- 内容区域采用深色/浅色主题 token。
- 顶部显示应用图标、应用标题和四个页面标签。
- 页面标签支持鼠标点击和左右方向键/h/l 切换。

### Service page

- 顶部显示页面标题、受管理服务数量和当前选中位置。
- 工具栏提供添加、删除、启动、停止、启动全部、停止全部和刷新操作。
- 主体使用可滚动表格行显示服务名称、显示名称、状态和错误消息。
- 单击行更新选中服务；上下方向键/k/j 保持原有选择行为。
- 删除通过确认弹层执行，确认后保存 JSON 并修正选中索引。
- 添加服务弹层包含搜索输入、可滚动结果列表、状态和确认/取消操作。

### Tools, scripts and settings pages

- 工具页保留六项条目，仅系统信息条目启用详情操作。
- 系统信息页展示操作系统、版本、构建号、计算机名、用户名、CPU、核心数和内存；Esc 返回工具列表。
- 脚本页展示 Navicat 清理脚本、执行按钮和结果提示。
- 设置页展示语言和主题两项可点击设置，Enter 和鼠标点击都可切换。

### Input and focus

添加服务的搜索框按 GPUI 官方 input 示例实现最小输入 Entity，支持字符输入、退格、光标、焦点和中文搜索；不依赖第三方组件库。

页面控件提供可见 hover/active/focus 状态。全局快捷键在输入框获得焦点时不应拦截普通字符输入；Esc 在弹层、详情页和普通页面按层级关闭或返回。

## Error Handling

错误分为三类：

1. 外部操作错误：服务不存在、权限不足、服务状态冲突、超时、PowerShell 失败和注册表访问失败。
2. 数据错误：JSON 读取失败、JSON 结构不符合预期、系统信息字段缺失或类型错误。
3. 持久化错误：配置目录创建失败、JSON 写入失败。

错误处理要求：

- 保留现有中英文错误映射。
- 服务操作错误显示在对应行。
- 系统信息失败显示失败状态和重试入口，不停留在无限加载状态。
- 服务枚举失败显示弹层错误和重试入口。
- 脚本失败显示脚本结果错误。
- 配置写入失败不丢弃内存状态，并显示底部提示。
- 系统信息 `Cores` 使用整数解析，`RAM` 使用浮点数解析，非法值才显示 `N/A`。

## Persistence

继续使用 exe 同级的 `config/` 目录：

```text
<exe directory>/config/managed_services.json
<exe directory>/config/settings.json
```

现有服务和设置 JSON 不迁移、不改字段、不改序列化值。直接重构只替换 UI 和状态更新机制，不改变已有用户数据的读取路径。

## Dependencies and Windows Build

### Dependency changes

保留：

- `serde`
- `serde_json`
- `winreg`
- `app-service`
- `winresource`

新增：

- `gpui = "=0.2.2"`

移除：

- `ratatui`
- `crossterm`
- `unicode-width`

第一阶段不新增 `gpui_platform`，优先使用 GPUI 0.2.2 已发布包中的 `Application::new()` 和 Windows 平台实现，减少平台 companion crate 的版本配对问题。

### Build resources

- 保留 `assets/app.ico` 和现有 Windows 资源信息。
- `build.rs` 将 Subsystem 从 `console` 改为 `windows`。
- 保留应用名称和文件描述信息。
- GPUI 的 Windows manifest feature 若由默认 feature 提供则保持启用；资源重复或链接冲突时以 GPUI 0.2.2 的构建要求为准调整 `build.rs`，不能恢复为 console 子系统。

## Testing and Verification

### Unit tests

保留并迁移现有测试：

- View 页面索引往返和边界。
- 页面选择循环。
- 主题和语言循环。
- 添加服务弹层默认状态。
- 设置文件读写往返。

新增测试：

- 服务搜索按名称和显示名称过滤。
- 空过滤结果和选中索引边界。
- 系统信息 JSON 中整数核心数和浮点内存解析。
- 系统信息字段缺失时的 `N/A` fallback。
- 服务操作结果更新成功状态和错误消息。
- 任务完成后状态可触发重新渲染。

### GPUI smoke tests

使用 GPUI 0.2.2 的 headless/test-support 能力验证：

- 根 Entity 可以创建。
- 主窗口根视图可以渲染。
- 页面切换更新当前页面。
- 设置切换更新状态。
- 添加服务弹层可以打开、过滤和关闭。

### Commands

完成重构后必须执行：

```text
cargo test
cargo clippy -- -D warnings
cargo build --release
```

Windows 手工验证至少覆盖：启动窗口、窗口关闭、四个页面切换、添加服务搜索、单个服务启停、全部服务启停、刷新、删除确认、系统信息、Navicat 清理、语言切换、主题切换和已有 JSON 配置恢复。

## Direct Refactor Sequence

1. 更新依赖和 Windows 子系统配置，确认 GPUI 0.2.2 在当前 Windows 工具链可构建。
2. 将 `App` 重命名为 `AppState`，移除 ratatui 类型并整理状态接口。
3. 新增 `backend.rs`，把同步外部操作和系统信息解析集中起来。
4. 用 GPUI `Application`、`WindowOptions` 和 `MainWindow` 替换 ratatui 主循环。
5. 重写共享组件、服务页、工具页、脚本页和设置页。
6. 接入 Entity 事件回调、后台任务、错误状态和模态弹层。
7. 删除 ratatui/crossterm 渲染代码和无用依赖。
8. 迁移/补充测试，运行完整验证命令并手工检查 Windows GUI 行为。
9. 更新 `README.md` 和 `AGENTS.md`，准确描述 GPUI 架构、页面和快捷键。

## Acceptance Criteria

- 程序以 Windows GUI 窗口启动，不再打开控制台 TUI。
- 现有已实现功能均可通过鼠标和原快捷键使用。
- 启动、服务操作、系统信息和脚本执行期间窗口不冻结。
- 旧版 `config/` 下的两个 JSON 文件可以被新版读取。
- 系统信息核心数和内存可以正确显示数值。
- 未实现的五项工具不会执行错误的系统信息操作。
- `cargo test`、`cargo clippy -- -D warnings` 和 `cargo build --release` 全部通过。
- 文档与实际实现一致。
