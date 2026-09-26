# rust-win-tool

一个使用 Rust 和 GPUI 构建的 Windows 桌面系统管理工具，提供图形界面来管理常用系统项目并查看设备状态。

完整中文使用和开发文档见 [docs](docs/README.md)。

## 功能

- **服务管理**：添加和移除关注的 Windows 服务，查看状态并执行启动、停止和刷新操作。
- **系统信息与监控**：查看系统信息以及 CPU、内存、磁盘和网络使用情况。
- **系统工具**：查看进程和启动项；运行 DNS 查询、Ping 和 TCP 端口连通性检查。
- **垃圾文件清理**：扫描用户临时文件、Windows 临时文件、缩略图缓存和 Windows 更新下载缓存；可将文件移入回收站或永久删除。
- **脚本**：管理自定义脚本，并提供 Navicat 注册信息清理和 DNS 缓存清理脚本。
- **设置**：支持简体中文和英文界面、深色和浅色主题。
- **系统托盘**：支持隐藏到托盘、显示主窗口和退出应用。

涉及系统服务、启动项和系统目录的操作可能需要管理员权限。程序启动时会请求提升权限；清理功能会跳过无法访问或正在使用的文件。请在执行清理或脚本前确认操作内容。

## 平台与依赖

- Windows 10/11（使用 Windows 原生 API）
- Rust 稳定版工具链与 Cargo
- GPUI `0.2.2`

项目还使用 `serde`、`serde_json`、`thiserror`、`winreg` 和 `windows-sys`。Cargo 会根据 `Cargo.toml` 下载 Rust 依赖。

## 构建与运行

在 Windows PowerShell 中，从仓库根目录执行：

```powershell
cargo build
cargo run
```

构建发布版本：

```powershell
cargo build --release
```

## 配置和数据

应用在可执行文件所在目录的 `config/` 子目录保存数据：

- `managed_services.json`：关注的服务列表
- `settings.json`：界面语言和主题
- `scripts.json`：自定义脚本
- `logs/rust-win-tool.log`：运行日志

应用会在需要时创建配置目录和数据文件。需要保留用户数据时，请在迁移或删除应用目录前备份 `config/`。

## 项目结构

```text
src/                 应用状态、Windows 服务后端、系统工具与 GPUI 界面
src/ui/              主窗口和共享界面组件
assets/              应用资源
docs/                设计文档和开发计划
```

## 开发

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

欢迎通过 Issue 报告问题或提出建议，也欢迎提交 Pull Request。提交前请说明变更目的、影响范围和验证方式。

## 许可证

本项目基于 [MIT License](LICENSE) 发布。
