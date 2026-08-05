//! CLI entry point for the Tauri application.
//!
//! Simply delegates to [`lib::run`], which starts both the embedded Axum HTTP
//! server and the Tauri event loop.

// Prevents additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    axum_tauri_vue_app_lib::run();
}
