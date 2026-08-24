//! 北向网关：HTTP 入口、路由、中间件与共享状态。
//!
//! 本层只做"翻译"：HTTP 请求 → DTO → 调用北向端口（`TaskUseCase`/`NoteUseCase`）
//! → 响应。只依赖应用层接口，不包含业务规则，也不直接依赖南向实现。

pub mod error;
pub mod extract;
pub mod handlers;
pub mod response;
pub mod routes;

pub use routes::create_router;

use crate::application::{CategoryUseCase, NoteUseCase, TaskUseCase};
use axum::extract::FromRef;
use std::sync::Arc;

/// Axum 路由共享状态：北向端口集合。
///
/// 只装"请求处理真正依赖的东西"（各用例端口）；跨层配置不进状态——
/// `create_router(state, &config)` 在装配期消费配置，请求路径上无感知。
///
/// `#[derive(FromRef)]` 为每个字段自动生成 `FromRef` 实现：handler 直接
/// `State<Arc<dyn TaskUseCase>>` 提取自己需要的那一个端口，而非背负整个
/// `AppState`。新增 service = 加一个字段，旧 handler 无需改动。
#[derive(Clone, FromRef)]
pub struct AppState {
    pub tasks: Arc<dyn TaskUseCase>,
    pub notes: Arc<dyn NoteUseCase>,
    pub categories: Arc<dyn CategoryUseCase>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        CategoryService, CategoryUseCase, NoteService, NoteUseCase, TaskService, TaskUseCase,
    };
    use crate::shared::AppConfig;
    use crate::south::db::init_pool;
    use crate::south::{
        SqlxCategoryRepository, SqlxNoteRepository, SqlxTaskRepository, SqlxTransactionManager,
    };
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    /// 用唯一临时库构建测试路由：真实 sqlx 仓储 + 应用层服务（经北向端口注入），
    /// 配置在装配期传给 `create_router`，不进入 `AppState`。
    async fn test_app() -> axum::Router {
        let db_path = std::env::temp_dir().join(format!("axum_api_test_{}.db", Uuid::new_v4()));
        let config = AppConfig {
            database_url: format!("sqlite:{}?mode=rwc", db_path.display()),
            host: "127.0.0.1".to_owned(),
            port: 0,
            log_level: "info".to_owned(),
            log_format: "text".to_owned(),
            request_timeout_secs: 15,
            db_max_connections: 5,
        };
        let pool = init_pool(&config).await.expect("init_pool failed");
        let task_repo = Arc::new(SqlxTaskRepository::new(pool.clone()));
        let note_repo = Arc::new(SqlxNoteRepository::new(pool.clone()));
        let category_repo = Arc::new(SqlxCategoryRepository::new(pool.clone()));
        let tx_manager = Arc::new(SqlxTransactionManager::new(pool.clone()));
        let tasks: Arc<dyn TaskUseCase> = Arc::new(TaskService::new(task_repo, tx_manager));
        let notes: Arc<dyn NoteUseCase> = Arc::new(NoteService::new(note_repo));
        let categories: Arc<dyn CategoryUseCase> = Arc::new(CategoryService::new(category_repo));
        create_router(AppState { tasks, notes, categories }, &config)
    }

    async fn json_response(app: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = app.oneshot(request).await.expect("request failed");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body failed");
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    fn post_json(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    fn put_json(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method("PUT")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok_envelope() {
        let app = test_app().await;
        let (status, body) = json_response(app, get("/api/health")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], 0);
        assert_eq!(body["message"], "ok");
        assert_eq!(body["data"], "ok");
    }

    #[tokio::test]
    async fn create_task_returns_task_with_utc_timestamp() {
        let app = test_app().await;
        let (status, body) =
            json_response(app, post_json("/api/tasks", r#"{"title":"写测试"}"#)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], 0);
        assert!(!body["data"]["id"].as_str().unwrap().is_empty());
        assert_eq!(body["data"]["title"], "写测试");
        // RFC 3339 UTC：以 Z 结尾，前端可无歧义解析。
        let created = body["data"]["createdAt"].as_str().unwrap();
        assert!(created.ends_with('Z'), "expected UTC RFC3339, got {created}");
    }

    #[tokio::test]
    async fn create_task_rejects_blank_title() {
        let app = test_app().await;
        let (status, body) =
            json_response(app, post_json("/api/tasks", r#"{"title":"   "}"#)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!body["message"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_task_rejects_title_too_long() {
        let long = "x".repeat(121);
        let body = format!(r#"{{"title":"{long}"}}"#);
        let app = test_app().await;
        let (status, _) = json_response(app, post_json("/api/tasks", &body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_task_rejects_missing_title() {
        let app = test_app().await;
        let (status, _) = json_response(app, post_json("/api/tasks", "{}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_tasks_paginates() {
        let app = test_app().await;
        for i in 0..3 {
            json_response(
                app.clone(),
                post_json("/api/tasks", &format!(r#"{{"title":"任务{i}"}}"#)),
            )
            .await;
        }
        let (status, body) = json_response(app, get("/api/tasks?limit=2&offset=0")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["total"], 3);
        assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn delete_missing_task_returns_404() {
        let app = test_app().await;
        let request = Request::builder()
            .method("DELETE")
            .uri("/api/tasks/not-exist")
            .body(Body::empty())
            .unwrap();
        let (status, _) = json_response(app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_task_with_note_commits_atomically() {
        let app = test_app().await;
        let (status, body) = json_response(
            app.clone(),
            post_json(
                "/api/tasks/with-note",
                r#"{"title":"带首条笔记的任务","content":"第一条笔记"}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["title"], "带首条笔记的任务");

        // 任务与笔记都在同一个事务里提交成功。
        let task_id = body["data"]["id"].as_str().unwrap().to_string();
        let (status, notes) = json_response(app, get(&format!("/api/tasks/{task_id}/notes"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(notes["data"].as_array().unwrap().len(), 1);
        assert_eq!(notes["data"][0]["content"], "第一条笔记");
    }

    #[tokio::test]
    async fn create_task_with_note_rolls_back_on_invalid_content() {
        let app = test_app().await;
        let (status, _) = json_response(
            app.clone(),
            post_json(
                "/api/tasks/with-note",
                r#"{"title":"会被回滚的任务","content":"   "}"#,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // 笔记内容非法 → 整个事务回滚 → 任务不应被创建。
        let (status, body) = json_response(app, get("/api/tasks")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["total"], 0, "回滚后不应残留孤儿任务");
    }

    #[tokio::test]
    async fn categories_crud_and_assign_to_task() {
        let app = test_app().await;

        // 创建分类
        let (status, body) = json_response(
            app.clone(),
            post_json("/api/categories", r##"{"name":"工作","color":"#ff0000"}"##),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["name"], "工作");
        let cat_id = body["data"]["id"].as_str().unwrap().to_string();

        // 列出分类
        let (status, body) = json_response(app.clone(), get("/api/categories")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"].as_array().unwrap().len(), 1);

        // 创建任务并归属到该分类
        let (_, body) = json_response(
            app.clone(),
            post_json("/api/tasks", r#"{"title":"写季度报告"}"#),
        )
        .await;
        let task_id = body["data"]["id"].as_str().unwrap().to_string();
        let (status, body) = json_response(
            app.clone(),
            put_json(
                &format!("/api/tasks/{task_id}/category"),
                &format!(r#"{{"categoryId":"{cat_id}"}}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["categoryId"], cat_id);

        // 删除分类应成功，且任务因外键 SET NULL 仍健在（分类置空）
        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/categories/{cat_id}"))
            .body(Body::empty())
            .unwrap();
        let (status, _) = json_response(app.clone(), request).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, body) = json_response(app, get("/api/categories")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
    }
}
