mod db;
mod models;
mod routes;

use tauri::Manager;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

#[tokio::main]
/// 桌面程序的入口：先准备 API/数据库，再交给 Tauri 运行事件循环。
pub async fn run() {
    tracing_subscriber::fmt::init();

    // 初始化数据库
    let pool = db::init_pool()
        .await
        .expect("Failed to initialize SQLite database");
    let state = routes::AppState { db: pool };

    // 创建 Axum 路由
    let app = routes::create_router(state).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    // 绑定到随机端口
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind local API port");
    let addr = listener
        .local_addr()
        .expect("Failed to read local API address");
    println!("Axum server running on http://{}", addr);
    let port = addr.port();

    // 在后台启动 Axum，同时运行 Tauri
    let axum_handle = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(?error, "Axum server stopped unexpectedly");
        }
    });

    // 启动 Tauri，并将端口传递给前端（通过环境变量或 Tauri 状态）
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_api_port])
        .setup(move |app| {
            // 将端口保存到 App 的状态中，方便前端命令调用
            app.manage(port);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // 正常情况下不会到达这里，Tauri 会阻塞
    axum_handle.abort();
}
#[tauri::command]
fn get_api_port(state: tauri::State<'_, u16>) -> u16 {
    *state
}
