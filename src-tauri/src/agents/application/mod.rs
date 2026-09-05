//! 多 Agent 协作子系统 —— 应用层。
//!
//! 编排流程、定义端口、装配服务。本层不碰 HTTP，也不碰具体 LLM 厂商。

pub mod orchestrator;
pub mod ports;
pub mod service;
