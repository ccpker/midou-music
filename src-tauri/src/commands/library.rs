// ════════════════════════════════════════════════
// 模块: commands::library
// 路径: src-tauri/src/commands/library.rs
// ────────────────────────────────────────────
// 功能: 本地音乐库扫描（多级文件夹递归）
//
// 返回树结构:
//   LibraryNode {
//     name: 目录/文件名
//     path: 绝对路径
//     is_dir: bool
//     children: Vec<LibraryNode>   // 目录的子项
//     songs: Vec<LocalSong>        // 目录内的音频文件
//   }
//
// 支持的音频格式: mp3 / flac / m4a / wav / aac / ogg
// ════════════════════════════════════════════════

use serde::Serialize;
use std::path::Path;

/// 目录树节点
#[derive(Serialize, Clone)]
pub struct LibraryNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// 目录的子项（仅 is_dir=true 时递归填充）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<LibraryNode>,
    /// 目录内的音频文件（仅 is_dir=true 时填充）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub songs: Vec<LocalSong>,
}

/// 本地歌曲
#[derive(Serialize, Clone)]
pub struct LocalSong {
    /// song_id = "local:{绝对路径}"
    pub song_id: String,
    pub name: String,
    pub singer: String,
    pub album: String,
    pub duration: u32,
    pub source: String,
    pub cover_url: Option<String>,
}

/// 支持的音频扩展名
const AUDIO_EXTS: [&str; 7] = ["mp3", "flac", "m4a", "wav", "aac", "ogg", "ape"];

/// 扫描音乐库根目录，返回完整目录树
///
/// 调用路径: 前端进入「本地」区时 invoke('scan_library', { root })
///
/// 注意: 递归扫描整个树。若音乐库巨大，前端可只展开需要的文件夹
/// （本命令一次性返回全树，简单可靠；后续可优化为懒加载）
#[tauri::command]
pub async fn scan_library(root: String) -> Result<LibraryNode, String> {
    crate::debug_log::info("library", &format!("扫描音乐库: {root}"));
    let path = Path::new(&root);
    if !path.exists() {
        return Err(format!("目录不存在: {root}"));
    }
    if !path.is_dir() {
        return Err(format!("不是目录: {root}"));
    }
    scan_dir(path, &root)
}

/// 递归扫描目录
fn scan_dir(dir: &Path, root: &str) -> Result<LibraryNode, String> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let node_path = dir.to_string_lossy().to_string();

    let mut node = LibraryNode {
        name,
        path: node_path.clone(),
        is_dir: true,
        children: Vec::new(),
        songs: Vec::new(),
    };

    let entries = std::fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;

    // 分两轮：先收集，排序后填充（目录在前，文件在后，各自按名称排序）
    let mut subdirs: Vec<std::path::PathBuf> = Vec::new();
    let mut files: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.is_file() && is_audio(&p) {
            files.push(p);
        }
    }

    subdirs.sort();
    files.sort();

    for d in subdirs {
        let child = scan_dir(&d, root)?;
        // 只保留非空目录（有歌曲或有子目录）
        if !child.children.is_empty() || !child.songs.is_empty() {
            node.children.push(child);
        }
    }

    for f in files {
        if let Some(song) = make_local_song(&f) {
            node.songs.push(song);
        }
    }

    Ok(node)
}

/// 判断是否为音频文件
fn is_audio(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 从文件路径构造 LocalSong
fn make_local_song(p: &Path) -> Option<LocalSong> {
    let abs = p.to_string_lossy().to_string();
    // 文件名去掉扩展名作为歌名
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // 尝试拆 "歌手 - 歌名" 格式
    let (singer, name) = split_song_name(stem);

    Some(LocalSong {
        song_id: format!("local:{}", abs),
        name,
        singer,
        album: String::new(),
        duration: 0, // 本地文件不主动读时长（避免解析开销，播放时由 audio 元素获取）
        source: "local".to_string(),
        cover_url: None,
    })
}

/// 拆分 "歌手 - 歌名" 格式
fn split_song_name(raw: &str) -> (String, String) {
    if let Some(pos) = raw.find(" - ") {
        let singer = raw[..pos].trim().to_string();
        let name = raw[pos + 3..].trim().to_string();
        (singer, name)
    } else {
        (String::new(), raw.to_string())
    }
}
