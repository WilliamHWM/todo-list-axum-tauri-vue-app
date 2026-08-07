//! 任务处理器：CRUD + 过滤/分页查询。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::application::{CreateTaskDto, UpdateTaskDto};
use crate::domain::{Task, TaskList, TaskQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

/// GET /api/tasks
///
/// 按关键字/完成状态过滤，支持排序与分页。
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> ApiResult<TaskList> {
    let result = state.tasks.list(query).await?;
    Ok(ApiResponse::ok(result))
}

/// POST /api/tasks
///
/// 创建任务；标题不变量由领域实体校验。
pub async fn create_task(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody<CreateTaskDto>,
) -> ApiResult<Task> {
    let task = state.tasks.create(payload).await?;
    Ok(ApiResponse::ok(task))
}

/// PUT /api/tasks/:id
///
/// 部分更新；至少提供一个字段，不变量由领域实体校验。
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskDto>,
) -> ApiResult<Task> {
    let task = state.tasks.update(&id, payload).await?;
    Ok(ApiResponse::ok(task))
}

/// DELETE /api/tasks/:id
///
/// 成功返回 204；任务不存在返回 404。
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.tasks.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
