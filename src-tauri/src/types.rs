// ════════════════════════════════════════════════
// 模块: types
// 路径: src-tauri/src/types.rs
// ────────────────────────────────────────────
// 功能: 全局共享数据结构
// 标注: 所有跨模块传递的数据类型在此定义
// 原则: 只定义结构，不含业务逻辑
// ════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// ── 歌曲 ─────────────────────────────────────────

/// 歌曲（跨窗口传递用）
#[derive(Serialize, Deserialize, Clone)]
pub struct Song {
    pub song_id: String,
    pub name: String,
    pub singer: String,
    pub album: String,
    pub duration: u32,
    pub source: String,
    /// 封面图 URL（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    /// 歌词 ID（可选，酷狗侧边车返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyric_id: Option<String>,
}

// ── 播放 ─────────────────────────────────────────

/// play_url 返回值
#[derive(Serialize)]
pub struct PlayUrlResult {
    pub url: String,
    pub source: String,
    pub quality: String,
    /// 酷狗 SSA 验证码标识（仅 20028 触发时非空，前端需启动滑块验证）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssa_code: Option<String>,
}

/// 播放状态（emit 到所有窗口）
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayState {
    pub song: Option<Song>,
    pub url: Option<String>,
    pub is_playing: bool,
    pub position: f64,
    pub duration: f64,
}

// ── 酷狗登录凭证 ────────────────────────────────

/// 酷狗登录凭证（内存缓存 + DB 持久化）
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct KugouAuth {
    /// 是否已登录
    pub logged_in: bool,
    /// 设备标识
    pub dfid: String,
    /// 登录 token（24h 有效）
    pub token: String,
    /// 用户 ID
    pub userid: u64,
    /// VIP token（7d 有效）
    pub vip_token: String,
    /// VIP 类型
    pub vip_type: u32,
    /// token 过期时间戳（秒）
    pub token_expires: i64,
    /// API mid（kmr 接口专用，md5(dfid) 或服务端下发）
    pub kugou_api_mid: String,
}

impl KugouAuth {
    /// 空凭证（未登录状态）
    pub fn default_fallback() -> Self {
        Self {
            logged_in: false,
            dfid: "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".to_string(),
            token: String::new(),
            userid: 0,
            vip_token: String::new(),
            vip_type: 0,
            token_expires: 0,
            kugou_api_mid: String::new(),
        }
    }
}

// ── 全局状态 ─────────────────────────────────────

/// 应用全局状态（注入到 tauri State）
pub struct AppState {
    /// HTTP 客户端（全局复用）
    pub client: reqwest::Client,
    /// SQLite 连接（Mutex 保护，确保 Send + Sync）
    pub db: Mutex<rusqlite::Connection>,
    /// 酷狗登录凭证
    pub kugou_auth: Mutex<KugouAuth>,
    /// rodio 音频播放句柄（Send，内部通过通道驱动专用线程）
    pub audio_player: crate::audio::AudioHandle,
}

/// AppState 必须线程安全（tauri 命令跨线程调用）
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}
