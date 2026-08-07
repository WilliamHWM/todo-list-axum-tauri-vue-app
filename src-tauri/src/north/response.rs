//! 统一成功响应信封。

use super::error::ApiError;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// 标准成功信封：`{ code: 0, message: "ok", data: ... }`。
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

/// 处理器返回类型：成功信封或错误。
pub type ApiResult<T> = Result<ApiResponse<T>, ApiError>;
