//! 应用层输入 DTO。
//!
//! 只做 JSON 反序列化的纯数据载体；是否合法由领域实体在 `new`/`update` 时校验，
//! 这里不做任何业务规则。

use serde::Deserialize;

/// 创建任务的输入。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskDto {
    pub title: String,
}

/// 更新任务的输入（字段全部可选，但至少提供一个）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskDto {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

/// 创建笔记的输入。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteDto {
    pub task_id: Option<String>,
    pub content: String,
}

/// 更新笔记的输入。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteDto {
    pub content: Option<String>,
}
