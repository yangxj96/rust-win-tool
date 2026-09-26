# 剩余功能路线图（P2 系统工具 / P4 脚本深化）

> 状态：P0、服务模块、清理、平台化（单实例/日志/托盘）已完成并提交。
> 本文档记录**尚未实现**的部分，供后续按阶段实现、验证与提交。
> 约定：每个阶段完成后执行 `cargo fmt --check` / `clippy -D warnings` / `test` / `build --release`，
> 做一次启动截图验证，然后单独 `git commit`（Conventional Commits 中文，不自动 push）。

## 已完成（工作区干净，本地领先 origin/master，尚未推送）

| 提交 | 内容 |
|---|---|
| `de5755d` | P0：服务列表搜索/状态筛选、删除二次确认、失败保留上次状态、系统信息刷新、页头状态提示 |
| `99d0d30` | 垃圾清理页：用户/系统临时文件、缩略图缓存、Windows 更新缓存、回收站；默认进回收站 + 二次确认 |
| `a7f3612` | 服务详情 + 启动类型编辑 + 依赖/被依赖查看（原生 QueryServiceConfig/2、EnumDependentServices、ChangeServiceConfig） |
| `99f23c8` | 服务列表排序（默认/名称/状态）+ 会话操作历史面板 |
| `36a59ab` | 托管项导入/导出（原生 GetOpenFileNameW / GetSaveFileNameW） |
| `b30be78` | 单实例运行 + 本地日志（config/logs/rust-win-tool.log，设置页可打开目录） |
| `03e8a72` | 系统托盘（Shell_NotifyIcon + 隐藏消息窗口；左键显示、右键菜单 显示/隐藏/退出） |

## 待人工验证（自动化受 UIPI 限制，需提权窗口手动操作）

- 垃圾清理：扫描大小是否合理、清理后文件确实进入回收站、可还原
- 服务：启动/停止的“启动中→运行中”轮询、启动类型修改、导入/导出文件对话框
- 托盘：通知区域是否有图标；左键显示窗口；右键菜单三项是否生效

## 待实现：C（P2 系统工具）

现状：「系统工具」页只有「系统信息」。新增工具应沿用「工具列表 → 详情页」的现有模式（`tools_selected` + `tool_detail_active`），
后端一律走原生 Win32 API，避免 PowerShell 进程开销。

### C1 进程管理（建议先做）

- 目标：进程列表（名称 / PID / 内存 / CPU 占用），排序，结束进程（危险操作需二次确认）
- API：`EnumProcesses`、`OpenProcess`、`QueryFullProcessImageNameW`、`GetProcessMemoryInfo`（psapi）、`TerminateProcess`、
  `GetProcessTimes`（CPU 时间差）
- `windows-sys` features：`Win32_System_ProcessStatus`、`Win32_System_Threading`（已有）、`Win32_System_Memory`（如需要）
- 涉及文件：新增 `src/process.rs`；`src/backend.rs`（包装）；`src/app.rs`（工具选择/详情状态）；`src/ui/main_window.rs`（工具页扩展）；`src/i18n.rs`
- 验证：单测排序/格式化；启动截图；手动结束一个无害进程（如 notepad）确认
- 风险：结束系统进程可能导致系统不稳定 → 默认二次确认，禁止结束 PID 0/4 与关键进程

### C2 实时监控

- 目标：CPU / 内存 / 磁盘 / 网络 使用率的定时采样 + 迷你折线
- API：`GetSystemTimes`、`GlobalMemoryStatusEx`、`GetDiskFreeSpaceExW`、`GetIfTable2`（iphlpapi）
- 采样在后台线程，每 ~1s 更新 `AppState`；迷你折线可用自绘（沿用自定义元素/`canvas` 思路）
- `windows-sys` features：`Win32_System_SystemInformation`、`Win32_Storage_FileSystem`、`Win32_NetworkManagement_IpHelper`
- 涉及文件：新增 `src/monitor.rs`；`src/backend.rs`；`src/app.rs`；`src/ui/main_window.rs`；`src/i18n.rs`
- 风险：采样线程生命周期与视图切换需处理，避免泄漏/常驻唤醒

### C3 网络诊断

- 目标：ping、DNS 查询、TCP 端口连通性检测
- API：`IcmpSendEcho`（iphlpapi）、`GetAddrInfoW`（ws2_32）、`connect`/`WSAConnect`（ws2_32）
- `windows-sys` features：`Win32_NetworkManagement_IpHelper`、`Win32_Networking_WinSock`
- 涉及文件：新增 `src/network.rs`；`src/backend.rs`；`src/app.rs`；`src/ui/main_window.rs`；`src/i18n.rs`
- 风险：结果异步 + 超时处理；UI 只展示文本结果，不做复杂图形

### C4 开机启动项

- 目标：列出并禁用启动项
- 来源：注册表 Run 键（HKCU/HKLM，均 `winreg` 已有）+ 启动文件夹；计划任务（Task Scheduler COM）**优先级最低，可暂缓**
- 涉及文件：新增 `src/startup.rs`；`src/backend.rs`；`src/app.rs`；`src/ui/main_window.rs`；`src/i18n.rs`
- 风险：写注册表需确认；禁用而非删除，支持恢复

## 待实现：D（P4 脚本深化）

- 内置脚本库（多个清理/修复脚本，危险等级 + 二次确认），替换当前单一 Navicat 脚本
- 自定义脚本（保存到 `config/`，可编辑）
- 脚本执行输出面板（当前只有单行结果）
- 清理前备份（注册表导出 / 文件移动替代删除）
- 涉及文件：`src/app.rs`、`src/backend.rs`、`src/ui/main_window.rs`、`src/i18n.rs`，可能新增 `src/scripts.rs`

## 可选增强（未排期）

- 服务：按状态分组显示（当前为排序）、托盘内启停
- 清理：浏览器缓存分类、清理历史
- 应用：托盘通知、自动更新（用户已明确不做）

## 工程约定回顾

- 保持 `gpui = "=0.2.2"`，不引入第三方 UI 组件库
- 保持 `config/managed_services.json`、`config/settings.json` 路径/字段兼容
- 新增原生能力时优先扩展 `windows-sys` features，避免新增依赖
- 每阶段独立提交，便于回滚
