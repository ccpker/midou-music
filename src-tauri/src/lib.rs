// ════════════════════════════════════════════════
// 模块: lib.rs — 米豆音乐 v2.0
// 路径: src-tauri/src/lib.rs
// ────────────────────────────────────────────
// 架构: Tauri v2 + React + rodio + KuGouMusicApi sidecar
// 酷狗全链路 → sidecar (localhost:6521)
// 酷我/B站 → Rust 原生
// ════════════════════════════════════════════════

mod audio;
mod commands;
mod db;
mod debug_log;
mod platform;
mod types;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use types::AppState;

/// 前端调用的日志命令
#[tauri::command]
fn debug_log_write(level: &str, tag: &str, msg: &str) {
    match level {
        "error" => crate::debug_log::error(tag, msg),
        "warn" => crate::debug_log::warn(tag, msg),
        _ => crate::debug_log::info(tag, msg),
    }
}

/// sidecar 健康检查 + 重启
#[tauri::command]
async fn sidecar_health() -> Result<serde_json::Value, String> {
    use crate::platform::kugou_sidecar;
    let alive = kugou_sidecar::health_check().await;
    Ok(serde_json::json!({"alive": alive}))
}

#[tauri::command]
async fn sidecar_restart() -> Result<serde_json::Value, String> {
    use crate::platform::kugou_sidecar;
    kugou_sidecar::restart_sidecar().await?;
    Ok(serde_json::json!({"ok": true}))
}

// ════════════════════════════════════════════════

#[tauri::command]
fn audio_play(state: tauri::State<'_, Arc<AppState>>, url: String) -> Result<(), String> {
    debug_log::info("audio", &format!("请求播放: {}", url));
    state.audio_player.play(url)
}

#[tauri::command]
fn audio_pause(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.audio_player.pause()
}

#[tauri::command]
fn audio_resume(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.audio_player.resume()
}

#[tauri::command]
fn audio_stop(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.audio_player.stop()
}

#[tauri::command]
fn audio_state(state: tauri::State<'_, Arc<AppState>>) -> Result<audio::AudioState, String> {
    state.audio_player.get_state()
        .ok_or_else(|| "获取播放状态超时".to_string())
}

#[tauri::command]
fn audio_set_volume(state: tauri::State<'_, Arc<AppState>>, volume: f32) -> Result<(), String> {
    state.audio_player.set_volume(volume)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ★ 必须先初始化日志，再创建 AudioHandle（它的日志需要走 debug_log）
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("midou-music");
    std::fs::create_dir_all(&app_dir)
        .expect("无法创建应用目录");
    let log_path = app_dir.join("debug.log");
    debug_log::init(log_path.clone());

    let audio_player = audio::AudioHandle::new()
        .expect("初始化音频设备失败 — 请检查声卡/驱动");

    // 启动 KuGouMusicApi sidecar
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    // 开发模式: 项目根目录（cargo 运行时 exe 在 target/debug/ 里）
    // 但在 Tauri dev 里 resources 可能不可用，读 Cargo manifest dir
    let sidecar_base = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            // 不是 cargo 运行时，用 exe 目录
            exe_dir.clone()
        });

    debug_log::info("startup", &format!("v2.0 sidecar 启动, 基目录={}", sidecar_base.display()));
    if let Err(e) = platform::kugou_sidecar::spawn_sidecar(&sidecar_base) {
        debug_log::error("startup", &format!("Sidecar 启动失败: {e}"));
        eprintln!("⚠️  Sidecar 启动失败: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let db_path = app_dir.join("midou-music.db");
            let db = rusqlite::Connection::open(&db_path)
                .map_err(|e| format!("打开数据库失败: {e}"))?;
            db::init_db(&db)?;

            // kugou_auth 初始为空 — 登录板块启动后从 Store 恢复并同步到内存
            let kugou_auth = types::KugouAuth::default_fallback();

            app.manage(Arc::new(AppState {
                client: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .expect("创建 HTTP 客户端失败"),
                db: std::sync::Mutex::new(db),
                kugou_auth: std::sync::Mutex::new(kugou_auth),
                audio_player,
            }));

            // 后台等待 sidecar 就绪
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_handle = app_handle;
                if let Err(e) = crate::platform::kugou_sidecar::wait_ready().await {
                    crate::debug_log::error("sidecar", &format!("就绪失败: {e}"));
                    let _ = app_handle.emit("sidecar_status", serde_json::json!({"ready": false, "error": e}));
                } else {
                    let _ = app_handle.emit("sidecar_status", serde_json::json!({"ready": true}));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search::search,
            commands::play::play_url,
            audio_play,
            audio_pause,
            audio_resume,
            audio_stop,
            audio_state,
            audio_set_volume,
            commands::window::open_player,
            commands::window::close_player,
            commands::window::is_player_open,
            commands::window::emit_play_state,
            commands::kugou_login::kugou_qr_key,
            commands::kugou_login::kugou_qr_check,
            commands::kugou_login::kugou_set_auth,
            commands::kugou_login::kugou_logout,
            commands::kugou_login::kugou_user_info,
            commands::kugou_login::kugou_playlists,
            commands::kugou_login::kugou_recent_plays,
            commands::kugou_login::kugou_user_songs,
            commands::kugou_login::kugou_playlist_songs,
            commands::kugou_login::kugou_register_device,
            sidecar_health,
            sidecar_restart,
            debug_log_write,
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
