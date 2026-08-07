//! 北向端口：应用层对外暴露的用例接口。
//!
//! 菱形架构中，北向网关（表现层）只依赖这里的接口，不依赖具体服务实现。
//! 这样 UI / API / CLI 可自由替换，北向也能注入 mock 做测试。

use crate::application::dto::{
    CreateNoteDto, CreateTaskDto, CreateTaskWithNoteDto, UpdateNoteDto, UpdateTaskDto,
};
use crate::application::error::ServiceError;
use crate::domain::{Note, Task, TaskList, TaskQuery};

/// 北向端口：任务用例。
#[async_trait::async_trait]
pub trait TaskUseCase: Send + Sync {
    /// 过滤 + 排序 + 分页查询任务。
    async fn list(&self, query: TaskQuery) -> Result<TaskList, ServiceError>;
    /// 创建任务。
    async fn create(&self, dto: CreateTaskDto) -> Result<Task, ServiceError>;
    /// 原子创建任务并附带首条笔记（同一事务，要么都成功要么都回滚）。
    async fn create_task_with_note(&self, dto: CreateTaskWithNoteDto) -> Result<Task, ServiceError>;
    /// 更新任务（部分字段）。
    async fn update(&self, id: &str, dto: UpdateTaskDto) -> Result<Task, ServiceError>;
    /// 删除任务。
    async fn delete(&self, id: &str) -> Result<(), ServiceError>;
}

/// 北向端口：笔记用例。
#[async_trait::async_trait]
pub trait NoteUseCase: Send + Sync {
    /// 创建笔记。
    async fn create(&self, dto: CreateNoteDto) -> Result<Note, ServiceError>;
    /// 返回某任务下的全部笔记。
    async fn list_by_task(&self, task_id: &str) -> Result<Vec<Note>, ServiceError>;
    /// 按 ID 查询单个笔记。
    async fn get(&self, id: &str) -> Result<Note, ServiceError>;
    /// 更新笔记。
    async fn update(&self, id: &str, dto: UpdateNoteDto) -> Result<Note, ServiceError>;
    /// 删除笔记。
    async fn delete(&self, id: &str) -> Result<(), ServiceError>;
}
