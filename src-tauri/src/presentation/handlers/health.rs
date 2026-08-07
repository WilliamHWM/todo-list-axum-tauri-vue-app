//! 健康检查处理器。

use super::super::response::ApiResponse;

/// GET /api/health
///
/// 返回 `{ code: 0, message: "ok", data: "ok" }` 作为进程存活证明。
pub async fn health() -> ApiResponse<&'static str> {
    ApiResponse::ok("ok")
}
