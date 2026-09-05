//! 南向网关：LLM 适配器 + 运行记录持久化。
//!
//! 这里实现应用层定义的端口：
//! - [`Llm`](crate::agents::application::ports::Llm)：LLM 能力，默认 `MockLlm`。
//! - [`AgentRunRepository`](crate::agents::domain::repository::AgentRunRepository)：运行记录，
//!   由 `SqlxAgentRunRepository` 落到与任务同一库的 `agent_runs` 表。

pub mod agent_run_repository;
pub mod llm;
