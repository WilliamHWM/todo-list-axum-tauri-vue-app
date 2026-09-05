//! OpenAI LLM 适配器。
//!
//! 运行时读取 `OPENAI_API_KEY` 与 `OPENAI_MODEL` 环境变量。
//! 通过 [`Llm`] 端口实现，替换 Mock 时无需改动编排器或表现层。
//! 切换方式：设 `AGENT_LLM=openai` 即可。

use crate::agents::application::ports::{Llm, LlmPrompt};
use crate::agents::domain::error::AgentError;
use async_trait::async_trait;

/// 基于 OpenAI 兼容 Chat Completions 接口的 LLM 实现。
pub struct OpenAiLlm {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiLlm {
    /// 从环境变量构造：`OPENAI_API_KEY`（必填）、`OPENAI_MODEL`（默认 gpt-4o-mini）。
    pub fn from_env() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_owned());
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Llm for OpenAiLlm {
    async fn complete(&self, prompt: &LlmPrompt) -> Result<String, AgentError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user }
            ],
            "temperature": 0.7
        });
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentError::Llm(format!("请求失败: {e}")))?;
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgentError::Llm(format!("解析响应失败: {e}")))?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        Ok(content)
    }
}
