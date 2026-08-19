// ════════════════════════════════════════════════
// 模块: lib.rs
// 路径: src-tauri/src/lib.rs
// ────────────────────────────────────────────
// 功能: Tauri 应用入口编排
// 规则: 围绕 Tauri 官方骨架旋转（不碰 main.rs）
// 原则: 本文件只做模块组装，不含业务逻辑
// ════════════════════════════════════════════════

mod commands;
mod db;
mod debug_log;
mod platform;
mod types;

/// 前端调用的日志命令（暴露 debug_log 模块）
#[tauri::command]
fn debug_log_write(level: &str, tag: &str, msg: &str) {
    match level {
        "error" => crate::debug_log::error(tag, msg),
        "warn" => crate::debug_log::warn(tag, msg),
        _ => crate::debug_log::info(tag, msg),
    }
}

use std::sync::Arc;
use tauri::Manager;
use types::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // ── 初始化 ──────────────────────────────
        .setup(|app| {
            // 数据目录
            let app_dir = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("midou-music");
            std::fs::create_dir_all(&app_dir)
                .map_err(|e| format!("创建应用目录失败: {e}"))?;

            // 调试日志
            let log_path = app_dir.join("debug.log");
            debug_log::init(log_path.clone());
            debug_log::info("startup", &format!("应用启动, 日志文件={}", log_path.display()));

            // 数据库
            let db_path = app_dir.join("midou-music.db");
            let db = rusqlite::Connection::open(&db_path)
                .map_err(|e| format!("打开数据库失败: {e}"))?;
            db::init_db(&db)?;

            // 加载酷狗凭证
            let kugou_auth = db::load_kugou_auth(&db);

            // 注入全局状态
            app.manage(Arc::new(AppState {
                client: reqwest::Client::new(),
                db: std::sync::Mutex::new(db),
                kugou_auth: std::sync::Mutex::new(kugou_auth),
            }));

            Ok(())
        })
        // ── 命令注册 ────────────────────────────
        .invoke_handler(tauri::generate_handler![
            // 搜索
            commands::search::search,
            // 播放
            commands::play::play_url,
            // 窗口
            commands::window::open_player,
            commands::window::close_player,
            commands::window::is_player_open,
            commands::window::emit_play_state,
            // 酷狗扫码登录
            commands::kugou_login::kugou_qr_key,
            commands::kugou_login::kugou_qr_check,
            commands::kugou_login::kugou_save_qr_token,
            commands::kugou_login::kugou_auth_status,
            commands::kugou_login::kugou_logout,
            // 酷狗歌单
            commands::kugou_playlist::kugou_playlists,
            commands::kugou_playlist::kugou_playlist_songs,
            // 调试日志（前端调用）
            debug_log_write,
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
