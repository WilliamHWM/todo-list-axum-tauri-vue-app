//! 笔记处理器：CRUD + 按任务列出。
//!
//! 路由与实现同文件共存（co-location）：[`router`] 声明本资源全部端点；
//! 「按任务列出」的 URL 在 `/api/tasks/:id/notes` 下，由 tasks 模块的路由挂载。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::application::{CreateNoteDto, NoteUseCase, UpdateNoteDto};
use crate::domain::Note;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// 笔记资源路由：`/api/notes` 前缀下的全部端点。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_note))
        .route(
            "/:id",
            get(get_note).put(update_note).delete(delete_note),
        )
}

/// POST /api/notes
///
/// 创建笔记；内容不变量由领域实体校验。
pub async fn create_note(
    State(notes): State<Arc<dyn NoteUseCase>>,
    JsonBody(payload): JsonBody<CreateNoteDto>,
) -> ApiResult<Note> {
    let note = notes.create(payload).await?;
    Ok(ApiResponse::ok(note))
}

/// GET /api/tasks/:id/notes
///
/// 返回某任务的全部笔记，按创建时间升序。
pub async fn list_notes_by_task(
    State(notes): State<Arc<dyn NoteUseCase>>,
    Path(id): Path<String>,
) -> ApiResult<Vec<Note>> {
    let notes = notes.list_by_task(&id).await?;
    Ok(ApiResponse::ok(notes))
}

/// GET /api/notes/:id
pub async fn get_note(
    State(notes): State<Arc<dyn NoteUseCase>>,
    Path(id): Path<String>,
) -> ApiResult<Note> {
    let note = notes.get(&id).await?;
    Ok(ApiResponse::ok(note))
}

/// PUT /api/notes/:id
pub async fn update_note(
    State(notes): State<Arc<dyn NoteUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateNoteDto>,
) -> ApiResult<Note> {
    let note = notes.update(&id, payload).await?;
    Ok(ApiResponse::ok(note))
}

/// DELETE /api/notes/:id
///
/// 成功返回 204；笔记不存在返回 404。
pub async fn delete_note(
    State(notes): State<Arc<dyn NoteUseCase>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    notes.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
