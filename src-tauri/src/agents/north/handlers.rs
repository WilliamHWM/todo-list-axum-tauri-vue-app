//! 多 Agent 团队处理器：触发一次协作运行、查看团队阵容。
//!
//! 与既有 handler 风格一致：路由与实现同文件共存；依赖北向端口而非具体服务。

use crate::agents::application::ports::AgentTeamUseCase;
use crate::agents::domain::artifact::{AgentCard, DevRun};
use crate::agents::domain::event::AgentEvent;
use crate::north::error::ApiError;
use crate::north::extract::JsonBody;
use crate::north::response::{ApiResponse, ApiResult};
use crate::north::AppState;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Router,
};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// 本资源路由：`/api/agents` 前缀下的端点。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/run", post(run_project))
        .route("/run/stream", post(run_project_stream))
        .route("/roles", get(roster))
}

/// POST /api/agents/run
///
/// 给定一句需求，跑完多 Agent 协作闭环，返回 [`DevRun`]（含全部产物与对话记录）。
#[derive(Deserialize)]
pub struct RunRequest {
    pub requirement: String,
}

pub async fn run_project(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
    JsonBody(payload): JsonBody<RunRequest>,
) -> ApiResult<DevRun> {
    if payload.requirement.trim().is_empty() {
        return Err(ApiError::bad_request("需求描述不能为空。"));
    }
    let run = agents
        .run_project(&payload.requirement)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(ApiResponse::ok(run))
}

/// GET /api/agents/roles
///
/// 返回当前团队阵容（各角色职责与头衔）。
pub async fn roster(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
) -> ApiResult<Vec<AgentCard>> {
    Ok(ApiResponse::ok(agents.roster().await))
}

/// POST /api/agents/run/stream
///
/// 以 SSE 实时推送一次独立项目协作的过程事件。前端用 `fetch` 读流、`\n\n` 分帧解析 `data:`。
pub async fn run_project_stream(
    State(agents): State<Arc<dyn AgentTeamUseCase>>,
    JsonBody(payload): JsonBody<RunRequest>,
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
            .run_project_stream(&payload.requirement, sender)
            .await
        {
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
