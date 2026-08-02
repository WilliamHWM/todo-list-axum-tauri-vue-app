use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库存储结构，同时也是 API 返回给前端的数据结构。
/// `rename_all` 让 JSON 使用前端惯用的 camelCase，而数据库字段仍保持 snake_case。
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub completed: Option<bool>,
}
