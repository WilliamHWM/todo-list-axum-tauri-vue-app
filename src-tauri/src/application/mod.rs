//! 应用层：用例服务 + 输入 DTO + 北向端口。
//!
//! 本层编排领域实体与南向仓储端口，承载"每个用例该做什么"的业务逻辑；不涉及 HTTP
//! 与 SQL。`ports.rs` 定义北向端口（表现层依赖的接口），服务实现之；南向端口
//! （仓储 trait）由领域层定义、基础设施层实现、此处注入。

pub mod dto;
pub mod error;
pub mod note_service;
pub mod ports;
pub mod task_service;

pub use dto::{CreateNoteDto, CreateTaskDto, UpdateNoteDto, UpdateTaskDto};
pub use error::ServiceError;
pub use note_service::NoteService;
pub use ports::{NoteUseCase, TaskUseCase};
pub use task_service::TaskService;
