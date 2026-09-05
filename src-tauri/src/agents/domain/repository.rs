//! 智能体运行记录的仓储端口（南向端口）。
//!
//! 这是 agents 子系统**自有**的聚合：`DevRun` 作为某次协作的快照被持久化，并通过
//! `task_id` 与任务模块关联——但只持有任务 id 字符串，不依赖任务领域的任何类型。
//! 因此两个模块保持解耦：任务模块完全不知道智能体的存在；关联是"外键引用"而非
//! "业务耦合"。
//!
//! SQL 只允许出现在 `south` 适配器里（见 `super::super::south`）。

use crate::agents::domain::artifact::DevRun;
use crate::agents::domain::error::AgentError;
use async_trait::async_trait;

/// 智能体运行记录的仓储端口。
#[async_trait]
pub trait AgentRunRepository: Send + Sync {
    /// 持久化一次运行并与某任务关联（`task_id` 为外部引用，不要求任务一定存在）。
    async fn save(&self, task_id: &str, run: &DevRun) -> Result<(), AgentError>;
    /// 列出某任务的全部运行（按开始时间倒序）。
    async fn list_by_task(&self, task_id: &str) -> Result<Vec<DevRun>, AgentError>;
}
