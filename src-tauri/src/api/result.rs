use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// A structured error response returned to the client as JSON.
///
/// All public-facing errors are wrapped in this type via the `ApiError` helper,
/// which also carries an HTTP status code.
#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

/// Combines an HTTP status code with an error message for use in route handlers.
///
/// Implements `IntoResponse` so it can be returned directly from an handler.
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { message: self.1 })).into_response()
    }
}

/// Convenience type alias for handlers: a successful JSON response or an error.
pub type ApiResult<T> = Result<Json<T>, ApiError>;
