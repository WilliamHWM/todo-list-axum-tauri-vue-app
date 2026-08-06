//! Axum + Tauri integration entry point.
//!
//! Boot order:
//! 1. Initialise tracing (optional JSON output for log collectors).
//! 2. Load `AppConfig` from environment variables.
//! 3. Open the SQLite pool and run pending migrations.
//! 4. Bind the Axum router to a random localhost port (default) and spawn the
//!    HTTP server on a background Tokio task.
//! 5. Hand control to Tauri's event loop. The chosen port is stored in Tauri
//!    state so the Vue frontend can discover it via `get_api_port`.

pub(crate) mod api;
pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod error;
pub(crate) mod models;

use api::create_router;
use api::AppState;
use tauri::Manager;
use tokio::net::TcpListener;

/// Start the embedded Axum server and the Tauri app in the same process.
///
/// The function never returns in the normal flow because `app.run()` blocks.
#[tokio::main]
pub async fn run() {
    // --- Configuration -----------------------------------------------------
    let config = config::AppConfig::from_env();
    init_tracing(&config);
    tracing::info!(
        log_level = %config.log_level,
        log_format = %config.log_format,
        "application configuration loaded"
    );

    // --- Database -----------------------------------------------------------
    let pool = db::init_pool(&config).await.unwrap_or_else(|e| {
        panic!("failed to initialize SQLite database: {e}");
    });
    tracing::info!("database pool initialized and migrations applied");

    let state = AppState {
        db: pool,
        config: config.clone(),
    };

    // --- API server ---------------------------------------------------------
    let app = create_router(state);
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

    // --- Tauri ------------------------------------------------------------
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
    // Abort the HTTP server (reached only if the Tauri event loop is
    // artificially terminated, e.g. in tests).
    axum_handle.abort();
}

/// Initialise structured logging.
///
/// The filter respects the `RUST_LOG` env var first, falling back to the
/// `APP_LOG_LEVEL` config. Set `APP_LOG_FORMAT=json` to emit JSON lines for
/// log aggregation tools.
fn init_tracing(config: &config::AppConfig) {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.log_format == "json" {
        let _ = fmt().json().with_env_filter(filter).try_init();
    } else {
        let _ = fmt().with_env_filter(filter).try_init();
    }
}

/// Return the port the embedded Axum server is listening on.
///
/// Called from the Vue frontend via `invoke("get_api_port")`.
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
