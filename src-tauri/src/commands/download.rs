// ════════════════════════════════════════════════
// 模块: commands::download
// 路径: src-tauri/src/commands/download.rs
// ────────────────────────────────────────────
// 功能: 歌曲下载（全源）+ 音质列表查询
//
// 下载流程:
//   1. 按 song_id 路由拿真实音频 URL（复用各 platform 的 play_url）
//   2. reqwest 流式下载到用户指定目录
//   3. 同时调 fetch_lyric 拿歌词，存同名 .lrc
//   4. 返回保存的绝对路径
//
// 音质:
//   get_qualities(source) 返回该源「候选音质」列表，前端让用户挑
//   实际能否拿到由平台鉴权决定（如酷狗 flac 需 VIP）
// ════════════════════════════════════════════════

use std::path::Path;
use std::sync::Arc;

use futures_util::StreamExt;

use crate::types::{AppState, KugouAuth};

/// 音源 → 候选音质列表（前端展示，用户选一个下载）
///
/// - kuwo: 车载 API 实测 320kmp3 稳定，128/192 备选
/// - bili: 平台统一 high，无音质选择
/// - kugou: 概念版 128 稳定，320/flac 需 VIP 权益
#[tauri::command]
pub async fn get_qualities(source: String) -> Result<Vec<String>, String> {
    Ok(match source.as_str() {
        "kuwo" => vec![
            "128kmp3".to_string(),
            "192kmp3".to_string(),
            "320kmp3".to_string(),
        ],
        "kugou" => vec![
            "128".to_string(),
            "320".to_string(),
            "flac".to_string(),
        ],
        "bili" | "local" => vec!["high".to_string()],
        _ => vec!["high".to_string()],
    })
}

/// 下载一首歌
///
/// 参数:
/// - song_id: 完整 song_id（kugou:xxx / auXxx / bvXxx / 纯数字=酷我）
/// - name/singer: 用于文件名 + 歌词匹配
/// - album: 用于歌词匹配（可选）
/// - source: 音源标识
/// - dir: 目标目录（用户点下载时选的目录，即音乐库根目录）
/// - quality: 音质（来自 get_qualities 的候选项）
///
/// 返回: 保存后的绝对路径
#[tauri::command]
pub async fn download_song(
    song_id: String,
    name: String,
    singer: String,
    album: String,
    duration: u32,
    source: String,
    dir: String,
    quality: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::debug_log::info(
        "download",
        &format!("下载请求: song_id={song_id}, name={name}, singer={singer}, album={album}, source={source}, quality={quality}, dir={dir}"),
    );

    // ── 1. 拿真实音频 URL ──────────────────────
    let url = resolve_download_url(&state, &song_id, &source, &quality).await?;
    if url.is_empty() {
        return Err("获取下载 URL 为空".to_string());
    }
    crate::debug_log::info("download", &format!("拿到 URL, 长度={}", url.len()));

    // ── 2. 构造安全文件名 ──────────────────────
    let ext = infer_ext(&url, &source);
    let safe_name = sanitize_filename(&format!("{} - {}", singer.trim(), name.trim()));
    let file_name = if safe_name.is_empty() {
        format!("download_{}", chrono_stamp())
    } else {
        safe_name
    };
    let file_path = Path::new(&dir).join(format!("{}.{}", file_name, ext));

    // ── 3. 流式下载 ────────────────────────────
    let resp = state
        .client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("下载 HTTP 错误: {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| format!("创建文件失败: {e}"))?;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载流错误: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("写文件失败: {e}"))?;
        downloaded += chunk.len() as u64;
        // 每 2MB 打一次日志
        if downloaded % (2 * 1024 * 1024) < 1024 * 1024 {
            let pct = if total > 0 {
                (downloaded as f64 / total as f64 * 100.0) as u32
            } else {
                0
            };
            crate::debug_log::info("download", &format!("进度: {downloaded} / {total} ({pct}%)"));
        }
    }
    crate::debug_log::info("download", &format!("下载完成: {}", file_path.display()));

    // ── 4. 下载歌词存 .lrc ──────────────────────
    let lrc_path = file_path.with_extension("lrc");
    if !lrc_path.exists() {
        match crate::platform::lyric::fetch_lyric(
            &state.client,
            &name,
            &singer,
            duration,
            &source,
            &song_id,
        )
        .await
        {
            Ok(lrc) if !lrc.trim().is_empty() => {
                if let Err(e) = tokio::fs::write(&lrc_path, lrc).await {
                    crate::debug_log::warn("download", &format!("写歌词失败: {e}"));
                } else {
                    crate::debug_log::info("download", &format!("歌词已存: {}", lrc_path.display()));
                }
            }
            _ => {
                crate::debug_log::info("download", "无歌词（或获取失败），跳过 .lrc");
            }
        }
    }

    Ok(file_path.to_string_lossy().to_string())
}

/// 按 source 路由，复用各 platform 的 play_url 拿 URL
async fn resolve_download_url(
    state: &Arc<AppState>,
    song_id: &str,
    source: &str,
    quality: &str,
) -> Result<String, String> {
    match source {
        "kuwo" => {
            let v = crate::platform::kuwo::play_url(&state.client, song_id, quality).await?;
            Ok(v.get("url").and_then(|s| s.as_str()).unwrap_or("").to_string())
        }
        "bili" => {
            let v = crate::platform::bili::play_url(&state.client, song_id, "high").await?;
            Ok(v.get("url").and_then(|s| s.as_str()).unwrap_or("").to_string())
        }
        "kugou" => {
            let auth: KugouAuth = state
                .kugou_auth
                .lock()
                .map_err(|e| format!("kugou_auth锁失败: {e}"))?
                .clone();
            if !auth.logged_in || auth.token.is_empty() {
                return Err("酷狗下载需要先扫码登录".to_string());
            }
            crate::platform::kugou::play_url(&state.client, song_id, &auth, quality).await
        }
        "local" => {
            // 本地文件本身无需下载，直接返回路径
            Ok(song_id.strip_prefix("local:").unwrap_or(song_id).to_string())
        }
        _ => Err(format!("未知音源: {source}")),
    }
}

/// 从 URL 推断扩展名
fn infer_ext(url: &str, source: &str) -> String {
    // 去掉 query 部分
    let path = url.split('?').next().unwrap_or(url);
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        let e = ext.to_lowercase();
        if matches!(e.as_str(), "mp3" | "flac" | "m4a" | "wav" | "aac" | "ogg") {
            return e;
        }
    }
    // 默认按源推断
    match source {
        "kugou" | "kuwo" => "mp3".to_string(),
        _ => "mp3".to_string(),
    }
}

/// 清洗文件名中的非法字符
fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 32 => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

/// 简单时间戳（用于兜底文件名）
fn chrono_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}
