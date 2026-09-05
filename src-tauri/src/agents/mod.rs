//! 多 Agent 软件协作团队子系统。
//!
//! 把"互联网软件团队"建模成一组各司其职的 Agent（产品经理 / 架构师 / 后端 / 前端 /
//! 测试 / 技术负责人），它们通过"共享产物（artifact blackboard）+ 消息总线"协作，
//! 由 [`application::TeamOrchestrator`] 驱动出"产品→架构→研发→测试→评审（可返工）"
//! 的完整开发闭环。
//!
//! 本子系统内部仍严格遵循全工程的菱形（六边形）架构：
//!
//! | 层 | 模块 | 职责 |
//! |---|---|---|
//! | 领域（核心） | [`domain`] | 角色、产物、消息、错误、LLM 端口契约 |
//! | 应用 | [`application`] | 编排器、北向端口 `AgentTeamUseCase`、南向端口 `Llm` |
//! | 南向网关 | [`south`] | LLM 适配器：`MockLlm`（默认）/ `OpenAiLlm` |
//! | 北向网关 | [`north`] | axum 路由：`/api/agents/run`、`/api/agents/roles` |
//!
//! 关键设计：
//! - **可插拔 LLM**：应用层只依赖 `Llm` trait，具体实现由组合根注入；默认 `MockLlm`
//!   零密钥即可跑通，设 `AGENT_LLM=openai` + `OPENAI_API_KEY` 环境变量即可切换真实模型。
//! - **协作而非硬编排**：角色间不直接调用，只读前置产物、写自己的产物、广播消息；
//!   新增角色或调整流程只改编排器。
//! - **返工闭环**：技术负责人评审未通过时结构化点名某角色返工，研发重做后测试复测，
//!   直到通过或达到最大迭代轮数。

pub mod application;
pub mod domain;
pub mod north;
pub mod south;
