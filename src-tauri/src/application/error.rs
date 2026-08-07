//! 应用层错误：同时容纳领域错误与仓储错误。
//!
//! 领域错误（`DomainError`）表达不变量被违反；仓储错误（`RepoError`）表达数据
//! 访问失败。二者统一为 `ServiceError` 后由表现层翻译成 HTTP 响应。

use crate::domain::{DomainError, RepoError};
use thiserror::Error;

/// 用例执行可能产生的错误。
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(#[from] RepoError),
}
