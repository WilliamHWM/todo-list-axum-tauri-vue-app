//! 笔记仓储的 sqlx 适配器：领域端口 → SQLite 具体实现。

use super::Pool;
use crate::domain::{Note, NoteRepository, RepoError};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

/// 查询列清单，行转实体时复用。
const SELECT_COLS: &str = "id, task_id, content, created_at";

/// sqlx 行 → 领域实体（原始数据重建，跳过校验）。
fn map_note(row: &SqliteRow) -> Result<Note, RepoError> {
    Ok(Note::rebuild(
        row.try_get("id")?,
        row.try_get("task_id")?,
        row.try_get("content")?,
        row.try_get("created_at")?,
    ))
}

/// 基于 SQLite 的 `NoteRepository` 实现。
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
        let sql = format!("SELECT {SELECT_COLS} FROM notes WHERE id = ?");
        let row = sqlx::query(&sql).bind(id).fetch_optional(&self.pool).await?;
        row.as_ref().map(map_note).transpose()
    }

    async fn insert(&self, note: &Note) -> Result<(), RepoError> {
        sqlx::query(
            "INSERT INTO notes (id, task_id, content, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(note.id())
        .bind(note.task_id())
        .bind(note.content())
        .bind(note.created_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, note: &Note) -> Result<bool, RepoError> {
        let result = sqlx::query("UPDATE notes SET content = ? WHERE id = ?")
            .bind(note.content())
            .bind(note.id())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        let result = sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError> {
        let sql = format!("SELECT {SELECT_COLS} FROM notes WHERE task_id = ? ORDER BY created_at ASC");
        let rows = sqlx::query(&sql).bind(task_id).fetch_all(&self.pool).await?;
        rows.iter().map(map_note).collect()
    }
}
