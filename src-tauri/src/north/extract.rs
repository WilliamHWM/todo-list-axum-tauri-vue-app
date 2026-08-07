//! 自定义提取器。

use super::error::ApiError;
use axum::{
    extract::{FromRequest, Json, Request},
    http::StatusCode,
};
use serde::de::DeserializeOwned;

/// JSON 请求体提取器：解析失败统一映射为 400 + 错误信封。
///
/// 不再做声明式校验——业务不变量由领域实体在 `new`/`update` 时校验，由应用层
/// 服务抛出，经 [`super::error::ApiError::from`] 翻译。
pub struct JsonBody<T>(pub T);

#[async_trait::async_trait]
impl<S, T> FromRequest<S> for JsonBody<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            ApiError(StatusCode::BAD_REQUEST, format!("无效的请求体: {e}"))
        })?;
        Ok(Self(value))
    }
}
