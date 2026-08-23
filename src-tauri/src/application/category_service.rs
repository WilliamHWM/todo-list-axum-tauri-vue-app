//! 分类用例服务。
//!
//! 与 [`crate::application::note_service::NoteService`] 相同的用例编排模式，实现
//! [`CategoryUseCase`] 北向端口。分类是独立聚合，不依赖其他聚合，作为「新模块」示范
//! 如何在既有菱形架构下干净接入：仅新增领域实体、仓储端口、用例服务三层，不改动既有模块。

use crate::application::error::ServiceError;
use crate::application::ports::CategoryUseCase;
use crate::application::{CreateCategoryDto, UpdateCategoryDto};
use crate::domain::{Category, CategoryRepository, DomainError};
use std::sync::Arc;

/// 分类用例服务，持有一个 `CategoryRepository` 南向端口。
#[derive(Clone)]
pub struct CategoryService {
    repo: Arc<dyn CategoryRepository>,
}

impl CategoryService {
    pub fn new(repo: Arc<dyn CategoryRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl CategoryUseCase for CategoryService {
    /// 列出全部分类。
    async fn list(&self) -> Result<Vec<Category>, ServiceError> {
        Ok(self.repo.list().await?)
    }

    /// 创建分类：校验名称 → 生成实体 → 持久化。
    async fn create(&self, dto: CreateCategoryDto) -> Result<Category, ServiceError> {
        let category = Category::new(&dto.name, dto.color)?;
        self.repo.create(&category).await?;
        Ok(category)
    }

    /// 按 ID 查询单个分类；不存在返回 `CategoryNotFound`。
    async fn get(&self, id: &str) -> Result<Category, ServiceError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::CategoryNotFound.into())
    }

    /// 更新分类：加载 → 应用变更 → 持久化。
    async fn update(&self, id: &str, dto: UpdateCategoryDto) -> Result<Category, ServiceError> {
        let mut category = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::CategoryNotFound)?;
        category.update(dto.name.as_deref(), dto.color.as_deref())?;
        self.repo.update(&category).await?;
        Ok(category)
    }

    /// 删除分类；不存在返回 `CategoryNotFound`。任务侧的外键为 `SET NULL`，
    /// 相关任务仅失去分类归属，不会被级联删除。
    async fn delete(&self, id: &str) -> Result<(), ServiceError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::CategoryNotFound.into())
        }
    }
}
