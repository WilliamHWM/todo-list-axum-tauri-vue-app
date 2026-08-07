//! HTTP 面向的错误：统一错误信封 + 服务错误 → HTTP 映射。
//!
//! 领域/应用层错误在此处翻译为带状态码的 `{ code, message }` JSON。

use crate::application::ServiceError;
use crate::domain::DomainError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// 所有非 2xx 响应的 JSON 形状。
#[derive(Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
}

/// 带 HTTP 状态码与人类可读消息的 API 错误。
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self(StatusCode::NOT_FOUND, message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = (self.0, self.1);
        tracing::warn!(
            status = status.as_u16(),
            message = %message,
            "api request failed"
        );
        (
            status,
            Json(ErrorBody {
                code: status.as_u16() as i32,
                message,
            }),
        )
            .into_response()
    }
}

/// 应用层错误 → API 错误。
///
/// 领域校验错误映射为 400，实体不存在映射为 404，仓储错误映射为 500。
impl From<ServiceError> for ApiError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::Domain(DomainError::TaskNotFound) => {
                Self::not_found("任务不存在或已被删除。")
            }
            ServiceError::Domain(DomainError::NoteNotFound) => {
                Self::not_found("笔记不存在或已被删除。")
            }
            ServiceError::Domain(e) => Self::bad_request(e.to_string()),
            ServiceError::Repository(e) => {
                tracing::error!(?e, "database request failed");
                Self::internal("数据库操作失败，请稍后重试。")
            }
        }
    }
}
