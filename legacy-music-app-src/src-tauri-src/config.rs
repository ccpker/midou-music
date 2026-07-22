// ════════════════════════════════════════════════
// 音楽自由 — 配置持久化
// ────────────────────────────────────────────
// 功能:   加载/保存 config.json 和 local_index.json
// 输入:   磁盘文件路径(AppConfig, LocalIndexData)
// 输出:   AppConfig, LocalIndexData
// 依赖:   crate::models, crate::utils, serde_json, std::fs
// 边界:   配置不存在时使用默认值,下载目录不存在时回退到Downloads
// 备注:   config.json 在exe同级目录, local_index.json在下载目录内
// ════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::models::{AppConfig, LocalIndexData};
use crate::utils::{log, log_exe_dir};

// ── 应用程序配置 ──

fn config_path() -> PathBuf {
    log_exe_dir().join("config.json")
}

/// 加载配置(不存在则用默认值)
pub fn load_config() -> AppConfig {
    let default_dir = dirs::download_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("音楽自由");
    let path = config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(dd) = v.get("download_dir").and_then(|v| v.as_str()) {
                let p = PathBuf::from(dd);
                if p.exists() || p.parent().map(|x| x.exists()).unwrap_or(false) {
                    log("INFO", "config", &format!("加载下载目录: {dd}"));
                    return AppConfig { download_dir: p };
                }
            }
        }
    }
    AppConfig { download_dir: default_dir }
}

/// 保存配置到磁盘
pub fn save_config(cfg: &AppConfig) {
    let path = config_path();
    let json = serde_json::json!({"download_dir": cfg.download_dir.to_string_lossy()});
    if let Ok(s) = serde_json::to_string_pretty(&json) {
        let _ = fs::write(&path, s);
        log("INFO", "config", "配置已保存");
    }
}

// ── 本地索引 ──

fn local_index_path(cfg: &AppConfig) -> PathBuf {
    cfg.download_dir.join("local_index.json")
}

/// 加载本地歌曲索引
pub fn load_local_index(cfg: &AppConfig) -> LocalIndexData {
    let path = local_index_path(cfg);
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<LocalIndexData>(&data) {
            log("INFO", "index", &format!("本地索引加载: {} 首歌, {} 个分类", v.songs.len(), v.categories.len()));
            return v;
        }
    }
    LocalIndexData::new()
}

/// 保存本地歌曲索引到磁盘
pub fn save_local_index(cfg: &AppConfig, idx: &LocalIndexData) {
    let path = local_index_path(cfg);
    if let Ok(s) = serde_json::to_string_pretty(idx) {
        let _ = fs::write(&path, s);
    }
}

// ── 酷狗登录态持久化 ──

/// 酷狗登录态（扫码/密码登录成功后持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KugouAuth {
    pub dfid: String,
    pub token: String,
    pub userid: u64,
    pub vip_token: String,
    pub vip_type: u32,
    pub username: String,
    pub logged_in: bool,
}

impl Default for KugouAuth {
    fn default() -> Self {
        Self {
            dfid: "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".into(),
            token: String::new(),
            userid: 0,
            vip_token: String::new(),
            vip_type: 0,
            username: String::new(),
            logged_in: false,
        }
    }
}

fn kugou_auth_path() -> PathBuf {
    log_exe_dir().join("kugou_auth.json")
}

/// 加载酷狗登录态（不存在则用默认值）
pub fn load_kugou_auth() -> KugouAuth {
    let path = kugou_auth_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(auth) = serde_json::from_str::<KugouAuth>(&data) {
            log("INFO", "kugou_auth", &format!("已加载酷狗登录态: logged_in={}", auth.logged_in));
            return auth;
        }
    }
    KugouAuth::default()
}

/// 保存酷狗登录态到磁盘
pub fn save_kugou_auth(auth: &KugouAuth) {
    let path = kugou_auth_path();
    if let Ok(s) = serde_json::to_string_pretty(auth) {
        let _ = fs::write(&path, s);
        log("INFO", "kugou_auth", "酷狗登录态已保存");
    }
}
