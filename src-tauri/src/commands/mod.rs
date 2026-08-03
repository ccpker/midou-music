// ════════════════════════════════════════════════
// 模块: commands（子模块入口）
// 路径: src-tauri/src/commands/mod.rs
// ────────────────────────────────────────────
// 功能: 聚合所有 Tauri 命令模块
// 原则: 所有 #[tauri::command] 必须注册到这里
// ════════════════════════════════════════════════

pub mod kugou_login;
pub mod play;
pub mod search;
pub mod window;
