// ════════════════════════════════════════════════
// 音楽自由 — 数据结构定义
// ────────────────────────────────────────────
// 功能:   系统所有核心数据结构的定义
// 输入:   (无，纯类型定义)
// 输出:   Song, LocalSong, LocalIndexData, AppConfig
// 依赖:   serde, std::path
// 边界:   (无)
// 备注:   所有search适配器都引用此模块
// ════════════════════════════════════════════════

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

// ── 歌曲(通用) ──

/// 统一歌曲模型，各源搜索返回均映射为此结构
#[derive(Serialize, Clone)]
pub struct Song {
    pub song_id: String,
    pub name: String,
    pub singer: String,
    pub album: String,
    pub duration: u32,
    pub source: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mbid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mb_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// 咪咕版权标识("1"=原唱)
    pub migu_copyright: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub migu_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// 排序分数
    pub score: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// B站视频cid(播放时用)
    pub bili_cid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// B站封面URL
    pub bili_cover: Option<String>,
}

// ── 本地歌曲(磁盘) ──

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalSong {
    pub song_id: String,
    pub name: String,
    pub singer: String,
    pub category: String,
    pub filename: String,
    pub path: String,
    pub quality: String,
    pub size: u64,
    pub downloaded_at: String,
}

// ── 本地索引 ──

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalIndexData {
    pub songs: Vec<LocalSong>,
    pub categories: Vec<String>,
}

impl LocalIndexData {
    pub fn new() -> Self {
        Self { songs: vec![], categories: vec!["默认".to_string()] }
    }

    pub fn find_by_id(&self, song_id: &str) -> Option<&LocalSong> {
        self.songs.iter().find(|s| s.song_id == song_id)
    }

    pub fn find_fuzzy(&self, name: &str, singer: &str) -> Option<&LocalSong> {
        let nl = name.to_lowercase();
        let sl = singer.to_lowercase();
        self.songs.iter().find(|s| {
            let sn = s.name.to_lowercase();
            let ss = s.singer.to_lowercase();
            sn == nl && ss.contains(&sl)
                || sl == ss && sn.contains(&nl)
                || sn.contains(&nl) && ss.contains(&sl)
        })
    }

    pub fn add_song(&mut self, song: LocalSong) {
        self.songs.retain(|s| s.song_id != song.song_id);
        self.songs.push(song);
    }

    pub fn remove_song(&mut self, song_id: &str) {
        self.songs.retain(|s| s.song_id != song_id);
    }

    pub fn add_category(&mut self, cat: &str) -> bool {
        if !self.categories.contains(&cat.to_string()) {
            self.categories.push(cat.to_string());
            true
        } else {
            false
        }
    }

    pub fn ensure_category(&mut self, cat: &str) {
        if !cat.is_empty() {
            self.add_category(cat);
        }
    }
}

// ── 配置 ──

#[derive(Clone)]
pub struct AppConfig {
    pub download_dir: PathBuf,
}
