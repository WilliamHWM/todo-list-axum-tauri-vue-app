//! Liveness / health-check endpoint.

use super::super::response::ApiResponse;

/// GET /api/health
///
/// Returns `{ code: 0, message: "ok", data: "ok" }` as long as the process
/// (and thus the embedded server) is alive.
pub async fn health() -> ApiResponse<&'static str> {
    ApiResponse::ok("ok")
}
