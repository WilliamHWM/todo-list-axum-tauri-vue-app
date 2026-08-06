//! Task handlers: CRUD + filtered/paginated listing.

use super::super::error::ApiError;
use super::super::extract::ValidatedJson;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::db;
use crate::models::{CreateTaskRequest, Task, TaskListResult, TaskQuery, UpdateTaskRequest};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};

/// GET /api/tasks
///
/// Return tasks filtered/sorted/paginated by the query parameters.
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> ApiResult<TaskListResult> {
    let result = db::tasks::search_tasks(&state.db, query).await?;
    Ok(ApiResponse::ok(result))
}

/// POST /api/tasks
///
/// Create a new task. The title is validated by the `ValidatedJson` extractor
/// and additionally trimmed / checked for blank content here.
pub async fn create_task(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateTaskRequest>,
) -> ApiResult<Task> {
    let title = payload.title.trim().to_owned();
    if title.is_empty() {
        return Err(ApiError::bad_request("任务标题不能为空。"));
    }
    let task = db::tasks::create_task(&state.db, CreateTaskRequest { title }).await?;
    Ok(ApiResponse::ok(task))
}

/// PUT /api/tasks/:id
///
/// Partially update a task. At least one of `title` or `completed` must be
/// provided.
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(mut payload): ValidatedJson<UpdateTaskRequest>,
) -> ApiResult<Task> {
    if payload.title.is_none() && payload.completed.is_none() {
        return Err(ApiError::bad_request("至少提供一个需要更新的字段。"));
    }
    if let Some(title) = payload.title.as_mut() {
        *title = title.trim().to_owned();
        if title.is_empty() {
            return Err(ApiError::bad_request("任务标题不能为空。"));
        }
    }
    db::tasks::update_task(&state.db, &id, payload)
        .await?
        .map(ApiResponse::ok)
        .ok_or_else(|| ApiError::not_found("任务不存在或已被删除。"))
}

/// DELETE /api/tasks/:id
///
/// Permanently remove a task. Returns 204 on success, 404 if not found.
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::tasks::delete_task(&state.db, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("任务不存在或已被删除。"))
    }
}
