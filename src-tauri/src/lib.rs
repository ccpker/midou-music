// ════════════════════════════════════════════════
// 模块: lib.rs
// 路径: src-tauri/src/lib.rs
// ────────────────────────────────────────────
// 功能: Tauri 应用逻辑 — 注册所有 #[tauri::command]
// 输入: 前端 invoke() 调用
// 输出: JSON (自动序列化)
// 依赖: platform/kuwo.rs, reqwest
// 测试: cargo check 通过即可
// 接口: fn run() — Tauri Builder 入口
// ════════════════════════════════════════════════

mod platform;

use serde::Serialize;
use std::sync::Arc;
use tauri::Manager;

// ── 数据结构 ──

#[derive(Serialize, Clone)]
pub struct Song {
    pub song_id: String,
    pub name: String,
    pub singer: String,
    pub album: String,
    pub duration: u32,
    pub source: String,
}

#[derive(Serialize)]
pub struct PlayUrlResult {
    pub url: String,
    pub source: String,
    pub quality: String,
}

// ── 全局状态 ──

struct AppState {
    client: reqwest::Client,
}

// ── Tauri Commands ──

#[tauri::command]
async fn search(
    keyword: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<Song>, String> {
    platform::kuwo::search(&state.client, &keyword, 0, 20).await
}

#[tauri::command]
async fn play_url(
    song_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PlayUrlResult, String> {
    let v = platform::kuwo::play_url(&state.client, &song_id, "320kmp3").await?;
    Ok(PlayUrlResult {
        url: v["url"].as_str().unwrap_or("").to_string(),
        source: "kuwo".to_string(),
        quality: v["quality"].as_str().unwrap_or("320kmp3").to_string(),
    })
}

// ── 入口 ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = Arc::new(AppState {
                client: reqwest::Client::new(),
            });
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search, play_url])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
