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
use tauri::{Emitter, Manager};
use types::AppState;

/// 判断路径是否为支持的音频文件（用于双击文件识别）
fn is_audio_path(s: &str) -> bool {
    let lower = s.to_lowercase();
    ["mp3", "flac", "m4a", "wav", "aac", "ogg"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 首次启动时解析命令行参数，拿到双击的文件路径
    let first_open_file: Option<String> = std::env::args()
        .skip(1)
        .find(|a| is_audio_path(a));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        // ── 单实例 + 文件关联：双击文件时把路径传给已开着的窗口 ──
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // argv 是启动参数，双击音频文件时包含文件路径
            if let Some(path) = argv.iter().find(|a| is_audio_path(a)) {
                crate::debug_log::info("file_open", &format!("双击打开文件: {path}"));
                // 通过事件通知前端播放
                let _ = app.emit("file_open", path.clone());
            }
        }))
        // ── 初始化 ──────────────────────────────
        .setup(move |app| {
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

            // 首次启动就带了音频文件参数（双击启动）→ 等主窗口就绪后通知前端
            if let Some(path) = first_open_file {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 等待主窗口 ready（前端监听事件注册完成）
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    crate::debug_log::info("file_open", &format!("首次启动打开文件: {path}"));
                    let _ = handle.emit("file_open", path);
                });
            }

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
            commands::window::resize_player,
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
            // 酷狗 VIP 签到
            commands::kugou_vip::kugou_sign_vip,
            commands::kugou_vip::kugou_vip_status,
            commands::kugou_vip::kugou_personal_fm,
            commands::kugou_vip::kugou_watch_ad,
            // 歌词
            commands::lyric::get_lyric,
            // 下载
            commands::download::download_song,
            commands::download::get_qualities,
            // 本地音乐库
            commands::library::scan_library,
            // 调试日志（前端调用）
            debug_log_write,
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
