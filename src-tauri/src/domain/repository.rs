//! 仓储端口（trait）：领域层定义接口，基础设施层实现。
//!
//! 领域层不关心数据库，只声明"能存什么、能查什么"。应用层面向这些 trait 编程，
//! 测试时可注入内存实现；生产环境注入 sqlx 适配器。

use crate::domain::note::Note;
use crate::domain::task::Task;
use serde::{Deserialize, Serialize};

/// 任务列表查询条件（全部可选，仅出现的条件参与过滤/排序/分页）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub keyword: Option<String>,
    pub completed: Option<bool>,
    pub sort: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 分页后的任务列表。
#[derive(Debug, Clone, Serialize)]
pub struct TaskList {
    pub items: Vec<Task>,
    pub total: i64,
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
    /// 按 ID 查询单个任务，不存在返回 `Ok(None)`。
    async fn find_by_id(&self, id: &str) -> Result<Option<Task>, RepoError>;
    /// 插入一个新任务。
    async fn insert(&self, task: &Task) -> Result<(), RepoError>;
    /// 全量更新一个已存在任务，返回是否真的更新了行。
    async fn update(&self, task: &Task) -> Result<bool, RepoError>;
    /// 按 ID 删除任务，返回是否真的删除了行。
    async fn delete(&self, id: &str) -> Result<bool, RepoError>;
    /// 按条件过滤 + 排序 + 分页查询任务。
    async fn search(&self, query: &TaskQuery) -> Result<TaskList, RepoError>;
}

/// 笔记仓储端口。
#[async_trait::async_trait]
pub trait NoteRepository: Send + Sync {
    /// 按 ID 查询单个笔记，不存在返回 `Ok(None)`。
    async fn find_by_id(&self, id: &str) -> Result<Option<Note>, RepoError>;
    /// 插入一个新笔记。
    async fn insert(&self, note: &Note) -> Result<(), RepoError>;
    /// 全量更新一个已存在笔记，返回是否真的更新了行。
    async fn update(&self, note: &Note) -> Result<bool, RepoError>;
    /// 按 ID 删除笔记，返回是否真的删除了行。
    async fn delete(&self, id: &str) -> Result<bool, RepoError>;
    /// 返回某个任务的全部笔记（按创建时间升序）。
    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, RepoError>;
}
