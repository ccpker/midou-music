// ════════════════════════════════════════════════
// 模块: main.rs
// 路径: src-tauri/src/main.rs
// ────────────────────────────────────────────
// 功能: Tauri 桌面入口
// 规则: 官方原话 — "don't modify this file, modify lib.rs instead"
// 依赖: app_lib (src-tauri/src/lib.rs)
// ════════════════════════════════════════════════

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    app_lib::run();
}
