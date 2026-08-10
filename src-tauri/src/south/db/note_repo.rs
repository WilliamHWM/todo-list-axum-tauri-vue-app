//! 笔记仓储的 sqlx 适配器：领域端口 → SQLite 具体实现。
//!
//! 与 [`super::task_repo`] 相同的约定：SQL 封装成接受任意 [`Executor`] 的
//! `pub(crate)` 助手函数，供普通仓储与工作单元的事务仓储复用。

use super::Pool;
use crate::domain::{Note, NoteRepository, RepoError};
use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Row, Sqlite};

const SELECT_COLS: &str = "id, task_id, content, created_at";

fn map_note(row: &SqliteRow) -> Result<Note, RepoError> {
    Ok(Note::rebuild(
        row.try_get("id")?,
        row.try_get("task_id")?,
        row.try_get("content")?,
        row.try_get("created_at")?,
    ))
}

// ---------------------------------------------------------------------------
// Executor 助手
// ---------------------------------------------------------------------------

pub(crate) async fn find_note_by_id<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<Note>, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(&format!("SELECT {SELECT_COLS} FROM notes WHERE id = ?"))
        .bind(id)
        .fetch_optional(executor)
        .await?;
    row.as_ref().map(map_note).transpose()
}

pub(crate) async fn insert_note<'e, E>(executor: E, note: &Note) -> Result<(), RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO notes (id, task_id, content, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(note.id())
    .bind(note.task_id())
    .bind(note.content())
    .bind(note.created_at())
    .execute(executor)
    .await?;
    Ok(())
}

pub(crate) async fn update_note<'e, E>(executor: E, note: &Note) -> Result<bool, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("UPDATE notes SET content = ? WHERE id = ?")
        .bind(note.content())
        .bind(note.id())
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub(crate) async fn delete_note<'e, E>(executor: E, id: &str) -> Result<bool, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub(crate) async fn list_notes_by_task<'e, E>(
    executor: E,
    task_id: &str,
) -> Result<Vec<Note>, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {SELECT_COLS} FROM notes WHERE task_id = ? ORDER BY created_at ASC");
    let rows = sqlx::query(&sql).bind(task_id).fetch_all(executor).await?;
    rows.iter().map(map_note).collect()
}

/// 基于 SQLite 的 `NoteRepository` 实现（非事务路径）。
#[derive(Clone)]
pub struct SqlxNoteRepository {
    pool: Pool,
}

impl SqlxNoteRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl NoteRepository for SqlxNoteRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError> {
        find_note_by_id(&self.pool, id).await
    }

    async fn insert(&self, note: &Note) -> Result<(), RepoError> {
        insert_note(&self.pool, note).await
    }

    async fn update(&self, note: &Note) -> Result<bool, RepoError> {
        update_note(&self.pool, note).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        delete_note(&self.pool, id).await
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError> {
        list_notes_by_task(&self.pool, task_id).await
    }
}
