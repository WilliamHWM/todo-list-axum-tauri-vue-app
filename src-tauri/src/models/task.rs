use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single todo item persisted in the database.
///
/// `rename_all = "camelCase"` converts the snake_case DB columns into the
/// camelCase JSON keys the Vue frontend expects.
#[derive(Debug, Serialize, FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}

/// Request body for creating a new task.
///
/// Only the `title` field is required; the server assigns the ID and timestamp.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
}

/// Request body for updating an existing task.
///
/// Both fields are optional — the server uses `COALESCE` so only supplied
/// fields are overwritten.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

/// Query parameters for `GET /api/tasks`.
///
/// Every field is optional; only the ones present become SQL conditions.
/// `sort`/`sortDir` are mapped to a column whitelist to avoid injection.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub keyword: Option<String>,
    pub completed: Option<bool>,
    pub sort: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Paginated result envelope returned by `GET /api/tasks`.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResult {
    pub items: Vec<Task>,
    pub total: i64,
}
