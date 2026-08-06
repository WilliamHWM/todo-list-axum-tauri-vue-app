//! Application-level (non-HTTP) error type.
//!
//! Used for startup / infrastructure failures such as configuration loading,
//! connection pool creation or running database migrations. HTTP-facing errors
//! live in [`crate::api::error`].

/// Errors that can occur outside of HTTP handler scope.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("配置错误: {0}")]
    Config(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("数据库迁移失败: {0}")]
    Migration(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("内部错误: {0}")]
    Internal(String),
}
