//! 健康检查处理器。

use super::super::response::ApiResponse;
use super::super::AppState;
use axum::{routing::get, Router};

/// 健康检查路由：`/api/health`。
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(health))
}

/// GET /api/health
///
/// 返回 `{ code: 0, message: "ok", data: "ok" }` 作为进程存活证明。
pub async fn health() -> ApiResponse<&'static str> {
    ApiResponse::ok("ok")
}
