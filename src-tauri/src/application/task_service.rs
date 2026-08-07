//! 任务用例服务。
//!
//! 每个方法对应一个应用用例：构造/变更领域实体（不变量在实体内部校验），然后
//! 通过注入的 `TaskRepository` 南向端口持久化。服务实现 [`TaskUseCase`] 北向端口，
//! 供表现层（北向网关）以接口形式调用。
//!
//! 跨仓储的原子用例（如 [`TaskService::create_task_with_note`]）通过注入的
//! [`UnitOfWorkFactory`] 南向端口开启数据库事务，保证"要么全成、要么全滚"。

use crate::application::error::ServiceError;
use crate::application::ports::TaskUseCase;
use crate::application::{CreateTaskDto, CreateTaskWithNoteDto, UpdateTaskDto};
use crate::domain::{DomainError, Note, Task, TaskList, TaskQuery, TaskRepository, UnitOfWorkFactory};
use std::sync::Arc;

/// 任务用例服务，持有 `TaskRepository` 与 `UnitOfWorkFactory` 两个南向端口。
#[derive(Clone)]
pub struct TaskService {
    repo: Arc<dyn TaskRepository>,
    uow: Arc<dyn UnitOfWorkFactory>,
}

impl TaskService {
    pub fn new(repo: Arc<dyn TaskRepository>, uow: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { repo, uow }
    }
}

#[async_trait::async_trait]
impl TaskUseCase for TaskService {
    /// 创建任务：校验标题 → 生成实体 → 持久化。
    async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError> {
        let task = Task::new(&dto.title)?;
        self.repo.insert(&task).await?;
        Ok(task)
    }

    /// 原子创建任务并附带首条笔记。
    ///
    /// 整个流程在一个数据库事务里：先插任务，再插笔记，全部成功才 `commit`；
    /// 任何一步失败（包括笔记内容非法、写入出错）都会回滚，不会留下只有任务
    /// 没有笔记的"孤儿"数据。
    async fn create_task_with_note(&self, dto: CreateTaskWithNoteDto) -> Result<Task, ServiceError> {
        let mut uow = self.uow.begin().await?;
        let task = Task::new(&dto.title)?;
        uow.task_repo().insert(&task).await?;
        let note = Note::new(Some(task.id().to_owned()), &dto.content)?;
        uow.note_repo().insert(&note).await?;
        uow.commit().await?;
        Ok(task)
    }

    /// 更新任务：加载 → 应用变更 → 持久化。
    ///
    /// 实体不存在返回 `TaskNotFound`；字段非法返回对应领域错误。
    async fn update(&self, id: &str, dto: UpdateTaskDto) -> Result<Task, ServiceError> {
        let mut task = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::TaskNotFound)?;
        task.update(dto.title.as_deref(), dto.completed)?;
        self.repo.update(&task).await?;
        Ok(task)
    }

    /// 删除任务；不存在返回 `TaskNotFound`。
    async fn delete(&self, id: &str) -> Result<(), ServiceError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::TaskNotFound.into())
        }
    }

    /// 过滤 + 排序 + 分页查询任务。
    async fn list(&self, query: TaskQuery) -> Result<TaskList, ServiceError> {
        Ok(self.repo.search(&query).await?)
    }
}
