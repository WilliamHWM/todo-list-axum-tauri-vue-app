//! 笔记处理器：CRUD + 按任务列出。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::application::{CreateNoteDto, UpdateNoteDto};
use crate::domain::Note;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

/// POST /api/notes
///
/// 创建笔记；内容不变量由领域实体校验。
pub async fn create_note(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody<CreateNoteDto>,
) -> ApiResult<Note> {
    let note = state.notes.create(payload).await?;
    Ok(ApiResponse::ok(note))
}

/// GET /api/tasks/:id/notes
///
/// 返回某任务的全部笔记，按创建时间升序。
pub async fn list_notes_by_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<Note>> {
    let notes = state.notes.list_by_task(&id).await?;
    Ok(ApiResponse::ok(notes))
}

/// GET /api/notes/:id
pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Note> {
    let note = state.notes.get(&id).await?;
    Ok(ApiResponse::ok(note))
}

/// PUT /api/notes/:id
pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateNoteDto>,
) -> ApiResult<Note> {
    let note = state.notes.update(&id, payload).await?;
    Ok(ApiResponse::ok(note))
}

/// DELETE /api/notes/:id
///
/// 成功返回 204；笔记不存在返回 404。
pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.notes.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
