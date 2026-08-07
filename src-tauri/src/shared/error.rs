//! 应用级（非 HTTP）错误。
//!
//! 用于启动/基础设施失败，如配置加载、连接池创建、数据库迁移。HTTP 面向的错误
//! 在北向网关 [`crate::north::error`]。

/// 组合根 / 基础设施层使用的通用错误。
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
