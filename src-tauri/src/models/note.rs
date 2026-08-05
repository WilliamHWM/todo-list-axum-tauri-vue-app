use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub task_id: Option<String>,
    pub content: String,
}

/// Request body for updating a note.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    pub content: Option<String>,
}
