//! CLI entry point for the Tauri application。
//!
//! 支持 `--export-types` 标志生成前端共享 TypeScript 类型文件。

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--export-types".to_string()) {
        axum_tauri_vue_app_lib::specta_export::export_types()
            .expect("类型导出失败");
        return;
    }
    axum_tauri_vue_app_lib::run();
}
