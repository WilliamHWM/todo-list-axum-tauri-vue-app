//! 应用层：用例服务 + 输入 DTO。
//!
//! 本层编排领域实体与仓储端口，承载"每个用例该做什么"的业务逻辑；不涉及 HTTP
//! 与 SQL。表现层拿到 DTO 后调用这里的服务，基础设施层实现仓储端口注入进来。

pub mod dto;
pub mod error;
pub mod note_service;
pub mod task_service;

pub use dto::{CreateNoteDto, CreateTaskDto, UpdateNoteDto, UpdateTaskDto};
pub use error::ServiceError;
pub use note_service::NoteService;
pub use task_service::TaskService;
