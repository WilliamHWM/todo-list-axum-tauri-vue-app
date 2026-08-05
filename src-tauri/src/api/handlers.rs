use super::{ApiError, ApiResult, AppState};
use crate::db;
use crate::models::{
    CreateNoteRequest, CreateTaskRequest, Note, Task, TaskListResult, TaskQuery,
    UpdateNoteRequest, UpdateTaskRequest,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

/// GET /api/health
///
/// Liveness probe — responds with "ok" as long as the process is running.
pub async fn health() -> &'static str {
    "ok"
}

/// GET /api/tasks
///
/// Return tasks filtered/sorted/paginated by the query parameters.
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> ApiResult<TaskListResult> {
    db::search_tasks(&state.db, query)
        .await
        .map(Json)
        .map_err(internal_error)
}

/// POST /api/tasks
///
/// Create a new task from the request body. The title is trimmed and validated
/// to be between 1 and 120 Unicode characters.
pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> ApiResult<Task> {
    let title = payload.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "任务标题必须是 1 到 120 个字符。".into(),
        ));
    }
    db::create_task(&state.db, CreateTaskRequest { title })
        .await
        .map(Json)
        .map_err(internal_error)
}

/// PUT /api/tasks/:id
///
/// Partially update a task. At least one of `title` or `completed` must be
/// provided.
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<UpdateTaskRequest>,
) -> ApiResult<Task> {
    if payload.title.is_none() && payload.completed.is_none() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "至少提供一个需要更新的字段。".into(),
        ));
    }
    if let Some(title) = &mut payload.title {
        *title = title.trim().to_owned();
        if title.is_empty() || title.chars().count() > 120 {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "任务标题必须是 1 到 120 个字符。".into(),
            ));
        }
    }
    db::update_task(&state.db, &id, payload)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "任务不存在或已被删除。".into(),
            )
        })
}

/// DELETE /api/tasks/:id
///
/// Permanently remove a task. Returns 204 on success, 404 if the task was not
/// found.
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::delete_task(&state.db, &id)
        .await
        .map_err(internal_error)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(
            StatusCode::NOT_FOUND,
            "任务不存在或已被删除。".into(),
        ))
    }
}

/// Map a SQLx error to a 500 Internal Server Error response.
///
/// Logs the original error with `tracing` so operators can inspect it while
/// the client receives a generic message.
fn internal_error(error: sqlx::Error) -> ApiError {
    tracing::error!(?error, "database request failed");
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "数据库操作失败，请稍后重试。".into(),
    )
}

/// Validate a note's content: must be non-blank and at most 5000 chars.
fn validate_note_content(content: &str) -> Result<String, ApiError> {
    let content = content.trim().to_owned();
    if content.is_empty() || content.chars().count() > 5000 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "笔记内容必须是 1 到 5000 个字符。".into(),
        ));
    }
    Ok(content)
}

/// POST /api/notes
pub async fn create_note(
    State(state): State<AppState>,
    Json(payload): Json<CreateNoteRequest>,
) -> ApiResult<Note> {
    let content = validate_note_content(&payload.content)?;
    db::create_note(&state.db, CreateNoteRequest { task_id: payload.task_id, content })
        .await
        .map(Json)
        .map_err(internal_error)
}

/// GET /api/tasks/:id/notes
pub async fn list_notes_by_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<Note>> {
    db::list_notes_by_task(&state.db, &id)
        .await
        .map(Json)
        .map_err(internal_error)
}

/// GET /api/notes/:id
pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Note> {
    db::get_note(&state.db, &id)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "笔记不存在或已被删除。".into()))
}

/// PUT /api/notes/:id
pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<UpdateNoteRequest>,
) -> ApiResult<Note> {
    if let Some(content) = &mut payload.content {
        *content = validate_note_content(content)?;
    }
    db::update_note(&state.db, &id, payload)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "笔记不存在或已被删除。".into()))
}

/// DELETE /api/notes/:id
pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::delete_note(&state.db, &id)
        .await
        .map_err(internal_error)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(StatusCode::NOT_FOUND, "笔记不存在或已被删除。".into()))
    }
}
