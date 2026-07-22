// ════════════════════════════════════════════════
// 音楽自由 — 下载引擎
// ────────────────────────────────────────────
// 功能:   下载歌曲到本地,含FLAC→320k→128k降级链
// 输入:   rid + 歌曲信息 + 分类
// 输出:   LocalSong(下载信息)
// 依赖:   crate::models, reqwest::Client, tokio::fs
// 边界:   文件名校验防重名; 单文件最大50MB
// 备注:   warp直写磁盘,不经过Tauri文件系统API
// ════════════════════════════════════════════════

use std::path::PathBuf;

use reqwest::Client;

use crate::models::{AppConfig, LocalSong};
use crate::search::kuwo;
use crate::utils::{ext_for_quality, sanitize_filename};
use crate::log_info;

/// 下载歌曲(带降级链)
///
/// # 过程
/// 1. 尝试FLAC音质 → 320k mp3 → 128k mp3
/// 2. warp直写磁盘到 Downloads/音楽自由/
/// 3. 文件名校验,自动防重名
///
/// # 返回
/// - `Ok(LocalSong)`: 下载完成信息
/// - `Err(String)`: 所有音质都失败
pub async fn download_with_fallback(
    client: &Client,
    rid: &str,
    name: &str,
    singer: &str,
    category: &str,
    cfg: &AppConfig,
) -> Result<LocalSong, String> {
    let target_dir = if category.is_empty() || category == "默认" {
        cfg.download_dir.clone()
    } else {
        let cat_dir = sanitize_filename(category);
        let p = cfg.download_dir.join(&cat_dir);
        tokio::fs::create_dir_all(&p).await.ok();
        p
    };

    let quality_order = ["flac", "mp3320", "mp3128"];
    let quality_label = ["FLAC", "320k", "128k"];

    for (i, &quality) in quality_order.iter().enumerate() {
        log_info!("download", "尝试 {}: {} - {} ({})", quality_label[i], name, singer, rid);

        // qq源播放不走kuwo play_url
        // NOTE: 当前下载仅支持kuwo源

        let play_resp = kuwo::play_url(client, rid, quality).await?;
        let url = play_resp.get("url").and_then(|v| v.as_str()).unwrap_or("");

        if url.is_empty() || url == "err" {
            log_info!("download", "{} 不可用", quality_label[i]);
            continue;
        }

        let ext = ext_for_quality(quality);
        let safe_name = sanitize_filename(name);
        let safe_singer = sanitize_filename(singer);
        let base_name = if safe_singer.is_empty() { safe_name.clone() } else { format!("{}-{}", safe_name, safe_singer) };
        let mut filename = format!("{}{}", base_name, ext);
        let mut filepath = target_dir.join(&filename);

        // 校验文件名,重复则加数字后缀
        let mut counter = 1;
        while filepath.exists() {
            filename = format!("{}_{}{}", base_name, counter, ext);
            filepath = target_dir.join(&filename);
            counter += 1;
        }

        // 下载文件
        let resp = client.get(url)
            .send().await.map_err(|e| format!("下载请求: {e}"))?;

        let bytes = resp.bytes().await.map_err(|e| format!("下载数据: {e}"))?;
        log_info!("download", "{} 大小: {} bytes", quality_label[i], bytes.len());

        tokio::fs::write(&filepath, &bytes).await
            .map_err(|e| format!("写文件: {e}"))?;

        let path_str = filepath.to_string_lossy().to_string();
        log_info!("download", "下载完成: {}", path_str);

        return Ok(LocalSong {
            song_id: rid.to_string(),
            name: name.to_string(),
            singer: singer.to_string(),
            category: if category.is_empty() { "默认".to_string() } else { category.to_string() },
            filename: filename.clone(),
            path: path_str,
            quality: quality.to_string(),
            size: bytes.len() as u64,
            downloaded_at: crate::utils::timestamp(),
        });
    }

    Err("所有音质尝试失败".to_string())
}

/// 查找本地歌曲URL(用于播放)
///
/// # 返回
/// - `Some(url_string)`: 本地文件URL
/// - `None`: 未找到
pub fn find_local_url(
    _cfg: &AppConfig,
    idx: &crate::models::LocalIndexData,
    song_id: &str,
    name: &str,
    singer: &str,
) -> Option<String> {
    if let Some(s) = idx.find_by_id(song_id) {
        let p = PathBuf::from(&s.path);
        if p.exists() {
            let url = format!("local://{}", p.to_string_lossy());
            log_info!("download", "本地命中: {}", url);
            return Some(url);
        }
    }
    if let Some(s) = idx.find_fuzzy(name, singer) {
        let p = PathBuf::from(&s.path);
        if p.exists() {
            let url = format!("local://{}", p.to_string_lossy());
            log_info!("download", "模糊命中: {} → {}", s.song_id, url);
            return Some(url);
        }
    }
    None
}
