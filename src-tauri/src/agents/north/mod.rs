//! 多 Agent 协作子系统 —— 北向网关（HTTP 入口）。
//!
//! 只做翻译：HTTP → 北向端口 [`AgentTeamUseCase`] → 统一信封。复用主工程的
//! `crate::north` 响应/错误/提取器，保持前后端一致。

pub mod handlers;
