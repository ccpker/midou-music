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

fn get_auth_opt(state: &tauri::State<'_, Arc<AppState>>) -> Option<crate::types::KugouAuth> {
    state.kugou_auth.lock().ok().map(|g| g.clone())
}

/// 获取播放 URL
#[tauri::command]
pub async fn play_url(
    song_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PlayUrlResult, String> {
    crate::debug_log::info("play_cmd", &format!("song_id={song_id}"));

    if song_id.starts_with("au") || song_id.starts_with("bv") {
        let v = crate::platform::bili::play_url(&state.client, &song_id, "high").await?;
        let u = v.get("url").and_then(|s| s.as_str()).unwrap_or("");
        Ok(PlayUrlResult {
            url: u.to_string(),
            source: "bili".to_string(),
            quality: "high".to_string(),
            ssa_code: None,
        })
    } else if song_id.starts_with("kugou:") {
        let auth = get_auth_opt(&state);
        let token = auth.as_ref().map(|a| a.token.as_str()).unwrap_or("");
        let userid = auth.as_ref().map(|a| a.userid.to_string()).unwrap_or_default();
        let dfid = auth.as_ref().map(|a| a.dfid.as_str()).unwrap_or("");
        // song_id 格式: kugou:HASH 或 kugou:HASH:AUDIO_ID 或 (旧) kugou:HASH|AlbumID
        let raw = song_id.strip_prefix("kugou:").unwrap_or(&song_id);
        // 先按 : 拆，取第一段；再按 | 裁掉旧格式的 AlbumID 后缀
        let hash = raw.split(':').next().unwrap_or(raw)
            .split('|').next().unwrap_or(raw);
        let audio_id: u64 = raw.split(':')
            .nth(1).and_then(|s| s.split('|').next())
            .and_then(|s| s.parse().ok()).unwrap_or(0);
        match crate::platform::kugou_sidecar::song_url(hash, audio_id, token, &userid, dfid).await {
            Ok(url) => {
                crate::debug_log::info("play_cmd", &format!(
                    "完成: source=kugou, url长度={}", url.len()
                ));
                Ok(PlayUrlResult {
                    url,
                    source: "kugou".to_string(),
                    quality: "high".to_string(),
                    ssa_code: None,
                })
            }
            Err(e) => {
                crate::debug_log::error("play_cmd", &format!(
                    "kugou song_url 失败: hash={hash}, err={e}"
                ));
                Err(if e.contains("no url in") {
                    "酷狗: 这首歌需要付费或版权受限，无法播放".into()
                } else {
                    e
                })
            }
        }
    } else {
        let v = crate::platform::kuwo::play_url(&state.client, &song_id, "320kmp3").await?;
        let u = v.get("url").and_then(|s| s.as_str()).unwrap_or("");
        Ok(PlayUrlResult {
            url: u.to_string(),
            source: "kuwo".to_string(),
            quality: "high".to_string(),
            ssa_code: None,
        })
    }
}
