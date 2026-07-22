// ════════════════════════════════════════════════
// 音楽自由 — MusicBrainz 验证适配器
// ────────────────────────────────────────────
// 功能:   调用MusicBrainz API验证歌曲真实身份(MBID)
// 输入:   歌名 + 歌手名
// 输出:   Option<(mbid, duration)>
// 依赖:   reqwest::Client, crate::utils
// 边界:   MB API限流; duration可能为0(无元数据)
// 备注:   主要用于纯净搜索的MBID查证
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;

use crate::utils::urlencoding;
use crate::log_info;
use std::time::Duration;

/// MusicBrainz 录音搜索
///
/// # 参数
/// - `song`: 歌名
/// - `artist`: 歌手名
///
/// # 返回
/// - `Ok(Some((mbid, duration_s)))`: 命中,含MBID和时长(秒)
/// - `Ok(None)`: 无结果
/// - `Err(String)`: 错误描述
///
/// # 过程
/// 1. 尝试"歌名+歌手"精确查询
/// 2. 精确无结果 → 降级为纯歌名查询
/// 3. 取第一条命中的MBID和duration
pub async fn search(client: &Client, song: &str, artist: &str) -> Result<Option<(String, u32)>, String> {
    let q_song = song.replace("\"", "").trim().to_string();
    let q_artist = artist.replace("\"", "").trim().to_string();

    let queries: Vec<String> = if !q_artist.is_empty() {
        vec![
            format!("recording:%22{}%22%20AND%20artist:%22{}%22",
                urlencoding(&q_song), urlencoding(&q_artist)),
            format!("recording:%22{}%22", urlencoding(&q_song)),
        ]
    } else {
        vec![format!("recording:%22{}%22", urlencoding(&q_song))]
    };

    for query in queries {
        let url = format!("https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=3", query);
        let resp = match client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (compatible; music-app/0.3.0; +https://example.com)")
            .timeout(Duration::from_secs(15))
            .send().await
        {
            Ok(r) => r,
            Err(e) => { log_info!("mb", "请求失败: {}", e); continue; }
        };

        let data: Value = match resp.json().await {
            Ok(d) => d,
            Err(e) => { log_info!("mb", "JSON失败: {}", e); continue; }
        };

        if let Some(recs) = data.get("recordings").and_then(|v| v.as_array()) {
            if let Some(first) = recs.first() {
                let mbid = first.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let dur_ms = first.get("duration").and_then(|v| v.as_i64()).unwrap_or(0);
                let dur_s = (dur_ms / 1000) as u32;
                log_info!("mb", "命中: {} dur={}ms (query={})", mbid, dur_ms, query);
                if !mbid.is_empty() {
                    return Ok(Some((mbid, dur_s)));
                }
            }
        } else {
            log_info!("mb", "无结果 (query={})", query);
        }
    }

    Ok(None)
}
