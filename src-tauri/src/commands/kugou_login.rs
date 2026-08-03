// ═══════════════════════════════════════════════════
// kugou_login.rs — Tauri 命令层（sidecar 版）
//
// 架构变更（2026-07-31）:
//   酷狗全链路 → 走 KuGouMusicApi sidecar (localhost:6521)
//   登录/搜索/歌单/播放/SSA/register_dev 全部由 sidecar 处理
//   Rust 端只做 HTTP 代理 + 类型转换
//
// Store(前端) = 唯一持久化真相源
// Rust 内存 = 缓存一份，前端通过 kugou_set_auth 推过来
// ═══════════════════════════════════════════════════

use std::sync::Arc;
use tauri::Emitter;
use crate::platform::kugou_sidecar;
use crate::types::{AppState, KugouAuth};

// ── 1️⃣ 获取二维码 → sidecar ──────────────────────────────

#[tauri::command]
pub async fn kugou_qr_key(
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let (key, img) = kugou_sidecar::login_qr_key().await?;
    Ok(serde_json::json!({ "qrcode_key": key, "qrcode_img": img }))
}

// ── 2️⃣ 轮询 → sidecar ──────────────────────────────────

#[tauri::command]
pub async fn kugou_qr_check(
    _state: tauri::State<'_, Arc<AppState>>,
    qrcode_key: String,
) -> Result<serde_json::Value, String> {
    let (status, auth_info) = kugou_sidecar::login_qr_check(&qrcode_key).await?;
    if let Some((token, userid, nickname)) = auth_info {
        Ok(serde_json::json!({
            "status": 4,
            "token": token,
            "userid": userid,
            "nickname": nickname,
        }))
    } else {
        Ok(serde_json::json!({ "status": status }))
    }
}

// ── 3️⃣ 前端推 token → Rust 内存 ────────────────────────

#[tauri::command]
pub fn kugou_set_auth(
    state: tauri::State<'_, Arc<AppState>>,
    token: String,
    userid: u64,
) -> Result<(), String> {
    let uid = userid;
    let auth = KugouAuth {
        logged_in: !token.is_empty(),
        token,
        userid: uid,
        ..KugouAuth::default_fallback()
    };
    let mut mem = state.kugou_auth.lock().map_err(|e| format!("锁: {e}"))?;
    *mem = auth;
    crate::debug_log::info("kugou_sidecar", &format!("kugou_set_auth: userid={}", uid));
    Ok(())
}

// ── 4️⃣ 登出 ──────────────────────────────────────────

#[tauri::command]
pub async fn kugou_logout(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    {
        let mut mem = state.kugou_auth.lock().map_err(|e| format!("Auth锁: {e}"))?;
        *mem = KugouAuth::default_fallback();
    }
    let _ = app_handle.emit("kugou_auth_updated", serde_json::json!({ "logged_in": false }));
    crate::debug_log::info("kugou_sidecar", "kugou_logout 已登出");
    Ok(())
}

// ── 5️⃣ 用户信息 → sidecar ────────────────────────────

#[tauri::command]
pub async fn kugou_user_info(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = get_auth(&state)?;
    kugou_sidecar::user_info(&auth.token, &auth.userid.to_string()).await
}

// ── 设备注册 → sidecar ───────────────────────────

#[tauri::command]
pub async fn kugou_register_device(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let dfid = kugou_sidecar::register_device().await?;
    // 同步写入内存
    if let Ok(mut mem) = state.kugou_auth.lock() {
        mem.dfid = dfid.clone();
    }
    Ok(dfid)
}

// ── 6️⃣ 歌单列表 → sidecar ────────────────────────────

#[tauri::command]
pub async fn kugou_playlists(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = get_auth(&state)?;
    let v = kugou_sidecar::user_playlists(&auth.token, &auth.userid.to_string(), &auth.dfid).await?;
    // sidecar 返回 data.info 数组，字段 list_create_listid → listid
    let list: Vec<serde_json::Value> = v["data"]["info"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|mut p| {
            // rename list_create_listid → listid
            if let Some(lid) = p.get("list_create_listid").and_then(|v| v.as_u64()) {
                p["listid"] = serde_json::Value::Number(lid.into());
            }
            if p.get("listid").is_some() { Some(p) } else { None }
        })
        .collect();
    Ok(serde_json::Value::Array(list))
}

// ── 7️⃣ 歌单歌曲 → sidecar ────────────────────────────

#[tauri::command]
pub async fn kugou_playlist_songs(
    state: tauri::State<'_, Arc<AppState>>,
    list_id: u64,
) -> Result<serde_json::Value, String> {
    let auth = get_auth(&state)?;
    crate::debug_log::info("kugou", &format!("歌单歌曲请求: list_id={}", list_id));
    let v = kugou_sidecar::playlist_songs(list_id, &auth.token, &auth.userid.to_string(), &auth.dfid).await?;
    let count = v["data"]["count"].as_u64().unwrap_or(0);
    crate::debug_log::info("kugou", &format!("歌单歌曲完成: list_id={}, count={}", list_id, count));
    Ok(v)
}

// ── 8️⃣ 最近播放 → sidecar ────────────────────────────

#[tauri::command]
pub async fn kugou_recent_plays(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = get_auth(&state)?;
    let v = kugou_sidecar::favorite_songs(&auth.token, &auth.userid.to_string()).await?;
    let list = v["data"]["list"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(serde_json::Value::Array(list))
}

// ── 9️⃣ 用户收藏 → sidecar ────────────────────────────

#[tauri::command]
pub async fn kugou_user_songs(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let auth = get_auth(&state)?;
    let v = kugou_sidecar::favorite_songs(&auth.token, &auth.userid.to_string()).await?;
    let list = v["data"]["list"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(serde_json::Value::Array(list))
}

// ── 辅助: 从 AppState 取 auth ──────────────────────

fn get_auth(state: &tauri::State<'_, Arc<AppState>>) -> Result<KugouAuth, String> {
    let guard = state.kugou_auth.lock().map_err(|e| format!("锁: {e}"))?;
    let auth = guard.clone();
    if auth.token.is_empty() {
        return Err("未登录".into());
    }
    Ok(auth)
}
