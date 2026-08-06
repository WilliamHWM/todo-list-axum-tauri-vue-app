//! Note handlers: CRUD + listing by task.

use super::super::error::ApiError;
use super::super::extract::ValidatedJson;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::db;
use crate::models::{CreateNoteRequest, Note, UpdateNoteRequest};
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

/// POST /api/notes
///
/// Create a new note. Content is validated by the extractor and trimmed here.
pub async fn create_note(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateNoteRequest>,
) -> ApiResult<Note> {
    let content = payload.content.trim().to_owned();
    if content.is_empty() {
        return Err(ApiError::bad_request("笔记内容不能为空。"));
    }
    let request = CreateNoteRequest {
        task_id: payload.task_id,
        content,
    };
    let note = db::notes::create_note(&state.db, request).await?;
    Ok(ApiResponse::ok(note))
}

/// GET /api/tasks/:id/notes
///
/// Return all notes belonging to a task, oldest first.
pub async fn list_notes_by_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Vec<Note>> {
    let notes = db::notes::list_notes_by_task(&state.db, &id).await?;
    Ok(ApiResponse::ok(notes))
}

/// GET /api/notes/:id
pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Note> {
    db::notes::get_note(&state.db, &id)
        .await?
        .map(ApiResponse::ok)
        .ok_or_else(|| ApiError::not_found("笔记不存在或已被删除。"))
}

/// PUT /api/notes/:id
pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(mut payload): ValidatedJson<UpdateNoteRequest>,
) -> ApiResult<Note> {
    if let Some(content) = payload.content.as_mut() {
        *content = content.trim().to_owned();
        if content.is_empty() {
            return Err(ApiError::bad_request("笔记内容不能为空。"));
        }
    }
    db::notes::update_note(&state.db, &id, payload)
        .await?
        .map(ApiResponse::ok)
        .ok_or_else(|| ApiError::not_found("笔记不存在或已被删除。"))
}

/// DELETE /api/notes/:id
pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::notes::delete_note(&state.db, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("笔记不存在或已被删除。"))
    }
}
