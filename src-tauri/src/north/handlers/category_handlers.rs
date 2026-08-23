//! 分类处理器：CRUD。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use crate::application::{CategoryUseCase, CreateCategoryDto, UpdateCategoryDto};
use crate::domain::Category;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

/// GET /api/categories
///
/// 列出全部分类（按创建时间升序）。
pub async fn list_categories(
    State(categories): State<Arc<dyn CategoryUseCase>>,
) -> ApiResult<Vec<Category>> {
    let categories = categories.list().await?;
    Ok(ApiResponse::ok(categories))
}

/// POST /api/categories
///
/// 创建分类；名称不变量由领域实体校验。
pub async fn create_category(
    State(categories): State<Arc<dyn CategoryUseCase>>,
    JsonBody(payload): JsonBody<CreateCategoryDto>,
) -> ApiResult<Category> {
    let category = categories.create(payload).await?;
    Ok(ApiResponse::ok(category))
}

/// GET /api/categories/:id
pub async fn get_category(
    State(categories): State<Arc<dyn CategoryUseCase>>,
    Path(id): Path<String>,
) -> ApiResult<Category> {
    let category = categories.get(&id).await?;
    Ok(ApiResponse::ok(category))
}

/// PUT /api/categories/:id
pub async fn update_category(
    State(categories): State<Arc<dyn CategoryUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCategoryDto>,
) -> ApiResult<Category> {
    let category = categories.update(&id, payload).await?;
    Ok(ApiResponse::ok(category))
}

/// DELETE /api/categories/:id
///
/// 成功返回 204；分类不存在返回 404。
pub async fn delete_category(
    State(categories): State<Arc<dyn CategoryUseCase>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    categories.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
