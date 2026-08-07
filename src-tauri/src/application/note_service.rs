//! 笔记用例服务。
//!
//! 与 [`crate::application::task_service::TaskService`] 相同的用例编排模式。

use crate::application::error::ServiceError;
use crate::application::{CreateNoteDto, UpdateNoteDto};
use crate::domain::{DomainError, Note, NoteRepository};
use std::sync::Arc;

/// 笔记用例服务，持有一个 `NoteRepository` 端口。
#[derive(Clone)]
pub struct NoteService {
    repo: Arc<dyn NoteRepository>,
}

impl NoteService {
    pub fn new(repo: Arc<dyn NoteRepository>) -> Self {
        Self { repo }
    }

    /// 创建笔记：校验内容 → 生成实体 → 持久化。
    pub async fn create(&self, dto: CreateNoteDto) -> Result<Note, ServiceError> {
        let note = Note::new(dto.task_id, &dto.content)?;
        self.repo.insert(&note).await?;
        Ok(note)
    }

    /// 返回某个任务的全部笔记（按创建时间升序）。
    pub async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, ServiceError> {
        Ok(self.repo.list_by_task(task_id).await?)
    }

    /// 按 ID 查询单个笔记；不存在返回 `NoteNotFound`。
    pub async fn get(&self, id: &str) -> Result<Note, ServiceError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::NoteNotFound.into())
    }

    /// 更新笔记：加载 → 应用变更 → 持久化。
    pub async fn update(&self, id: &str, dto: UpdateNoteDto) -> Result<Note, ServiceError> {
        let mut note = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::NoteNotFound)?;
        note.update(dto.content.as_deref())?;
        self.repo.update(&note).await?;
        Ok(note)
    }

    /// 删除笔记；不存在返回 `NoteNotFound`。
    pub async fn delete(&self, id: &str) -> Result<(), ServiceError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::NoteNotFound.into())
        }
    }
}
