//! Axum + Tauri 组合根（菱形架构装配）。
//!
//! 装配顺序：`shared`（跨层约定）→ `south`（南向网关：连接池 + 仓储实现）
//! → `application`（用例服务，实现北向端口）→ `north`（北向网关：Axum 路由）。
//! 本文件只负责把各层装配起来，不包含业务逻辑。
//!
//! 菱形架构约定：
//! - 领域核心居中（domain + application）；`application/ports.rs` 定义北向端口。
//! - `north/`（北向网关）只依赖北向端口接口，不依赖具体服务实现。
//! - `south/`（南向网关）实现领域层定义的仓储端口，SQL 只出现在这里。
//!
//! Boot order:
//! 1. 初始化 tracing（可选 JSON 输出）。
//! 2. 从环境变量加载 `AppConfig`。
//! 3. 打开 SQLite 连接池并应用迁移（south）。
//! 4. 构造应用层服务并注入南向适配器（application + south）。
//! 5. 绑定 Axum 到随机回环端口，后台任务托管 HTTP 服务（north）。
//! 6. 交给 Tauri 事件循环；端口存入 Tauri 状态，前端通过 `get_api_port` 发现。

mod application;
mod domain;
mod north;
mod shared;
mod south;

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

    // --- 南向网关：连接池 + 迁移 ----------------------------------------------
    let pool = south::db::init_pool(&config).await.unwrap_or_else(|e| {
        eprintln!("failed to initialize database: {e}");
        std::process::exit(1);
    });
    tracing::info!("database pool initialized and migrations applied");

    // 预热连接池：取出一条连接并立即归还。配置错误（URL 无效、迁移约束违规等）
    // 在此阶段就能暴露，而不是等到首个 HTTP 请求超时。
    pool.acquire().await.expect("database pool warmup failed");

    // --- 装配：组合根在此把南向适配器注入用例服务，产出北向网关状态 ----------------
    let state = build_app_state(pool, config.clone());

    // --- API 服务器（北向网关）-------------------------------------------------
    // Axum 与 Tauri 在同一进程内运行，生命周期绑定；Tauri 退出时进程终止，
    // Axum 服务随之结束，无需单独的 abort。
    let app = north::create_router(state);
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind local API port: {e}"));
    let addr = listener
        .local_addr()
        .expect("failed to read local API address");
    let port = addr.port();
    tracing::info!(%addr, "Axum API server started");

    // 后台运行 Axum 服务；panic 时由 `app.run()` 的 propagate_panic 行为处理。
    let axum_handle = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(?error, "Axum server stopped unexpectedly");
        }
    });

    // --- Tauri -----------------------------------------------------------------
    tauri::Builder::<tauri::Wry>::new()
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            app.manage(port);
            Ok(())
        })
        .run(tauri::generate_context!())
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

/// 组合根：把连接池装配成北向网关所需的全部服务。
///
/// 每个南向适配器（`SqlxTaskRepository` / `SqlxNoteRepository` /
/// `SqlxTransactionManager`）各自持有一个 `Pool` 句柄——`SqlitePool` 内部即 `Arc`，
/// 这里的 `clone` 只是原子计数 +1，开销可忽略，且三个适配器必须各持一份，无法再少。
/// 把这段"知道所有具体类型"的装配收口到本函数，使 `run()` 只做流程编排。
fn build_app_state(
    pool: south::db::Pool,
    config: shared::AppConfig,
) -> north::AppState {
    let task_repo = Arc::new(south::SqlxTaskRepository::new(pool.clone()));
    let note_repo = Arc::new(south::SqlxNoteRepository::new(pool.clone()));
    let category_repo = Arc::new(south::SqlxCategoryRepository::new(pool.clone()));
    let tx_manager = Arc::new(south::SqlxTransactionManager::new(pool));

    let tasks: Arc<dyn application::TaskUseCase> =
        Arc::new(application::TaskService::new(task_repo, tx_manager));
    let notes: Arc<dyn application::NoteUseCase> =
        Arc::new(application::NoteService::new(note_repo));
    let categories: Arc<dyn application::CategoryUseCase> =
        Arc::new(application::CategoryService::new(category_repo));

    north::AppState {
        tasks,
        notes,
        categories,
        config,
    }
}
