//! 任务用例服务。
//!
//! 每个方法对应一个应用用例：构造/变更领域实体（不变量在实体内部校验），然后
//! 通过注入的 `TaskRepository` 南向端口持久化。服务实现 [`TaskUseCase`] 北向端口，
//! 供表现层（北向网关）以接口形式调用。
//!
//! ## 事务处理
//!
//! 跨仓储的原子用例（如 [`TaskService::create_task_with_note`]）通过注入的
//! [`TransactionManager`]（领域南向端口）开启事务，应用层不感知任何 sqlx 类型：
//!
//! - `tx_manager.begin()` 返回绑定事务的 [`TransactionContext`]（具体类型，无装箱）
//! - 事务内通过 `ctx.tasks()` / `ctx.notes()` 获取仓储执行写入
//! - `ctx.commit()` 消费上下文并提交；任一步出错 → 上下文 drop → 自动 ROLLBACK

use crate::application::error::ServiceError;
use crate::application::ports::TaskUseCase;
use crate::application::{CreateTaskDto, CreateTaskWithNoteDto, UpdateTaskDto};
use crate::domain::{
    DomainError, Note, Task, TaskList, TaskQuery, TaskRepository, NoteRepository,
    TransactionContext, TransactionManager,
};
use std::sync::Arc;

/// 任务用例服务。
///
/// - `repo`：独立读写任务（非事务路径）
/// - `tx_manager`：事务管理器（领域南向端口），供跨表原子用例开启事务
#[derive(Clone)]
pub struct TaskService<M: TransactionManager> {
    repo: Arc<dyn TaskRepository>,
    tx_manager: Arc<M>,
}

impl<M: TransactionManager> TaskService<M> {
    pub fn new(repo: Arc<dyn TaskRepository>, tx_manager: Arc<M>) -> Self {
        Self { repo, tx_manager }
    }
}

#[async_trait::async_trait]
impl<M: TransactionManager> TaskUseCase for TaskService<M> {
    /// 过滤 + 排序 + 分页查询任务。
    async fn list(&self, query: TaskQuery) -> Result<TaskList, ServiceError> {
        Ok(self.repo.search(&query).await?)
    }

    /// 创建任务：校验标题 → 生成实体 → 持久化。
    async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError> {
        let task = Task::new(&dto.title)?;
        self.repo.insert(&task).await?;
        Ok(task)
    }

    /// 原子创建任务并附带首条笔记。
    ///
    /// 事务边界在此用例开启：
    /// 1. `tx_manager.begin()` 开启 SQLite `BEGIN` 事务，返回具体的事务上下文
    /// 2. 事务内插入任务（`ctx.tasks()`）
    /// 3. 事务内插入笔记（`ctx.notes()`）
    /// 4. `ctx.commit()` 提交；任一步 `Err` → 上下文 drop → 自动 ROLLBACK
    async fn create_task_with_note(
        &self,
        dto: CreateTaskWithNoteDto,
    ) -> Result<Task, ServiceError> {
        let mut ctx = self.tx_manager.begin().await?;
        let task = Task::new(&dto.title)?;
        ctx.tasks().insert(&task).await?;
        let note = Note::new(Some(task.id().to_owned()), &dto.content)?;
        ctx.notes().insert(&note).await?;
        ctx.commit().await?;
        Ok(task)
    }

    /// 更新任务：加载 → 应用变更 → 持久化。
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
}
