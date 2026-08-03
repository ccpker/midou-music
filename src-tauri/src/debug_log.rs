// ════════════════════════════════════════════════
// 模块: debug_log
// 路径: src-tauri/src/debug_log.rs
// ────────────────────────────────────────────
// 功能: 简化文件日志，写到 debug.log
// ════════════════════════════════════════════════

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 初始化日志路径（调用一次）
pub fn init(path: PathBuf) {
    let mut guard = LOG_FILE.lock().unwrap();
    *guard = Some(path);
}

/// 追加一行日志
pub fn log(level: &str, tag: &str, msg: &str) {
    let guard = LOG_FILE.lock().unwrap();
    let path = match guard.as_ref() {
        Some(p) => p,
        None => return,
    };
    let now = chrono_lite();
    let line = format!("[{now}] [{level}] [{tag}] {msg}\n", now = now, level = level, tag = tag, msg = msg);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn info(tag: &str, msg: &str) { log("INFO", tag, msg); }
pub fn warn(tag: &str, msg: &str) { log("WARN", tag, msg); }
pub fn error(tag: &str, msg: &str) { log("ERROR", tag, msg); }

fn chrono_lite() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24 + 8; // UTC+8
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    let ms = now.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, s, ms)
}
