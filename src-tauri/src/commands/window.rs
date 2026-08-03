// ════════════════════════════════════════════════
// 模块: commands::window
// 路径: src-tauri/src/commands/window.rs
// ────────────────────────────────────────────
// 功能: 播放条独立窗口管理
// 标注:
//   - 窗口 label: "player"
//   - 入口页面: player.html（dist 内）
//   - 位置: 跟随主窗口，底部居中，480×120
// 权限依赖: capabilities/default.json 中 window.* + webview.*
//
// 事件通信:
//   - emit('play_state', PlayState)     → PlayerBar 接收
//   - emit('player_cmd', String)        → App.vue 接收（上一首/下一首）
//   - emit('player_position', ...)      → App.vue 接收（进度同步，可选）
// ════════════════════════════════════════════════

use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};
use crate::types::PlayState;

// ── 常量 ────────────────────────────────────────

const PLAYER_LABEL: &str = "player";
const PLAYER_WIDTH: f64 = 480.0;
const PLAYER_HEIGHT: f64 = 160.0; // 增加高度容纳调试面板
const PLAYER_OFFSET_BOTTOM: i32 = 20; // 距主窗口底部像素

// ── 工具函数 ─────────────────────────────────────

/// 获取主窗口在屏幕上的绝对坐标
fn main_window_rect(app: &AppHandle) -> Result<(i32, i32, u32, u32), String> {
    let win = app
        .get_webview_window("main")
        .ok_or("主窗口未找到")?;
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    Ok((pos.x, pos.y, size.width, size.height))
}

// ── Commands ─────────────────────────────────────

/// 打开播放条窗口（若已打开则忽略）
///
/// 调用路径: App.vue → invoke('open_player')
#[tauri::command]
pub async fn open_player(app: AppHandle) -> Result<(), String> {
    crate::debug_log::info("window", "open_player 被调用");
    if app.get_webview_window(PLAYER_LABEL).is_some() {
        crate::debug_log::info("window", "播放条窗口已存在，跳过");
        return Ok(());
    }

    let (mx, my, mw, mh) = main_window_rect(&app)?;

    // 播放条: 主窗口下方，明显可见的位置
    let x = mx + ((mw as i32 - PLAYER_WIDTH as i32) / 2);
    let y = my + mh as i32 + 10; // 主窗口下方10像素

    crate::debug_log::info("window", &format!("创建播放条窗口: x={x}, y={y}"));
    match WebviewWindowBuilder::new(&app, PLAYER_LABEL, tauri::WebviewUrl::App("player.html".into()))
        .title("🎵 播放")
        .inner_size(PLAYER_WIDTH, PLAYER_HEIGHT)
        .position(x as f64, y as f64)
        .decorations(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .build() {
        Ok(_) => {
            crate::debug_log::info("window", "播放条窗口创建成功");
            Ok(())
        }
        Err(e) => {
            crate::debug_log::error("window", &format!("创建播放条窗口失败: {e}"));
            Err(format!("创建播放条窗口失败: {e}"))
        }
    }
}

/// 关闭播放条窗口
///
/// 调用路径: PlayerBar.vue → closePlayer()
#[tauri::command]
pub async fn close_player(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(PLAYER_LABEL) {
        win.close().map_err(|e| format!("关闭窗口失败: {e}"))?;
    }
    Ok(())
}

/// 播放条窗口是否已打开
#[tauri::command]
pub async fn is_player_open(app: AppHandle) -> Result<bool, String> {
    Ok(app.get_webview_window(PLAYER_LABEL).is_some())
}

/// 发送播放状态（emit 到所有窗口）
#[tauri::command]
pub async fn emit_play_state(app: AppHandle, state: PlayState) -> Result<(), String> {
    crate::debug_log::info("window", &format!("emit_play_state: song={}, url={}", 
        state.song.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
        state.url.as_ref().map(|u| u.as_str()).unwrap_or("无")
    ));
    app.emit("play_state", &state)
        .map_err(|e| format!("emit 失败: {e}"))
}
