// ════════════════════════════════════════════════
// 音楽自由 — 工具函数
// ────────────────────────────────────────────
// 功能:   日志、时间、文件名清理、URL编码等通用工具
// 输入:   (函数参数各异)
// 输出:   (各种返回值)
// 依赖:   无(纯std + std::io)
// 边界:   日志文件路径为exe同目录下的 logs/
// 备注:   log_info! 宏带 #[macro_export] 供全模块使用
// ════════════════════════════════════════════════

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ── 日志系统 ──

/// 获取日志根目录(优先读环境变量 MUSIC_LOG_DIR,否则用exe所在目录)
pub fn log_base_dir() -> PathBuf {
    std::env::var("MUSIC_LOG_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(log_exe_dir)
}

/// 获取exe所在目录
pub fn log_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
}

/// 生成时间戳 [YYYY-MM-DD HH:MM:SS.mmm]
pub fn timestamp() -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = ts.as_secs() + 8 * 3600;
    let millis = ts.subsec_millis();
    let days = secs / 86400;
    let t = secs % 86400;
    let h = t / 3600;
    let m = (t % 3600) / 60;
    let s = t % 60;
    let (y, mo, d) = days_to_date(days as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}.{millis:03}")
}

fn days_to_date(mut days: i64) -> (i64, u32, u32) {
    let mut y = 1970i64;
    loop {
        let dy = if is_leap(y) { 366 } else { 365 };
        if days < dy { break; }
        days -= dy;
        y += 1;
    }
    let md = if is_leap(y) { [31,29,31,30,31,30,31,31,30,31,30,31] }
                        else { [31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut mo = 1u32;
    for &m in &md {
        if days < m as i64 { break; }
        days -= m as i64;
        mo += 1;
    }
    (y, mo, (days + 1) as u32)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// 写日志到文件
pub fn log(level: &str, module: &str, msg: &str) {
    let dir = log_base_dir().join("logs");
    let _ = fs::create_dir_all(&dir);
    let line = format!("[{}] [{:<5}] [{:<12}] {}", timestamp(), level, module, msg);
    if let Ok(mut f) = fs::OpenOptions::new()
        .append(true).create(true).open(dir.join("音楽自由"))
    {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

/// 便捷日志宏
#[macro_export]
macro_rules! log_info {
    ($mod:expr, $($arg:tt)*) => {
        $crate::utils::log("INFO", $mod, &format!($($arg)*))
    };
}

// ── 文件名/URL处理 ──

/// 生成酷我随机用户ID
/// 格式: 12345,web,web,web (与Python版兼容)
pub fn random_user() -> String {
    "12345,web,web,web".to_string()
}

/// 去掉酷我API返回的 try{...} 包裹
pub fn strip_try(text: &str) -> String {
    text.strip_prefix("try{")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(text)
        .to_string()
}

/// 清理文件名(去掉非法字符)
pub fn sanitize_filename(s: &str) -> String {
    s.replace('\\', "")
        .replace('/', "")
        .replace(':', "：")
        .replace('*', "·")
        .replace('?', "")
        .replace('"', "'")
        .replace('<', "《")
        .replace('>', "》")
        .replace('|', "")
        .trim()
        .to_string()
}

/// 根据音质返回文件扩展名
pub fn ext_for_quality(q: &str) -> &str {
    match q {
        "flac" => ".flac",
        "mp3128" => ".mp3",
        _ => ".mp3",
    }
}

/// URL编码(百分比编码)
pub fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
