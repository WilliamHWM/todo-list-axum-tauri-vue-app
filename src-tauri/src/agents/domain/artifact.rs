//! 协作产物、消息与运行结果。
//!
//! - [`Artifact`]：某个角色在流程中产出的"交付物"（PRD、设计、代码片段、测试报告、评审）。
//! - [`Message`]：消息总线上的对话记录（谁对谁说了什么），最终汇成 [`DevRun::transcript`]。
//! - [`ReviewDecision`]：技术负责人评审的结构化结论，编排器据此决定合入或返工。
//! - [`DevRun`]：一次完整协作运行的快照，作为北向网关的返回体。

use crate::agents::domain::role::AgentRole;
use serde::{Deserialize, Serialize};

/// 交付物的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Spec,
    Design,
    BackendCode,
    FrontendCode,
    TestPlan,
    TestReport,
    Review,
}

impl ArtifactKind {
    /// 中文标签，用于标题与消息。
    pub fn label(self) -> &'static str {
        match self {
            ArtifactKind::Spec => "产品需求",
            ArtifactKind::Design => "技术方案",
            ArtifactKind::BackendCode => "后端实现",
            ArtifactKind::FrontendCode => "前端实现",
            ArtifactKind::TestPlan => "测试计划",
            ArtifactKind::TestReport => "测试报告",
            ArtifactKind::Review => "评审结论",
        }
    }
}

/// 消息类型（消息总线上的对话种类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    System,
    Note,
    Spec,
    Design,
    BackendCode,
    FrontendCode,
    TestPlan,
    TestReport,
    Review,
    Revision,
}

/// 一次交付物。
///
/// `revision` 从 1 开始；被技术负责人打回后重新产出时自增，便于追踪版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub role: AgentRole,
    pub kind: ArtifactKind,
    pub title: String,
    pub content: String,
    pub revision: u32,
    /// UTC RFC 3339（与全应用统一的时间约定一致）。
    pub created_at: String,
}

/// 消息总线上的单条消息。
///
/// `to` 为 `None` 表示广播给全队；否则是点对点（如技术负责人点名某角色返工）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub from: AgentRole,
    pub to: Option<AgentRole>,
    pub kind: MessageKind,
    pub content: String,
    pub ts: String,
}

/// 团队花名册中的单张卡片（用于 `/api/agents/roles` 对外展示阵容）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub role: AgentRole,
    pub name: String,
    pub title: String,
    pub responsibility: String,
}

/// 技术负责人的评审决策（结构化），由 LLM 以 JSON 返回后解析。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDecision {
    /// 是否准予合入。
    pub approved: bool,
    /// 评审意见（无论通过与否都给出理由）。
    pub comments: String,
    /// 若未通过，点名需要返工的角色；通过时为 `None`。
    pub target: Option<AgentRole>,
}

/// 一次完整协作运行的快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevRun {
    pub id: String,
    pub requirement: String,
    /// UTC RFC 3339。
    pub started_at: String,
    /// UTC RFC 3339。
    pub finished_at: String,
    /// 评审迭代轮数（含通过的最后一轮）。
    pub iterations: u32,
    /// 最终是否通过技术负责人评审。
    pub approved: bool,
    /// 当前生效的全部交付物（被返工的角色只保留最新一版）。
    pub artifacts: Vec<Artifact>,
    /// 完整对话记录（含所有历史版本与返工请求），按时间排序。
    pub transcript: Vec<Message>,
}
