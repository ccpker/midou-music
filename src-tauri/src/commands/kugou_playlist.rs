// ════════════════════════════════════════════════
// 模块: commands::kugou_playlist
// 路径: src-tauri/src/commands/kugou_playlist.rs
// ────────────────────────────────────────────
// 功能: 酷狗歌单（我的收藏/歌单）命令
//   kugou_playlists       - 获取用户歌单列表
//   kugou_playlist_songs  - 获取歌单内歌曲
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::{AppState, Song};

/// 获取用户歌单列表
///
/// 返回 [{listid, name, global_collection_id, song_count}, ...]
#[tauri::command]
pub async fn kugou_playlists(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    let playlists = crate::platform::kugou::fetch_playlists(&state.client, &auth).await?;

    // 精简字段，返回前端需要的
    Ok(playlists
        .into_iter()
        .map(|p| {
            let listid = p
                .get("listid")
                .or_else(|| p.get("list_create_listid"))
                .or_else(|| p.get("specialid"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let global_id = p
                .get("global_collection_id")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let name = p
                .get("listname")
                .or_else(|| p.get("name"))
                .or_else(|| p.get("specialname"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let song_count = p
                .get("m_count")
                .or_else(|| p.get("count"))
                .or_else(|| p.get("song_count"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "listid": listid,
                "global_collection_id": global_id,
                "name": name,
                "song_count": song_count,
            })
        })
        .collect())
}

/// 获取歌单内歌曲
///
/// 参数 global_collection_id: 歌单全局 ID（如 collection_3_1514557990_2_0）
#[tauri::command]
pub async fn kugou_playlist_songs(
    global_collection_id: String,
    page: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<Song>, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    crate::platform::kugou::fetch_playlist_songs(
        &state.client,
        &auth,
        &global_collection_id,
        page.unwrap_or(0),
    )
    .await
}
