//! 分类聚合（独立聚合根）。
//!
//! 与 `Task` / `Note` 相同的约定：身份（UUID）与创建时间在构造时生成，字段私有只读，
//! 状态变更必须经 [`Category::update`]。分类是「任务」的归属维度（一对多），自身不依赖
//! 任何其他聚合，因此是干净、独立的新模块，演示一个新聚合如何按菱形架构接入。

use crate::domain::error::DomainError;
use crate::shared::time;
use serde::Serialize;
use specta::Type;
use uuid::Uuid;

/// 分类名称最大长度（字符数）。
pub const NAME_MAX_LEN: usize = 40;

/// 分类聚合：名称 + 展示色 + 创建时间。
#[derive(Type, Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    id: String,
    name: String,
    color: String,
    created_at: String,
}

impl Category {
    /// 构造新分类；校验名称不变量，颜色缺省时使用主题色。
    pub fn new(name: &str, color: Option<String>) -> Result<Self, DomainError> {
        let name = name.trim();
        validate_name(name)?;
        let color = color
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| "#409EFF".to_owned());
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            color,
            created_at: time::utc_now_rfc3339(),
        })
    }

    /// 从持久化原始数据重建实体（仅供仓储适配器使用，跳过校验）。
    pub fn rebuild(id: String, name: String, color: String, created_at: String) -> Self {
        Self {
            id,
            name,
            color,
            created_at,
        }
    }

    /// 部分更新；`name` 与 `color` 至少提供一个。
    pub fn update(
        &mut self,
        name: Option<&str>,
        color: Option<&str>,
    ) -> Result<(), DomainError> {
        if name.is_none() && color.is_none() {
            return Err(DomainError::EmptyUpdate);
        }
        if let Some(name) = name {
            let name = name.trim();
            validate_name(name)?;
            self.name = name.to_owned();
        }
        if let Some(color) = color {
            self.color = color.to_owned();
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn color(&self) -> &str {
        &self.color
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

fn validate_name(name: &str) -> Result<(), DomainError> {
    if name.is_empty() {
        return Err(DomainError::CategoryNameEmpty);
    }
    if name.chars().count() > NAME_MAX_LEN {
        return Err(DomainError::CategoryNameTooLong);
    }
    Ok(())
}
