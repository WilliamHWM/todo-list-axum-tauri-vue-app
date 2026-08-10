//! 仓储端口（trait）：领域层定义接口，基础设施层实现。
//!
//! 领域层不关心数据库，只声明"能存什么、能查什么"。应用层面向这些 trait 编程，
//! 测试时可注入内存实现；生产环境注入 sqlx 适配器。
//!
//! ## 接口设计：`&self` 而非 `&mut self`
//!
//! 使用 `&self` 而非 `&mut self`，原因：
//! - `Arc<dyn TaskRepository>` 只能通过 `&self` 借用（`Arc::as_ref()` 返回 `&T`）
//! - 应用层 `TaskService` 持有 `&self` 的 `Arc<dyn TaskRepository>`，方法签名需匹配
//! - 事务仓储的内部实现通过 `UnsafeCell` 提供 `&mut Transaction`（内部可变性）

use crate::domain::note::Note;
use crate::domain::task::Task;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/// 任务列表查询条件（全部可选，仅出现的条件参与过滤/排序/分页）。
#[typeshare]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub keyword: Option<String>,
    pub completed: Option<bool>,
    pub sort: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// 分页后的任务列表。
#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub items: Vec<Task>,
    pub total: i32,
}

/// 仓储访问错误（对 sqlx 错误的透明封装，领域层无需感知具体实现）。
#[derive(Debug, thiserror::Error)]
#[error("数据访问失败: {0}")]
pub struct RepoError(pub String);

impl RepoError {
    pub fn wrap(source: impl std::fmt::Display) -> Self {
        Self(source.to_string())
    }
}

/// 任务仓储端口。
#[async_trait::async_trait]
pub trait TaskRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError>;
    async fn insert(&self, task: &Task) -> Result<(), RepoError>;
    async fn update(&self, task: &Task) -> Result<bool, RepoError>;
    async fn delete(&self, id: &str) -> Result<bool, RepoError>;
    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError>;
}

/// 笔记仓储端口。
#[async_trait::async_trait]
pub trait NoteRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError>;
    async fn insert(&self, note: &Note) -> Result<(), RepoError>;
    async fn update(&self, note: &Note) -> Result<bool, RepoError>;
    async fn delete(&self, id: &str) -> Result<bool, RepoError>;
    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError>;
}
