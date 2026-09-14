// ════════════════════════════════════════════════
// 模块: commands::play
// 路径: src-tauri/src/commands/play.rs
// ────────────────────────────────────────────
// 功能: 获取歌曲播放 URL（多平台路由）
//
// song_id 格式约定:
//   纯数字  → 酷我 MUSIC_RID
//   au{MUSIC_ID}  → B站音乐区纯音频
//   bv{BVID}      → B站视频区音频版
//   kugou:{hash} → 酷狗
//   qq:*          → QQ音乐（待接入）
//
// 路由策略: 按 song_id 前缀自动识别音源
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::{AppState, PlayUrlResult};

/// 获取播放 URL
///
/// 调用路径: App.vue → handlePlay() → invoke('play_url')
#[tauri::command]
pub async fn play_url(
    song_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PlayUrlResult, String> {
    crate::debug_log::info("play_cmd", &format!("song_id={song_id}"));
    let (url, source) = if song_id.starts_with("local:") {
        // 本地文件：返回绝对路径，前端用 convertFileSrc 转 asset 协议
        let path = &song_id["local:".len()..];
        (path.to_string(), "local".to_string())
    } else if song_id.starts_with("au") || song_id.starts_with("bv") {
        // B站音频
        let v = crate::platform::bili::play_url(&state.client, &song_id, "high").await?;
        let u = v.get("url").and_then(|s| s.as_str()).unwrap_or("");
        (u.to_string(), "bili".to_string())
    } else if song_id.starts_with("kugou:") {
        // 酷狗
        let auth = state
            .kugou_auth
            .lock()
            .map_err(|e| format!("kugou_auth锁失败: {}", e))?
            .clone();
        let u = crate::platform::kugou::play_url(&state.client, &song_id, &auth, "128").await?;
        (u, "kugou".to_string())
    } else {
        // 默认为酷我
        let v = crate::platform::kuwo::play_url(&state.client, &song_id, "320kmp3").await?;
        let u = v.get("url").and_then(|s| s.as_str()).unwrap_or("");
        (u.to_string(), "kuwo".to_string())
    };

    crate::debug_log::info("play_cmd", &format!("完成: source={source}, url长度={}", url.len()));
    Ok(PlayUrlResult {
        url,
        source,
        quality: "high".to_string(),
    })
}
