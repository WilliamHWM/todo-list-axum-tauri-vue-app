//! 笔记实体：内容不变量 + 变更动作。
//!
//! 与任务实体相同的约定：身份与创建时间在构造时生成，字段私有只读，变更必须
//! 经由 [`Note::update`]。

use crate::domain::error::DomainError;
use crate::shared::time;
use serde::Serialize;
use uuid::Uuid;

/// 内容最大长度（字符数，与前端一致）。
pub const CONTENT_MAX_LEN: usize = 5000;

/// 附属于任务（或独立）的笔记聚合。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    id: String,
    task_id: Option<String>,
    content: String,
    created_at: String,
}

impl Note {
    /// 构造一个新笔记，就地校验内容不变量。
    ///
    /// 内容会先 `trim`，空白视为无效；`task_id` 可为空（独立笔记）。
    pub fn new(task_id: Option<String>, content: &str) -> Result<Self, DomainError> {
        let content = content.trim();
        validate_content(content)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            content: content.to_owned(),
            created_at: time::utc_now_rfc3339(),
        })
    }

    /// 从持久化原始数据重建实体（仅供仓储适配器使用，跳过校验）。
    pub fn rebuild(id: String, task_id: Option<String>, content: String, created_at: String) -> Self {
        Self {
            id,
            task_id,
            content,
            created_at,
        }
    }

    /// 更新内容：提供的值会重新校验并 `trim`。
    pub fn update(&mut self, content: Option<&str>) -> Result<(), DomainError> {
        if let Some(content) = content {
            let content = content.trim();
            validate_content(content)?;
            self.content = content.to_owned();
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

fn validate_content(content: &str) -> Result<(), DomainError> {
    if content.is_empty() {
        return Err(DomainError::NoteContentEmpty);
    }
    if content.chars().count() > CONTENT_MAX_LEN {
        return Err(DomainError::NoteContentTooLong);
    }
    Ok(())
}
