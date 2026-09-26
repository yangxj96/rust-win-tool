//! 应用模块门面与 GPUI 启动入口。

mod bootstrap;
mod state;

pub use state::*;

/// 启动应用并交由 GPUI 管理窗口生命周期。
pub fn run() {
    bootstrap::run();
}
