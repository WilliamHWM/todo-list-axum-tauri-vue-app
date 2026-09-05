//! 智能体协作的流式事件，用于 SSE 推送，让前端实时回放"团队在干嘛"。
//!
//! 事件通过 [`serde`] 序列化为 JSON 帧；前端按 `type` 字段区分并处理。

use crate::agents::domain::artifact::{Artifact, DevRun, Message};
use serde::Serialize;

/// 一次协作过程中向前端推送的事件。
///
/// - `status`：流程阶段提示（如"产品经理正在产出需求"）。
/// - `message`：共享消息总线上的某条对话。
/// - `artifact`：某角色新交付的一份产物。
/// - `done`：全流程结束，附带最终运行快照。
/// - `error`：流程中断的错误信息。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    Status { phase: String, note: String },
    Message(Message),
    Artifact(Artifact),
    Done(DevRun),
    Error { message: String },
}
