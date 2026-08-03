// ════════════════════════════════════════════════
// 模块: commands::search
// 路径: src-tauri/src/commands/search.rs
// ────────────────────────────────────────────
// 功能: 搜索歌曲（多平台路由）
//
// 酷狗 → sidecar (localhost:6521)
// 酷我/B站 → Rust 原生
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::{AppState, Song};
use crate::debug_log;

/// 搜索歌曲
#[tauri::command]
pub async fn search(
    keyword: String,
    source: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<Song>, String> {
    let start = std::time::Instant::now();
    debug_log::info("search", &format!("开始: source={}, keyword={}", source, keyword));

    let result = match source.as_str() {
        "bili" => crate::platform::bili::search(&state.client, &keyword, 0, 20).await,
        "kugou" => {
            let auth = get_auth_opt(&state);
            let tok = auth.as_ref().map(|a| a.token.as_str()).unwrap_or("");
            let uid = auth.as_ref().map(|a| a.userid.to_string()).unwrap_or_default();
            search_kugou_sidecar(&keyword, tok, &uid).await
        }
        _ => crate::platform::kuwo::search(&state.client, &keyword, 0, 20).await,
    };

    match &result {
        Ok(songs) => debug_log::info("search", &format!("完成: {} 首, 耗时 {:?}", songs.len(), start.elapsed())),
        Err(e) => debug_log::error("search", &format!("失败: {}, 耗时 {:?}", e, start.elapsed())),
    }
    result
}

fn get_auth_opt(state: &tauri::State<'_, Arc<AppState>>) -> Option<crate::types::KugouAuth> {
    state.kugou_auth.lock().ok().map(|g| g.clone())
}

/// 通过 sidecar 搜索酷狗，转成 Song 列表
async fn search_kugou_sidecar(keyword: &str, token: &str, userid: &str) -> Result<Vec<Song>, String> {
    let v = crate::platform::kugou_sidecar::search(keyword, 1, token, userid).await?;
    // sidecar 返回: { status:1, data:{ lists:[{SongName,FileHash,SingerName,AlbumName,Duration,...}] } }
    let lists = v["data"]["lists"].as_array()
        .ok_or_else(|| format!("sidecar search: unexpected format: {:.200}", v))?;

    let mut songs = Vec::with_capacity(lists.len());
    for item in lists {
        let hash = item["FileHash"].as_str().unwrap_or("");
        let name = item["FileName"].as_str().unwrap_or("??").to_string();
        let singer = item["SingerName"].as_str().unwrap_or("??").to_string();
        let album = item["AlbumName"].as_str().unwrap_or("").to_string();
        let duration = item["Duration"].as_u64().unwrap_or(0) as u32;

        // 搜索不拼 AlbumID——它不是 audio_id，传了反而误导 song_url
        let song_id = format!("kugou:{}", hash);

        songs.push(Song {
            song_id,
            name,
            singer,
            album,
            duration,
            source: "kugou".to_string(),
            cover_url: None,
            lyric_id: None,
        });
    }
    Ok(songs)
}
