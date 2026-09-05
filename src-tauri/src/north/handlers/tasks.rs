//! 任务处理器：CRUD + 过滤/分页查询。
//!
//! 路由与实现同文件共存（co-location）：[`router`] 声明本资源全部端点，
//! 新增端点只改本文件，`routes.rs` 无需感知。

use super::super::error::ApiError;
use super::super::extract::JsonBody;
use super::super::response::{ApiResponse, ApiResult};
use super::super::AppState;
use crate::agents::application::ports::AgentTeamUseCase;
use crate::agents::domain::artifact::DevRun;
use crate::agents::domain::event::AgentEvent;
use crate::application::{CreateTaskDto, CreateTaskWithNoteDto, SetTaskCategoryDto, UpdateTaskDto};
use crate::domain::{Task, TaskList, TaskQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post, put},
    Json, Router,
};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::application::TaskUseCase;

/// 任务资源路由：`/api/tasks` 前缀下的全部端点。
///
/// `/:id/notes` 是笔记资源的嵌套视图——URL 归属任务、实现归属笔记模块，
/// 路由在此挂载。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/with-note", post(create_task_with_note))
        .route("/:id", put(update_task).delete(delete_task))
        .route("/:id/category", put(set_task_category))
        .route("/:id/notes", get(super::notes::list_notes_by_task))
        .route("/:id/agent-run", post(run_agent_for_task))
        .route("/:id/agent-run/stream", post(run_agent_stream))
        .route("/:id/agent-runs", get(list_agent_runs))
}

/// GET /api/tasks
///
/// 按关键字/完成状态过滤，支持排序与分页。
pub async fn list_tasks(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Query(query): Query<TaskQuery>,
) -> ApiResult<TaskList> {
    let result = tasks.list(query).await?;
    Ok(ApiResponse::ok(result))
}

/// POST /api/tasks
///
/// 创建任务；标题不变量由领域实体校验。
pub async fn create_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    JsonBody(payload): JsonBody<CreateTaskDto>,
) -> ApiResult<Task> {
    let task = tasks.create(payload).await?;
    Ok(ApiResponse::ok(task))
}

/// POST /api/tasks/with-note
///
/// 在同一数据库事务里创建任务并附带首条笔记：任务或笔记任一步失败都会整体回滚。
pub async fn create_task_with_note(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    JsonBody(payload): JsonBody<CreateTaskWithNoteDto>,
) -> ApiResult<Task> {
    let task = tasks.create_task_with_note(payload).await?;
    Ok(ApiResponse::ok(task))
}

/// PUT /api/tasks/:id
///
/// 部分更新；至少提供一个字段，不变量由领域实体校验。
pub async fn update_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskDto>,
) -> ApiResult<Task> {
    let task = tasks.update(&id, payload).await?;
    Ok(ApiResponse::ok(task))
}

/// DELETE /api/tasks/:id
///
/// 成功返回 204；任务不存在返回 404。
pub async fn delete_task(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tasks.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/tasks/:id/category
///
/// 设置任务所属分类；`categoryId` 为 `null` 时清除归属。
pub async fn set_task_category(
    State(tasks): State<Arc<dyn TaskUseCase>>,
    Path(id): Path<String>,
    Json(payload): Json<SetTaskCategoryDto>,
) -> ApiResult<Task> {
    let task = tasks.set_category(&id, payload.category_id).await?;
    Ok(ApiResponse::ok(task))
}

/// POST /api/tasks/:id/agent-run
///
/// 让智能体团队针对「这个任务」协作开发，运行结果持久化到该任务下（见 `agent_runs` 表）。
/// `requirement` 通常即任务标题。
#[derive(Deserialize)]
pub struct RunAgentRequest {
    requirement: String,
}

pub async fn run_agent_for_task(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
    Path(task_id): Path<String>,
    JsonBody(payload): JsonBody<RunAgentRequest>,
) -> ApiResult<DevRun> {
    if payload.requirement.trim().is_empty() {
        return Err(ApiError::bad_request("需求描述不能为空。"));
    }
    let run = agents
        .run_for_task(&task_id, &payload.requirement)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(ApiResponse::ok(run))
}

/// GET /api/tasks/:id/agent-runs
///
/// 列出某任务下的全部智能体运行记录（按时间倒序）。
pub async fn list_agent_runs(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
    Path(task_id): Path<String>,
) -> ApiResult<Vec<DevRun>> {
    let runs = agents
        .list_runs(&task_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(ApiResponse::ok(runs))
}

/// POST /api/tasks/:id/agent-run/stream
///
/// 以 SSE 实时推送智能体协作过程（状态/消息/产物/结束/错误）。前端用 `fetch` 读流、
/// 按 `\n\n` 分帧解析 `data:` 行即可边跑边渲染；流程结束通道自动关闭。
pub async fn run_agent_stream(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
    Path(task_id): Path<String>,
    JsonBody(payload): JsonBody<RunAgentRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    if payload.requirement.trim().is_empty() {
        let (tx, rx) = mpsc::channel::<AgentEvent>(8);
        let _ = tx
            .send(AgentEvent::Error {
                message: "需求描述不能为空。".to_owned(),
            })
            .await;
        drop(tx);
        return sse_from(ReceiverStream::new(rx));
    }

    let (tx, rx) = mpsc::channel::<AgentEvent>(64);
    let tx_err = tx.clone();
    drop(tx);
    tokio::spawn(async move {
        let sender = tx_err.clone();
        if let Err(e) = agents
            .run_for_task_stream(&task_id, &payload.requirement, sender)
            .await
        {
            // 通过通道把错误推给前端（tx_err 在任务结束时才丢弃，故通道保持开启）。
            let _ = tx_err
                .send(AgentEvent::Error {
                    message: e.to_string(),
                })
                .await;
        }
    });
    sse_from(ReceiverStream::new(rx))
}

/// 把事件通道包装成 SSE 响应：每个事件序列化后作为一帧 `data:` 下发，并启用 keep-alive。
fn sse_from(
    rx: ReceiverStream<AgentEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(rx.map(|ev| {
        let data = serde_json::to_string(&ev).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().data(data))
    }))
    .keep_alive(KeepAlive::default())
}
