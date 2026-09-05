//! 任务实体：标题不变量 + 变更动作。
//!
//! 身份（UUID）与创建时间在构造时生成，字段私有、只读暴露，杜绝外部随意修改
//! 破坏不变量。所有状态变更必须经过 [`Task::update`]。

use crate::domain::error::DomainError;
use crate::shared::time;
use serde::Serialize;
use specta::Type;
use uuid::Uuid;

/// 标题最大长度（字符数，与前端一致）。
pub const TITLE_MAX_LEN: usize = 120;

/// 待办任务聚合。
#[derive(Type, Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    id: String,
    title: String,
    completed: bool,
    category_id: Option<String>,
    created_at: String,
}

impl Task {
    /// 构造一个新任务，就地校验标题不变量。
    ///
    /// 标题会先 `trim`，空白标题视为无效。生成 UUID 主键与 UTC 创建时间。
    /// 新任务默认不归属任何分类（`category_id = None`）。
    pub fn new(title: &str) -> Result<Self, DomainError> {
        let title = title.trim();
        validate_title(title)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_owned(),
            completed: false,
            category_id: None,
            created_at: time::utc_now_rfc3339(),
        })
    }

    /// 从持久化原始数据重建实体（仅供仓储适配器使用，跳过校验）。
    pub fn rebuild(
        id: String,
        title: String,
        completed: bool,
        category_id: Option<String>,
        created_at: String,
    ) -> Self {
        Self {
            id,
            title,
            completed,
            category_id,
            created_at,
        }
    }

    /// 部分更新：`title` 与 `completed` 至少提供一个。
    ///
    /// 提供的标题会重新校验并 `trim`；返回 `Err(EmptyUpdate)` 表示没有可更新字段。
    pub fn update(
        &mut self,
        title: Option<&str>,
        completed: Option<bool>,
    ) -> Result<(), DomainError> {
        if title.is_none() && completed.is_none() {
            return Err(DomainError::EmptyUpdate);
        }
        if let Some(title) = title {
            let title = title.trim();
            validate_title(title)?;
            self.title = title.to_owned();
        }
        if let Some(completed) = completed {
            self.completed = completed;
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn category_id(&self) -> Option<&str> {
        self.category_id.as_deref()
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

fn validate_title(title: &str) -> Result<(), DomainError> {
    if title.is_empty() {
        return Err(DomainError::TaskTitleEmpty);
    }
    if title.chars().count() > TITLE_MAX_LEN {
        return Err(DomainError::TaskTitleTooLong);
    }
    Ok(())
}
