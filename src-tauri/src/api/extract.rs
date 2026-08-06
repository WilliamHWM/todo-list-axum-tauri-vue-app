//! Custom extractors used by handlers.

use super::error::ApiError;
use axum::{
    extract::{FromRequest, Json, Request},
    http::StatusCode,
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// JSON body extractor that also runs declarative validation rules.
///
/// The request DTOs derive `Validate` (via the `validator` crate); any rule
/// violation is turned into a 400 response before the handler runs.
pub struct ValidatedJson<T>(pub T);

#[async_trait::async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            ApiError(StatusCode::BAD_REQUEST, format!("无效的请求体: {e}"))
        })?;
        value
            .validate()
            .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
        Ok(Self(value))
    }
}
