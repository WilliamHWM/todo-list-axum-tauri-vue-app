//! 多 Agent 软件协作团队 —— 领域层。
//!
//! 本子系统自成一体，内部仍然遵循菱形（六边形）架构：领域核心居中，应用层编排，
//! 南向网关（LLM 适配器）实现领域定义的端口，北向网关（axum 路由）只依赖应用层
//! 北向端口。
//!
//! 领域层只定义"协作团队是什么"：角色、产物、消息、错误、LLM 端口契约。
//! 不依赖任何框架（axum/sqlx/tauri/reqwest），也不依赖具体 LLM 厂商。

pub mod artifact;
pub mod error;
pub mod event;
pub mod repository;
pub mod role;
