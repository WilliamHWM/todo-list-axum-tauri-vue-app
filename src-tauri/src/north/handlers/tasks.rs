//! 任务处理器：CRUD + 过滤/分页查询。
//!
//! 路由与实现同文件共存（co-location）：[`router`] 声明本资源全部端点，
//! 新增端点只改本文件，`routes.rs` 无需感知。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::application::{CreateTaskDto, CreateTaskWithNoteDto, SetTaskCategoryDto, UpdateTaskDto};
use crate::domain::{Task, TaskList, TaskQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use std::sync::Arc;

use crate::application::TaskUseCase;

/// 任务资源路由：`/api/tasks` 前缀下的全部端点。
///
/// `/:id/notes` 是笔记资源的嵌套视图——URL 归属任务、实现归属笔记模块，
/// 路由在此挂载。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/with-note", post(create_task_with_note))
        .route("/:id", put(update_task).delete(delete_task))
        .route("/:id/category", put(set_task_category))
        .route("/:id/notes", get(super::notes::list_notes_by_task))
}

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
