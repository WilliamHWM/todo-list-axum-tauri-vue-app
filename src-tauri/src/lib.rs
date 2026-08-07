//! Axum + Tauri 组合根。
//!
//! 按 DDD 分层组装：`shared`（跨层约定）→ `infrastructure`（连接池 + 仓储实现）
//! → `application`（用例服务）→ `presentation`（Axum 路由）。本文件只负责把各层
//! 装配起来，不包含业务逻辑。
//!
//! Boot order:
//! 1. 初始化 tracing（可选 JSON 输出）。
//! 2. 从环境变量加载 `AppConfig`。
//! 3. 打开 SQLite 连接池并应用迁移（infrastructure）。
//! 4. 构造应用层服务并注入仓储适配器（application + infrastructure）。
//! 5. 绑定 Axum 到随机回环端口，后台任务托管 HTTP 服务。
//! 6. 交给 Tauri 事件循环；端口存入 Tauri 状态，前端通过 `get_api_port` 发现。

mod application;
mod domain;
mod infrastructure;
mod presentation;
mod shared;

use std::sync::Arc;
use tauri::Manager;
use tokio::net::TcpListener;

/// 启动内嵌 Axum 服务器与 Tauri 应用（同一进程）。
///
/// 正常流程下不返回，因为 `app.run()` 阻塞。
#[tokio::main]
pub async fn run() {
    // --- 跨层配置 -----------------------------------------------------------
    let config = shared::AppConfig::from_env();
    init_tracing(&config);
    tracing::info!(
        log_level = %config.log_level,
        log_format = %config.log_format,
        "application configuration loaded"
    );

    // --- 基础设施：连接池 + 迁移 ----------------------------------------------
    let pool = infrastructure::db::init_pool(&config).await.unwrap_or_else(|e| {
        panic!("failed to initialize SQLite database: {e}");
    });
    tracing::info!("database pool initialized and migrations applied");

    // --- 装配：仓储实现 → 应用层服务 → 表现层状态 -------------------------------
    let task_repo = Arc::new(infrastructure::SqlxTaskRepository::new(pool.clone()));
    let note_repo = Arc::new(infrastructure::SqlxNoteRepository::new(pool));
    let state = presentation::AppState {
        tasks: application::TaskService::new(task_repo),
        notes: application::NoteService::new(note_repo),
        config: config.clone(),
    };

    // --- API 服务器 -----------------------------------------------------------
    let app = presentation::create_router(state);
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind local API port: {e}"));
    let addr = listener
        .local_addr()
        .expect("failed to read local API address");
    let port = addr.port();
    tracing::info!(%addr, "Axum API server started");

    let axum_handle = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(?error, "Axum server stopped unexpectedly");
        }
    });

    // --- Tauri -----------------------------------------------------------------
    let context = tauri::generate_context!();
    tauri::Builder::<tauri::Wry>::new()
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            app.manage(port);
            Ok(())
        })
        .run(context)
        .unwrap_or_else(|e| {
            panic!("error while running tauri application: {e}");
        });
    // 仅在 Tauri 事件循环被人工终止时（例如测试）才会到达，用于停止 HTTP 服务。
    axum_handle.abort();
}

/// 初始化结构化日志。
///
/// 优先读取 `RUST_LOG`，否则回退到 `APP_LOG_LEVEL`；`APP_LOG_FORMAT=json` 输出
/// JSON 行便于日志采集。
fn init_tracing(config: &shared::AppConfig) {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.log_format == "json" {
        let _ = fmt().json().with_env_filter(filter).try_init();
    } else {
        let _ = fmt().with_env_filter(filter).try_init();
    }
}

/// 返回内嵌 Axum 服务器监听的端口。
///
/// 前端通过 `invoke("get_api_port")` 调用。
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
