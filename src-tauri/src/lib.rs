//! Axum + Tauri integration entry point.
//!
//! This is the crate root. It initializes the database, wires up the API
//! router, starts the embedded HTTP server on a random localhost port, and
//! hands control to Tauri's event loop. The port is stored in Tauri state so
//! the Vue frontend can discover it via the `get_api_port` invoke command.

pub(crate) mod api;
pub(crate) mod db;
pub(crate) mod models;

use api::create_router;
use api::AppState;
use tauri::Manager;
use tokio::net::TcpListener;

/// Start the embedded Axum server and the Tauri app in the same process.
///
/// 1. Initialise the SQLite connection pool (and run migrations).
/// 2. Bind an Axum router to a random localhost port.
/// 3. Spawn the HTTP server on a background Tokio task.
/// 4. Run the Tauri event loop, exposing `get_api_port` to the frontend.
///
/// The function never returns in the normal flow because `app.run()` blocks.
#[tokio::main]
pub async fn run() {
    // Initialise structured logging (written to stdout/stderr).
    tracing_subscriber::fmt::init();

    // --- Database -----------------------------------------------------------
    let pool = db::init_pool()
        .await
        .expect("Failed to initialize SQLite database");
    let state = AppState { db: pool };

    // --- API server ---------------------------------------------------------
    let app = create_router(state);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind local API port");
    let addr = listener
        .local_addr()
        .expect("Failed to read local API address");
    println!("Axum server running on http://{}", addr);
    let port = addr.port();

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
        .run(context)   // 确保 tauri.conf.json 存在且内容正确
        .unwrap_or_else(|e| {              // 替换 expect，更灵活
            panic!("error while running tauri application: {}", e);
        });
    // Abort the HTTP server (reached only if the Tauri event loop is
    // artificially terminated, e.g. in tests).
    axum_handle.abort();
}

/// Return the port the embedded Axum server is listening on.
///
/// Called from the Vue frontend via `invoke("get_api_port")`.
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
