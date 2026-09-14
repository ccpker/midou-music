// ════════════════════════════════════════════════
// 模块: commands::kugou_vip
// 路径: src-tauri/src/commands/kugou_vip.rs
// ────────────────────────────────────────────
// 功能: 酷狗签到领 VIP 命令
//   kugou_sign_vip    - 签到领 1 天 VIP
//   kugou_vip_status  - 查询 VIP 状态（含 tvip 权益）
// ════════════════════════════════════════════════

use std::sync::Arc;
use crate::types::AppState;

/// 签到领 1 天 VIP
///
/// 返回 { status: 1, error_code: 0, data: {...} }（签到成功）
#[tauri::command]
pub async fn kugou_sign_vip(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    crate::platform::kugou::sign_vip(&state.client, &auth).await
}

/// 查询当前账号 VIP 状态
///
/// 返回 get_union_vip 原始 JSON（busi_vip[] 里有 tvip 权益）
#[tauri::command]
pub async fn kugou_vip_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    crate::platform::kugou::get_vip_status(&state.client, &auth).await
}

/// 私人 FM 推荐（个性化推荐流）
///
/// 返回 Vec<Song>，可直接用 play_url 播放
#[tauri::command]
pub async fn kugou_personal_fm(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::Song>, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    crate::platform::kugou::personal_fm(&state.client, &auth).await
}

/// 看广告领 VIP 时长（循环最多 8 次，间隔 4 秒）
///
/// 返回 { total_hours, done_count, total_limit, detail, errors }
#[tauri::command]
pub async fn kugou_watch_ad(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = state
        .kugou_auth
        .lock()
        .map_err(|e| format!("kugou_auth锁失败: {}", e))?
        .clone();

    crate::platform::kugou::watch_ad(&state.client, &auth).await
}
