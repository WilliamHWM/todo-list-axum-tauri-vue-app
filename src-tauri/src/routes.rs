use crate::{
    db,
    models::{CreateTaskRequest, Task, UpdateTaskRequest},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { message: self.1 })).into_response()
    }
}
type ApiResult<T> = Result<Json<T>, ApiError>;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id", put(update_task).delete(delete_task))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
async fn list_tasks(State(state): State<AppState>) -> ApiResult<Vec<Task>> {
    db::get_all_tasks(&state.db)
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> ApiResult<Task> {
    let title = payload.title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "任务标题必须是 1 到 120 个字符。".into(),
        ));
    }
    db::create_task(&state.db, CreateTaskRequest { title })
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut payload): Json<UpdateTaskRequest>,
) -> ApiResult<Task> {
    if payload.title.is_none() && payload.completed.is_none() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "至少提供一个需要更新的字段。".into(),
        ));
    }
    if let Some(title) = &mut payload.title {
        *title = title.trim().to_owned();
        if title.is_empty() || title.chars().count() > 120 {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "任务标题必须是 1 到 120 个字符。".into(),
            ));
        }
    }
    db::update_task(&state.db, &id, payload)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "任务不存在或已被删除。".into()))
}
async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if db::delete_task(&state.db, &id)
        .await
        .map_err(internal_error)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(
            StatusCode::NOT_FOUND,
            "任务不存在或已被删除。".into(),
        ))
    }
}
fn internal_error(error: sqlx::Error) -> ApiError {
    tracing::error!(?error, "database request failed");
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "数据库操作失败，请稍后重试。".into(),
    )
}
