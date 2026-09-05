//! Agent 角色定义。
//!
//! 每个角色对应真实软件团队中的一个岗位：产品、架构、后端、前端、测试、技术负责人。
//! 角色自带"人设"（system prompt 片段）、职责说明与展示信息；编排器据此决定
//! 由谁在流程的哪一步产出什么。

use serde::{Deserialize, Serialize};

/// 协作团队中的角色。
///
/// 通过 `serde(rename_all = "snake_case")` 同时作为对外 JSON 的枚举值（如
/// 评审决策里的 `target` 字段），保证前后端一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentRole {
    ProductManager,
    Architect,
    BackendDev,
    FrontendDev,
    QaEngineer,
    TechLead,
}

impl AgentRole {
    /// 中文展示名。
    pub fn name(self) -> &'static str {
        match self {
            AgentRole::ProductManager => "产品经理",
            AgentRole::Architect => "架构师",
            AgentRole::BackendDev => "后端工程师",
            AgentRole::FrontendDev => "前端工程师",
            AgentRole::QaEngineer => "测试工程师",
            AgentRole::TechLead => "技术负责人",
        }
    }

    /// 岗位头衔。
    pub fn title(self) -> &'static str {
        match self {
            AgentRole::ProductManager => "Product Manager",
            AgentRole::Architect => "Software Architect",
            AgentRole::BackendDev => "Backend Engineer",
            AgentRole::FrontendDev => "Frontend Engineer",
            AgentRole::QaEngineer => "QA Engineer",
            AgentRole::TechLead => "Tech Lead",
        }
    }

    /// 职责说明。
    pub fn responsibility(self) -> &'static str {
        match self {
            AgentRole::ProductManager => "梳理需求、撰写 PRD、定义用户故事与验收标准。",
            AgentRole::Architect => "做技术选型与模块划分，输出架构设计与数据模型。",
            AgentRole::BackendDev => "实现领域层与 API，落实不变量与事务。",
            AgentRole::FrontendDev => "实现 UI 组件与状态管理，对接 API。",
            AgentRole::QaEngineer => "编写测试计划、执行测试并产出测试报告。",
            AgentRole::TechLead => "评审全部产物，决定是否合入或要求返工。",
        }
    }

    /// 该角色固定的人设 / 系统提示词，喂给 LLM 以塑造输出风格与关注点。
    pub fn system_prompt(self) -> String {
        let base = "你是一个高效、严谨的软件工程协作团队成员，输出直接、可落地，避免空话。";
        let duty = match self {
            AgentRole::ProductManager => {
                "你是产品经理：用 PRD 把模糊需求变成清晰、可验收的产品定义。"
            }
            AgentRole::Architect => {
                "你是架构师：给出与现有菱形（六边形）架构一致的模块化技术方案。"
            }
            AgentRole::BackendDev => {
                "你是后端工程师：用 Rust + Axum + SQLx 落地领域不变量与事务。"
            }
            AgentRole::FrontendDev => {
                "你是前端工程师：用 Vue 3 + TypeScript + Pinia 实现 UI 与状态管理。"
            }
            AgentRole::QaEngineer => {
                "你是测试工程师：覆盖输入校验、正常流程、分页与事务回滚，输出可量化结果。"
            }
            AgentRole::TechLead => {
                "你是技术负责人：严格评审，只以 JSON 给出结论，决定合入或返工。"
            }
        };
        format!("{base}\n{duty}")
    }

    /// 完整团队默认阵容。
    pub fn all() -> &'static [AgentRole] {
        &[
            AgentRole::ProductManager,
            AgentRole::Architect,
            AgentRole::BackendDev,
            AgentRole::FrontendDev,
            AgentRole::QaEngineer,
            AgentRole::TechLead,
        ]
    }

    /// 把字符串解析回角色（解析评审决策里的 `target` 用）。
    pub fn from_snake(s: &str) -> Option<AgentRole> {
        match s {
            "productManager" | "product_manager" => Some(AgentRole::ProductManager),
            "architect" => Some(AgentRole::Architect),
            "backendDev" | "backend_dev" => Some(AgentRole::BackendDev),
            "frontendDev" | "frontend_dev" => Some(AgentRole::FrontendDev),
            "qaEngineer" | "qa_engineer" => Some(AgentRole::QaEngineer),
            "techLead" | "tech_lead" => Some(AgentRole::TechLead),
            _ => None,
        }
    }
}
