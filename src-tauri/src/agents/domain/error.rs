//! 领域错误。
//!
//! 只描述"协作流程/LLM 端口层面"的失败，不携带 HTTP 概念；由北向网关翻译成
//! HTTP 响应。

use thiserror::Error;

/// 多 Agent 协作子系统可能产生的错误。
#[derive(Debug, Error)]
pub enum AgentError {
    /// LLM 调用失败（网络/解析/厂商返回异常）。
    #[error("LLM 调用失败: {0}")]
    Llm(String),
    /// 流程内部不一致（如产物缺失）。
    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for AgentError {
    fn from(error: serde_json::Error) -> Self {
        AgentError::Internal(error.to_string())
    }
}
