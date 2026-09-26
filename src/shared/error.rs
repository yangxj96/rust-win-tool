//! 后台功能统一返回的错误类型。

/// 阻塞式后端操作返回的错误类型。
///
/// 服务错误保留稳定的原始关键词，`AppState` 会据此映射为本地化界面提示。
#[derive(Debug, Clone, thiserror::Error)]
pub enum BackendError {
    #[error("service operation failed: {0}")]
    Service(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("response parse failed: {0}")]
    Parse(String),
    #[error("registry operation failed: {0}")]
    Registry(String),
    #[cfg(not(windows))]
    #[error("this operation is only supported on Windows")]
    UnsupportedPlatform,
}
