// ════════════════════════════════════════════════
// 音楽自由 — 歌词获取适配器
// ────────────────────────────────────────────
// 功能:   多源歌词获取(酷我 + LRCLIB)
// 输入:   歌名 + 歌手名
// 输出:   JSON { lyric, synced, source }
// 依赖:   reqwest::Client, serde_json
// 边界:   歌词可能不存在; LRCLIB需要URL编码
// 备注:   酷我优先,LRCLIB兜底
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;

use crate::log_info;

// ── 主入口(公开) ──

/// 多备选源获取歌词
///
/// # 过程
/// 0. 如果 source="kugou" 且传了 duration+hash → 酷狗歌词优先
/// 1. 酷我歌词搜索(优先)
/// 2. 酷我失败 → LRCLIB(次选)
///
/// # 返回
/// `{ "lyric": "...", "synced": true/false, "source": "kuwo"|"lrclib"|"kugou" }`
pub async fn fetch(
    client: &Client,
    name: &str,
    artist: &str,
    source: Option<&str>,
    duration_ms: Option<u32>,
    file_hash: Option<&str>,
) -> Result<Value, String> {
    // 0. 如果是 kugou 源，优先酷狗歌词
    if source == Some("kugou") {
        if let (Some(dur), Some(hash)) = (duration_ms, file_hash) {
            match kugou_lyric(client, name, artist, dur, hash).await {
                Ok(Some(lrc)) => {
                    return Ok(serde_json::json!({
                        "lyric": lrc,
                        "synced": lrc.contains("[00:"),
                        "source": "kugou"
                    }));
                }
                Ok(None) => log_info!("lyric", "酷狗无歌词，走酷我"),
                Err(e) => log_info!("lyric", "酷狗歌词失败: {e}"),
            }
        }
    }

    // 1. 酷我歌词搜索(优先)
    match kuwo_lyric_search(client, name, artist).await {
        Ok(Some(lyric)) => {
            return Ok(serde_json::json!({"lyric": lyric, "synced": false, "source": "kuwo"}));
        }
        Ok(None) => log_info!("lyric", "酷无歌词,走LRCLIB"),
        Err(e) => log_info!("lyric", "酷我歌词失败: {e}"),
    }

    // 2. LRCLIB兜底
    lrclib_lyric(client, name, artist).await
}

// ── 酷狗歌词(内部) ──

/// 酷狗歌词获取
async fn kugou_lyric(
    client: &Client,
    name: &str,
    artist: &str,
    duration_ms: u32,
    file_hash: &str,
) -> Result<Option<String>, String> {
    match crate::platform::kugou::lyrics_impl(client, name, artist, duration_ms, file_hash).await {
        Ok(lrc) => Ok(Some(lrc)),
        Err(e) => {
            log_info!("lyric", "酷狗歌词失败: {e}");
            Ok(None)
        }
    }
}

// ── 酷我歌词搜索(内部) ──

/// 从酷我搜索歌词
async fn kuwo_lyric_search(client: &Client, name: &str, artist: &str) -> Result<Option<String>, String> {
    let kw = format!("{} {}", name, artist);
    let resp = client.get("http://search.kuwo.cn/r.s")
        .query(&[
            ("all", kw.as_str()), ("ft", "music"), ("itemset", "web_2013"),
            ("client", "kt"), ("pn", "0"), ("rn", "1"),
            ("rformat", "json"), ("encoding", "utf8"),
        ])
        .header("User-Agent", "Mozilla/5.0")
        .send().await.map_err(|e| format!("歌词搜索: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("歌词响应: {e}"))?;
    let cleaned = text.strip_prefix("try{")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(&text)
        .replace('\'', "\"");

    let root: Value = serde_json::from_str(&cleaned).map_err(|e| format!("歌词JSON: {e}"))?;
    if let Some(abslist) = root.get("abslist").and_then(|v| v.as_array()) {
        if let Some(first) = abslist.first() {
            if let Some(rid_raw) = first.get("MUSICRID").and_then(|v| v.as_str()) {
                let rid = rid_raw.replace("MUSIC_", "");
                return kuwo_lyric_get(client, &rid).await;
            }
        }
    }
    Ok(None)
}

/// 根据RID获取酷我歌词内容
async fn kuwo_lyric_get(client: &Client, rid: &str) -> Result<Option<String>, String> {
    let url = format!("https://mobi.kuwo.cn/mobi.s?f=so&rid={}", rid);
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send().await.map_err(|e| format!("歌词内容: {e}"))?;

    let body = resp.text().await.map_err(|e| format!("歌词内容响应: {e}"))?;
    if body.is_empty() || body.contains("err") {
        return Ok(None);
    }

    // 提取 <lyric>...</lyric> 内容
    if let Some(start) = body.find("<lyric>") {
        if let Some(end) = body.find("</lyric>") {
            let lyric = &body[start + 7..end];
            if !lyric.trim().is_empty() {
                return Ok(Some(lyric.to_string()));
            }
        }
    }
    Ok(None)
}

// ── LRCLIB歌词(内部) ──

/// 从 LRCLIB 获取歌词(LRC格式,含时间轴)
async fn lrclib_lyric(client: &Client, name: &str, artist: &str) -> Result<Value, String> {
    let encoded_name: String = name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string()
        } else if c == ' ' {
            "%20".to_string()
        } else {
            format!("%{:02X}", c as u8)
        })
        .collect();

    let encoded_artist: String = artist.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string()
        } else if c == ' ' {
            "%20".to_string()
        } else {
            format!("%{:02X}", c as u8)
        })
        .collect();

    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        encoded_artist, encoded_name
    );

    log_info!("lrclib", "请求: {}", url);
    let resp = match client.get(&url)
        .header("User-Agent", "music-app/0.3.0")
        .timeout(std::time::Duration::from_secs(10))
        .send().await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            log_info!("lrclib", "HTTP {}", r.status());
            let status = r.status();
            return Ok(serde_json::json!({"lyric": null, "synced": false, "source": "lrclib", "error": format!("HTTP {}", status)}));
        }
        Err(e) => {
            log_info!("lrclib", "请求失败: {e}");
            return Ok(serde_json::json!({"lyric": null, "synced": false, "source": "lrclib", "error": e.to_string()}));
        }
    };

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            return Ok(serde_json::json!({"lyric": null, "synced": false, "source": "lrclib", "error": format!("JSON: {e}")}));
        }
    };

    // 优先取 syncedLyrics(LRC格式), 其次 plainLyrics
    let lyric = data.get("syncedLyrics")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| data.get("plainLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty()));

    Ok(serde_json::json!({
        "lyric": lyric,
        "synced": lyric.is_some() && data.get("syncedLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).is_some(),
        "source": "lrclib"
    }))
}
