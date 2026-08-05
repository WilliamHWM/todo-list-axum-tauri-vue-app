//! Data models for the application.
//!
//! This module defines the domain structs used throughout the crate:
//! the `Task` entity stored in SQLite, and the request DTOs for
//! creating and updating tasks. All types derive `Serialize`/`Deserialize`
//! so they can be used directly as API request/response bodies.

pub mod task;

pub use task::{CreateTaskRequest, Task, UpdateTaskRequest};
