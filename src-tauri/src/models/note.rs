use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// A single note attached to a task (or standalone).
#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub task_id: Option<String>,
    pub content: String,
    pub created_at: String,
}

/// Request body for creating a note.
#[derive(Debug, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub task_id: Option<String>,
    #[validate(length(
        min = 1,
        max = 5000,
        message = "笔记内容必须是 1 到 5000 个字符。"
    ))]
    pub content: String,
}

/// Request body for updating a note.
#[derive(Debug, Deserialize, Clone, Default, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    #[validate(length(
        min = 1,
        max = 5000,
        message = "笔记内容必须是 1 到 5000 个字符。"
    ))]
    pub content: Option<String>,
}
