//! 任务用例服务。
//!
//! 每个方法对应一个应用用例：构造/变更领域实体（不变量在实体内部校验），然后
//! 通过注入的 `TaskRepository` 端口持久化。服务本身不感知 SQL 与 HTTP。

use crate::application::error::ServiceError;
use crate::application::{CreateTaskDto, UpdateTaskDto};
use crate::domain::{DomainError, Task, TaskList, TaskQuery, TaskRepository};
use std::sync::Arc;

/// 任务用例服务，持有一个 `TaskRepository` 端口（生产为 sqlx 适配器）。
#[derive(Clone)]
pub struct TaskService {
    repo: Arc<dyn TaskRepository>,
}

impl TaskService {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    /// 创建任务：校验标题 → 生成实体 → 持久化。
    pub async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError> {
        let task = Task::new(&dto.title)?;
        self.repo.insert(&task).await?;
        Ok(task)
    }

    /// 更新任务：加载 → 应用变更 → 持久化。
    ///
    /// 实体不存在返回 `TaskNotFound`；字段非法返回对应领域错误。
    pub async fn update(&self, id: &str, dto: UpdateTaskDto) -> Result<Task, ServiceError> {
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
    pub async fn delete(&self, id: &str) -> Result<(), ServiceError> {
        if self.repo.delete(id).await? {
            Ok(())
        } else {
            Err(DomainError::TaskNotFound.into())
        }
    }

    /// 过滤 + 排序 + 分页查询任务。
    pub async fn list(&self, query: TaskQuery) -> Result<TaskList, ServiceError> {
        Ok(self.repo.search(&query).await?)
    }
}
