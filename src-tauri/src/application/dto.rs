//! 应用层输入 DTO。
//!
//! 只做 JSON 反序列化的纯数据载体；是否合法由领域实体在 `new`/`update` 时校验，
//! 这里不做任何业务规则。
//!
//! `#[derive(Type)]` 标记的类型会被生成到前端 `src/domain/generated.ts`，作为前后端
//! 共享的类型契约（见项目 README 的"类型共享"一节）。

use serde::Deserialize;
use specta::Type;

/// 创建任务的输入。
#[derive(Type, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskDto {
    pub title: String,
}

/// 原子创建任务并附带首条笔记的输入。
#[derive(Type, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskWithNoteDto {
    pub title: String,
    pub content: String,
}

/// 更新任务的输入（字段全部可选，但至少提供一个）。
#[derive(Type, Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskDto {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

/// 创建笔记的输入。
#[derive(Type, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteDto {
    pub task_id: Option<String>,
    pub content: String,
}

/// 更新笔记的输入。
#[derive(Type, Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteDto {
    pub content: Option<String>,
}

/// 创建分类的输入。
#[derive(Type, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCategoryDto {
    pub name: String,
    pub color: Option<String>,
}

/// 更新分类的输入（字段全部可选，但至少提供一个）。
#[derive(Type, Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCategoryDto {
    pub name: Option<String>,
    pub color: Option<String>,
}

/// 设置任务所属分类的输入（`null` 表示清除归属）。
#[derive(Type, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTaskCategoryDto {
    pub category_id: Option<String>,
}
