//! Uniform JSON response envelope.
//!
//! Every 2xx API response uses the `{ code, message, data }` shape so clients
//! can parse responses consistently (the frontend axios interceptor unwraps
//! `data` and surfaces `message` on failure).

use super::error::ApiError;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Standard success envelope: `{ code: 0, message: "ok", data: ... }`.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok".to_owned(),
            data,
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Alias used by handlers: either an `ApiResponse` envelope or an `ApiError`.
pub type ApiResult<T> = Result<ApiResponse<T>, ApiError>;
