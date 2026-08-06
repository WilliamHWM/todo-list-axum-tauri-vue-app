//! HTTP-facing error type shared by all handlers.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// JSON shape returned for every non-2xx response.
#[derive(Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
}

/// A structured API error carrying an HTTP status and a human-readable message.
///
/// Implements [`IntoResponse`] so handlers can return it directly.
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

/// Convenient conversion so repository errors bubble up as 500s automatically.
impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(?error, "database request failed");
        Self::internal("数据库操作失败，请稍后重试。")
    }
}
