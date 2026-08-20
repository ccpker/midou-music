// ════════════════════════════════════════════════
// 模块: commands::lyric
// 路径: src-tauri/src/commands/lyric.rs
// ────────────────────────────────────────────
// 功能: 获取歌词（LRC 文本）
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::AppState;

/// 获取歌词
///
/// 调用路径: 前端 → invoke('get_lyric', { name, singer, duration, source, songId })
///
/// 返回: LRC 格式歌词文本；找不到返回空字符串（不抛错，前端静默降级）
#[tauri::command]
pub async fn get_lyric(
    name: String,
    singer: String,
    duration: u32,
    source: String,
    song_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::debug_log::info(
        "lyric",
        &format!("get_lyric: name={name}, singer={singer}, source={source}, song_id={song_id}"),
    );

    match crate::platform::lyric::fetch_lyric(
        &state.client,
        &name,
        &singer,
        duration,
        &source,
        &song_id,
    )
    .await
    {
        Ok(lrc) => Ok(lrc),
        Err(e) => {
            // 找不到歌词不算致命错误，返回空字符串让前端降级显示
            crate::debug_log::info("lyric", &format!("歌词获取失败(返回空): {e}"));
            Ok(String::new())
        }
    }
}
