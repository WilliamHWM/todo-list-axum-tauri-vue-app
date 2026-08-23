//! 任务处理器：CRUD + 过滤/分页查询。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use crate::application::{CreateTaskDto, CreateTaskWithNoteDto, SetTaskCategoryDto, UpdateTaskDto};
use crate::domain::{Task, TaskList, TaskQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::application::TaskUseCase;

/// GET /api/tasks
///
/// 按关键字/完成状态过滤，支持排序与分页。
pub async fn list_tasks(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Query(query): Query<TaskQuery>,
) -> ApiResult<TaskList> {
    let result = tasks.list(query).await?;
    Ok(ApiResponse::ok(result))
}

/// POST /api/tasks
///
/// 创建任务；标题不变量由领域实体校验。
pub async fn create_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    JsonBody(payload): JsonBody<CreateTaskDto>,
) -> ApiResult<Task> {
    let task = tasks.create(payload).await?;
    Ok(ApiResponse::ok(task))
}

/// POST /api/tasks/with-note
///
/// 在同一数据库事务里创建任务并附带首条笔记：任务或笔记任一步失败都会整体回滚。
pub async fn create_task_with_note(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    JsonBody(payload): JsonBody<CreateTaskWithNoteDto>,
) -> ApiResult<Task> {
    let task = tasks.create_task_with_note(payload).await?;
    Ok(ApiResponse::ok(task))
}

/// PUT /api/tasks/:id
///
/// 部分更新；至少提供一个字段，不变量由领域实体校验。
pub async fn update_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskDto>,
) -> ApiResult<Task> {
    let task = tasks.update(&id, payload).await?;
    Ok(ApiResponse::ok(task))
}

/// DELETE /api/tasks/:id
///
/// 成功返回 204；任务不存在返回 404。
pub async fn delete_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tasks.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/tasks/:id/category
///
/// 设置任务所属分类；`categoryId` 为 `null` 时清除归属。
pub async fn set_task_category(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<SetTaskCategoryDto>,
) -> ApiResult<Task> {
    let task = tasks.set_category(&id, payload.category_id).await?;
    Ok(ApiResponse::ok(task))
}
