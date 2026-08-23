//! 任务仓储的 sqlx 适配器：领域端口 → SQLite 具体实现。
//!
//! SQL 语句封装成接受任意 [`Executor`] 的 `pub(crate)` 助手函数（连接池 `&Pool`
//! 或事务 `&mut Transaction` 均可用），供 [`super::SqlxTaskRepository`]（普通路径）
//! 与工作单元里的事务仓储复用，保证两处 SQL 完全一致。

use super::Pool;
use crate::domain::{RepoError, Task, TaskList, TaskQuery, TaskRepository};
use sqlx::query_builder::QueryBuilder;
use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Row, Sqlite};

const SELECT_COLS: &str = "id, title, completed, category_id, created_at";

fn push_conditions(builder: &mut QueryBuilder<Sqlite>, q: &TaskQuery) {
    builder.push(" WHERE 1=1");
    if let Some(keyword) = q.keyword.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        builder.push(" AND title LIKE ");
        builder.push_bind(format!("%{keyword}%"));
    }
    if let Some(completed) = q.completed {
        builder.push(" AND completed = ");
        builder.push_bind(completed);
    }
}

fn sort_clause(sort: Option<&str>, sort_dir: Option<&str>) -> String {
    let column = match sort {
        Some("title") => "title",
        _ => "created_at",
    };
    let direction = match sort_dir {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    format!("ORDER BY {column} {direction}")
}

fn map_task(row: &SqliteRow) -> Result<Task, RepoError> {
    Ok(Task::rebuild(
        row.try_get("id")?,
        row.try_get("title")?,
        row.try_get("completed")?,
        row.try_get("category_id")?,
        row.try_get("created_at")?,
    ))
}

// ---------------------------------------------------------------------------
// Executor 助手：`E` 可以是 `&Pool`（独立语句）或 `&mut Transaction`（事务内）。
// ---------------------------------------------------------------------------

pub(crate) async fn find_task_by_id<'e, E>(
    executor: E,
    id: &str,
) -> Result<Option<Task>, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query(&format!("SELECT {SELECT_COLS} FROM tasks WHERE id = ?"))
        .bind(id)
        .fetch_optional(executor)
        .await?;
    row.as_ref().map(map_task).transpose()
}

pub(crate) async fn insert_task<'e, E>(executor: E, task: &Task) -> Result<(), RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO tasks (id, title, completed, category_id, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(task.id())
    .bind(task.title())
    .bind(task.completed())
    .bind(task.category_id())
    .bind(task.created_at())
    .execute(executor)
    .await?;
    Ok(())
}

pub(crate) async fn update_task<'e, E>(executor: E, task: &Task) -> Result<bool, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("UPDATE tasks SET title = ?, completed = ? WHERE id = ?")
        .bind(task.title())
        .bind(task.completed())
        .bind(task.id())
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub(crate) async fn delete_task<'e, E>(executor: E, id: &str) -> Result<bool, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// 仅更新任务的 `category_id` 一列（`None` 置空，关联外键 `SET NULL`）。
pub(crate) async fn assign_category<'e, E>(
    executor: E,
    task_id: &str,
    category_id: Option<String>,
) -> Result<bool, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("UPDATE tasks SET category_id = ? WHERE id = ?")
        .bind(category_id)
        .bind(task_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub(crate) async fn count_tasks<'e, E>(
    executor: E,
    query: &TaskQuery,
) -> Result<i64, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let mut count_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT COUNT(*) AS total FROM tasks");
    push_conditions(&mut count_builder, query);
    Ok(count_builder.build_query_scalar().fetch_one(executor).await?)
}

pub(crate) async fn list_tasks<'e, E>(
    executor: E,
    query: &TaskQuery,
) -> Result<Vec<Task>, RepoError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut list_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new(&format!("SELECT {SELECT_COLS} FROM tasks"));
    push_conditions(&mut list_builder, query);
    list_builder.push(" ");
    list_builder.push(sort_clause(query.sort.as_deref(), query.sort_dir.as_deref()));
    list_builder.push(" LIMIT ");
    list_builder.push_bind(limit);
    list_builder.push(" OFFSET ");
    list_builder.push_bind(offset);

    let rows = list_builder.build().fetch_all(executor).await?;
    rows.iter().map(map_task).collect()
}

/// 基于 SQLite 的 `TaskRepository` 实现（非事务路径）。
#[derive(Clone)]
pub struct SqlxTaskRepository {
    pool: Pool,
}

impl SqlxTaskRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TaskRepository for SqlxTaskRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError> {
        find_task_by_id(&self.pool, id).await
    }

    async fn insert(&self, task: &Task) -> Result<(), RepoError> {
        insert_task(&self.pool, task).await
    }

    async fn update(&self, task: &Task) -> Result<bool, RepoError> {
        update_task(&self.pool, task).await
    }

    async fn delete(&self, id: &str) -> Result<bool, RepoError> {
        delete_task(&self.pool, id).await
    }

    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError> {
        let total = count_tasks(&self.pool, query).await? as i32;
        let items = list_tasks(&self.pool, query).await?;
        Ok(TaskList { items, total })
    }

    async fn assign_category(
        &self,
        task_id: &str,
        category_id: Option<String>,
    ) -> Result<bool, RepoError> {
        assign_category(&self.pool, task_id, category_id).await
    }
}
