// ════════════════════════════════════════════════
// 模块: commands::search
// 路径: src-tauri/src/commands/search.rs
// ────────────────────────────────────────────
// 功能: 搜索歌曲（多平台路由）
//
// 参数:
//   keyword  - 搜索关键词
//   source  - 音源标识（默认 "kuwo"）
//     "kuwo" → 酷我音乐
//     "bili" → B站音频
//     "kugou" → 酷狗音乐（v0.4）
//     "qq"    → QQ音乐（待接入）
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::{AppState, Song};

/// 搜索歌曲
///
/// 调用路径: App.vue → useSearch() → invoke('search')
#[tauri::command]
pub async fn search(
    keyword: String,
    source: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<Song>, String> {
    match source.as_str() {
        "bili" => crate::platform::bili::search(&state.client, &keyword, 0, 20).await,
        "kugou" => {
            let auth = state
                .kugou_auth
                .lock()
                .map_err(|e| format!("kugou_auth锁失败: {}", e))?
                .clone();
            crate::platform::kugou::search(&state.client, &keyword, 0, &auth).await
        }
        // TODO: "qq"
        _ => crate::platform::kuwo::search(&state.client, &keyword, 0, 20).await,
    }
}
