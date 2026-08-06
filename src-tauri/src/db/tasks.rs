//! Task repository: all SQL statements for the `tasks` table.

use super::Pool;
use crate::models::{CreateTaskRequest, Task, TaskListResult, TaskQuery, UpdateTaskRequest};
use chrono::{SecondsFormat, Utc};
use sqlx::query_builder::QueryBuilder;
use sqlx::Sqlite;
use uuid::Uuid;

/// Append the optional WHERE conditions from `q` to a query builder.
///
/// Only conditions that are `Some` are pushed, and values always go through
/// `push_bind` (parameterized) so the final SQL cannot be injected.
fn push_task_conditions(builder: &mut QueryBuilder<Sqlite>, q: &TaskQuery) {
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

/// Sort column whitelist: maps a public sort key to an actual DB column.
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

/// Dynamic filtered + sorted + paginated task query built at runtime.
///
/// A single set of WHERE conditions is applied to both the COUNT query (for the
/// total) and the paged SELECT, so the numbers always agree.
pub async fn search_tasks(pool: &Pool, q: TaskQuery) -> Result<TaskListResult, sqlx::Error> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);

    let mut count_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT COUNT(*) AS total FROM tasks");
    push_task_conditions(&mut count_builder, &q);
    let total: i64 = count_builder.build_query_scalar().fetch_one(pool).await?;

    let mut list_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT id, title, completed, created_at FROM tasks");
    push_task_conditions(&mut list_builder, &q);
    list_builder.push(" ");
    list_builder.push(sort_clause(q.sort.as_deref(), q.sort_dir.as_deref()));
    list_builder.push(" LIMIT ");
    list_builder.push_bind(limit);
    list_builder.push(" OFFSET ");
    list_builder.push_bind(offset);

    let items = list_builder.build_query_as::<Task>().fetch_all(pool).await?;
    Ok(TaskListResult { items, total })
}

/// Insert a new task and return it with its generated ID and timestamp.
///
/// The timestamp is generated in Rust via `chrono::Utc::now()` and written in
/// RFC 3339 format (e.g. `2026-08-06T07:30:00.123456Z`) so the API always
/// transmits an unambiguous UTC instant. The `RETURNING` clause ensures the
/// Rust side reads back the exact value that was written.
pub async fn create_task(pool: &Pool, req: CreateTaskRequest) -> Result<Task, sqlx::Error> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    sqlx::query_as(
        "INSERT INTO tasks (id, title, created_at) VALUES (?, ?, ?) \
         RETURNING id, title, completed, created_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(req.title)
    .bind(now)
    .fetch_one(pool)
    .await
}

/// Partially update a task by ID.
///
/// Only fields that are `Some` in `req` are modified; `NULL` values are ignored
/// via `COALESCE`. Returns `Ok(None)` when no row with that ID exists.
pub async fn update_task(
    pool: &Pool,
    id: &str,
    req: UpdateTaskRequest,
) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE tasks \
         SET title = COALESCE(?, title), \
             completed = COALESCE(?, completed) \
         WHERE id = ? \
         RETURNING id, title, completed, created_at",
    )
    .bind(req.title)
    .bind(req.completed)
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Delete a task by ID.
///
/// Returns `true` if a row was actually removed, `false` otherwise.
pub async fn delete_task(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}
