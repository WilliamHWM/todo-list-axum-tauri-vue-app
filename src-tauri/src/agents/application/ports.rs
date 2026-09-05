//! 应用层端口（北向 + 南向）。
//!
//! - [`AgentTeamUseCase`]：北向端口，北向网关（axum）只依赖它，不依赖具体编排实现。
//! - [`Llm`]：南向端口，应用层依赖的"大模型能力"，由 `south/llm` 适配器实现。
//! - [`LlmPrompt`]：调用 LLM 的统一入参（系统提示 + 用户提示 + 角色）。

use crate::agents::domain::artifact::{AgentCard, DevRun};
use crate::agents::domain::error::AgentError;
use crate::agents::domain::event::AgentEvent;
use crate::agents::domain::role::AgentRole;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

/// 北向端口：多 Agent 团队对外暴露的用例。
#[async_trait]
pub trait AgentTeamUseCase: Send + Sync {
    /// 给定一句需求，跑完"产品→架构→研发→测试→评审（可返工）"全流程，返回运行快照。
    /// 该运行**不会**持久化，适用于独立演示。
    async fn run_project(&self, requirement: &str) -> Result<DevRun, AgentError>;
    /// 让团队针对「某个已有任务」协作开发，并把运行记录持久化到该任务下。
    /// `requirement` 通常取自任务标题。
    async fn run_for_task(&self, task_id: &str, requirement: &str) -> Result<DevRun, AgentError>;
    /// 以 SSE 友好的方式跑独立项目：过程事件通过 `tx` 实时广播，流程结束即返回。
    async fn run_project_stream(
        &self,
        requirement: &str,
        tx: Sender<AgentEvent>,
    ) -> Result<(), AgentError>;
    /// 以 SSE 友好的方式针对任务协作：事件通过 `tx` 实时广播，并最终持久化运行记录。
    async fn run_for_task_stream(
        &self,
        task_id: &str,
        requirement: &str,
        tx: Sender<AgentEvent>,
    ) -> Result<(), AgentError>;
    /// 列出某任务下的全部运行记录（按时间倒序）。
    async fn list_runs(&self, task_id: &str) -> Result<Vec<DevRun>, AgentError>;
    /// 返回团队花名册（阵容）。
    async fn roster(&self) -> Vec<AgentCard>;
}

/// 南向端口：LLM 能力。
///
/// 应用层只认这个 trait；具体是 Mock 还是 OpenAI，由组合根在装配期注入。
#[async_trait]
pub trait Llm: Send + Sync {
    /// 给定提示词，返回模型生成文本。
    async fn complete(&self, prompt: &LlmPrompt) -> Result<String, AgentError>;
}

/// 调用 LLM 的统一入参。
#[derive(Debug, Clone)]
pub struct LlmPrompt {
    /// 系统提示（角色人设）。
    pub system: String,
    /// 用户提示（需求 + 前置产物上下文）。
    pub user: String,
    /// 发起请求的角色（便于适配器按角色分流或记录）。
    pub role: AgentRole,
}
