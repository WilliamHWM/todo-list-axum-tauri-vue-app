//! Note repository: all SQL statements for the `notes` table.

use super::Pool;
use crate::models::{CreateNoteRequest, Note, UpdateNoteRequest};
use chrono::{SecondsFormat, Utc};
use uuid::Uuid;

/// Insert a new note and return it with its generated ID and timestamp.
///
/// The timestamp is generated in Rust via `chrono::Utc::now()` in RFC 3339
/// format so the API transmits an unambiguous UTC instant.
pub async fn create_note(pool: &Pool, req: CreateNoteRequest) -> Result<Note, sqlx::Error> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    sqlx::query_as(
        "INSERT INTO notes (id, task_id, content, created_at) VALUES (?, ?, ?, ?) \
         RETURNING id, task_id, content, created_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(req.task_id)
    .bind(req.content)
    .bind(now)
    .fetch_one(pool)
    .await
}

/// Return all notes belonging to a task, oldest first.
pub async fn list_notes_by_task(pool: &Pool, task_id: &str) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, task_id, content, created_at FROM notes \
         WHERE task_id = ? ORDER BY created_at ASC",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await
}

/// Return a single note by ID, or `None` if it does not exist.
pub async fn get_note(pool: &Pool, id: &str) -> Result<Option<Note>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, task_id, content, created_at FROM notes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update a note (only fields that are `Some` are overwritten).
pub async fn update_note(
    pool: &Pool,
    id: &str,
    req: UpdateNoteRequest,
) -> Result<Option<Note>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE notes SET content = COALESCE(?, content) WHERE id = ? \
         RETURNING id, task_id, content, created_at",
    )
    .bind(req.content)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Delete a note by ID. Returns `true` if a row was removed.
pub async fn delete_note(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}
