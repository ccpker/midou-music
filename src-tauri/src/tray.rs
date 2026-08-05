// ════════════════════════════════════════════════
// 模块: tray
// 路径: src-tauri/src/tray.rs
// ────────────────────────────────────────────
// 功能: 系统托盘零件（后置初始化）
// 原则: 
//   - 前端初始化完成后才调用
//   - 不阻塞启动流程
//   - 只管托盘图标和菜单
// ════════════════════════════════════════════════

use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use crate::types::AppState;
use std::sync::Arc;

static TRAY_INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 初始化系统托盘（后置，前端调用）
/// 
/// 调用位置: 前端 App.tsx 初始化完成后调用
/// 示例: await invoke("init_tray")
#[tauri::command]
pub async fn init_tray(app: AppHandle) -> Result<(), String> {
    // 防止重复初始化
    if TRAY_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    
    // 创建菜单项
    let show_i = MenuItem::with_id(&app, "show", "显示主窗口", true, None::<&str>)
        .map_err(|e| format!("创建菜单项失败: {e}"))?;
    let quit_i = MenuItem::with_id(&app, "quit", "退出", true, None::<&str>)
        .map_err(|e| format!("创建菜单项失败: {e}"))?;
    
    // 创建菜单
    let menu = Menu::with_items(&app, &[&show_i, &quit_i])
        .map_err(|e| format!("创建菜单失败: {e}"))?;
    
    // 创建托盘图标
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "quit" => {
                    // 停止音频再退出
                    let state = app.state::<Arc<AppState>>();
                    let _ = state.audio_player.stop();
                    app.exit(0);
                }
                "show" => {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(&app)
        .map_err(|e| format!("创建托盘失败: {e}"))?;
    
    TRAY_INITIALIZED.store(true, std::sync::atomic::Ordering::SeqCst);
    crate::debug_log::info("tray", "托盘初始化成功");
    Ok(())
}

/// 主窗口关闭时隐藏到托盘（由前端调用）
#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
        crate::debug_log::info("tray", "主窗口隐藏到托盘");
    }
    Ok(())
}
