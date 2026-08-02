use crate::models::{CreateTaskRequest, Task, UpdateTaskRequest};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

pub async fn init_pool() -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:data.db?mode=rwc")
        .await?;
    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(&pool)
        .await?;
    Ok(pool)
}

pub async fn get_all_tasks(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, completed, created_at FROM tasks ORDER BY created_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn create_task(pool: &SqlitePool, req: CreateTaskRequest) -> Result<Task, sqlx::Error> {
    // 由数据库生成 created_at，并用 RETURNING 取回真实写入的数据，避免 Rust 和 SQLite 时间不一致。
    sqlx::query_as(
        "INSERT INTO tasks (id, title) VALUES (?, ?) RETURNING id, title, completed, created_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(req.title)
    .fetch_one(pool)
    .await
}

pub async fn update_task(
    pool: &SqlitePool,
    id: &str,
    req: UpdateTaskRequest,
) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as("UPDATE tasks SET title = COALESCE(?, title), completed = COALESCE(?, completed) WHERE id = ? RETURNING id, title, completed, created_at")
        .bind(req.title).bind(req.completed).bind(id).fetch_optional(pool).await
}

pub async fn delete_task(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}
