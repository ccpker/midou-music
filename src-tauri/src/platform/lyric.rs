// ════════════════════════════════════════════════
// 模块: platform::lyric
// 路径: src-tauri/src/platform/lyric.rs
// ────────────────────────────────────────────
// 功能: 歌词获取（多源三级回退）
//
// 策略:
//   1. 酷狗官方歌词（source=kugou 且有 FileHash）
//      lyrics.kugou.com/search → download（两步，legacy 已验证）
//   2. LRCLIB 精确匹配  lrclib.net/api/get
//   3. LRCLIB 模糊搜索  lrclib.net/api/search
//
// 返回: LRC 格式歌词文本（含 [ti:]/[ar:]/[mm:ss.xx] 行）
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;

/// 酷狗歌词接口 UA（与 legacy 一致）
const KG_LYRIC_UA: &str = "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

/// 歌词获取主入口（三级回退）
///
/// 参数:
/// - name: 歌名
/// - singer: 歌手名
/// - duration_secs: 时长（秒），酷狗歌词匹配用
/// - source: 音源标识（kugou/kuwo/bili）
/// - song_id: 完整 song_id（酷狗含 FileHash）
pub async fn fetch_lyric(
    client: &Client,
    name: &str,
    singer: &str,
    duration_secs: u32,
    source: &str,
    song_id: &str,
) -> Result<String, String> {
    // ── 第一级：酷狗官方歌词（仅酷狗源且有 hash）──
    if source == "kugou" {
        let file_hash = extract_kugou_hash(song_id);
        if !file_hash.is_empty() {
            match kugou_lyric(client, name, duration_secs, &file_hash).await {
                Ok(lrc) if !lrc.trim().is_empty() => {
                    crate::debug_log::info("lyric", &format!("酷狗歌词命中: {name} - {singer}"));
                    return Ok(lrc);
                }
                Ok(_) => {
                    crate::debug_log::info("lyric", "酷狗歌词为空，回退 LRCLIB");
                }
                Err(e) => {
                    crate::debug_log::info("lyric", &format!("酷狗歌词失败({e})，回退 LRCLIB"));
                }
            }
        }
    }

    // ── 第二级：LRCLIB 精确匹配 ──
    match lrclib_get(client, name, singer).await {
        Ok(lrc) if !lrc.trim().is_empty() => {
            crate::debug_log::info("lyric", &format!("LRCLIB 精确命中: {name} - {singer}"));
            return Ok(lrc);
        }
        _ => {
            crate::debug_log::info("lyric", "LRCLIB 精确未命中，回退模糊搜索");
        }
    }

    // ── 第三级：LRCLIB 模糊搜索 ──
    lrclib_search(client, name, singer).await
}

/// 从 song_id 提取酷狗 FileHash
/// song_id 格式: "kugou:{FileHash}" 或 "kugou:{FileHash}|{AlbumID}"
fn extract_kugou_hash(song_id: &str) -> String {
    let s = song_id.strip_prefix("kugou:").unwrap_or(song_id);
    let hash = s.split('|').next().unwrap_or("").to_string();
    hash
}

/// 酷狗官方歌词（两步：search → download）
async fn kugou_lyric(
    client: &Client,
    name: &str,
    duration_secs: u32,
    file_hash: &str,
) -> Result<String, String> {
    // Step 1: 搜索歌词（duration 单位毫秒，hash 精确定位）
    let duration_ms = (duration_secs as u64).saturating_mul(1000);
    let search_url = format!(
        "http://lyrics.kugou.com/search?ver=1&man=yes&client=pc&keyword={}&duration={}&hash={}",
        url_encode(name),
        duration_ms,
        url_encode(file_hash),
    );
    let resp = client
        .get(&search_url)
        .header("User-Agent", KG_LYRIC_UA)
        .send()
        .await
        .map_err(|e| format!("酷狗歌词搜索请求: {e}"))?;
    let text = resp.text().await.map_err(|e| format!("酷狗歌词搜索响应: {e}"))?;

    let root: Value = serde_json::from_str(&text).map_err(|e| format!("酷狗歌词JSON解析: {e}"))?;
    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 200 {
        return Err("酷狗歌词未找到".into());
    }

    let first = root
        .get("candidates")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .ok_or("酷狗歌词 candidate 为空")?;
    let lyric_id = first.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let accesskey = first.get("accesskey").and_then(|v| v.as_str()).unwrap_or("");
    if lyric_id.is_empty() || accesskey.is_empty() {
        return Err("酷狗歌词 id/accesskey 缺失".into());
    }

    // Step 2: 下载歌词
    let dl_url = format!(
        "http://lyrics.kugou.com/download?ver=1&client=pc&id={}&accesskey={}&fmt=lrc&charset=utf8",
        lyric_id, accesskey,
    );
    let dl_resp = client
        .get(&dl_url)
        .header("User-Agent", KG_LYRIC_UA)
        .send()
        .await
        .map_err(|e| format!("酷狗歌词下载: {e}"))?;
    let lrc = dl_resp.text().await.map_err(|e| format!("酷狗歌词内容: {e}"))?;

    // 兼容：若返回 JSON 包裹，取 content 字段（content 为 base64 编码的 LRC）
    let lrc = if lrc.trim_start().starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(&lrc) {
            if let Some(c) = v.get("content").and_then(|c| c.as_str()) {
                if !c.trim().is_empty() {
                    // content 是 base64，解码为 LRC 文本
                    decode_base64_lrc(c).unwrap_or_else(|| c.to_string())
                } else {
                    lrc
                }
            } else {
                lrc
            }
        } else {
            lrc
        }
    } else {
        lrc
    };

    if lrc.trim().is_empty() {
        return Err("酷狗歌词内容为空".into());
    }
    Ok(lrc)
}

/// LRCLIB 精确匹配
/// GET https://lrclib.net/api/get?track_name=&artist_name=
async fn lrclib_get(client: &Client, name: &str, singer: &str) -> Result<String, String> {
    let url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        url_encode(name),
        url_encode(singer),
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "midou-music/0.4 (https://github.com/ccpker/midou-music)")
        .send()
        .await
        .map_err(|e| format!("LRCLIB 精确请求: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LRCLIB 精确 HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("LRCLIB 精确响应: {e}"))?;
    parse_lrclib_single(&text)
}

/// LRCLIB 模糊搜索
/// GET https://lrclib.net/api/search?q=name+singer
async fn lrclib_search(client: &Client, name: &str, singer: &str) -> Result<String, String> {
    let q = if singer.is_empty() {
        name.to_string()
    } else {
        format!("{name} {singer}")
    };
    let url = format!("https://lrclib.net/api/search?q={}", url_encode(&q));
    let resp = client
        .get(&url)
        .header("User-Agent", "midou-music/0.4 (https://github.com/ccpker/midou-music)")
        .send()
        .await
        .map_err(|e| format!("LRCLIB 模糊请求: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LRCLIB 模糊 HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("LRCLIB 模糊响应: {e}"))?;

    let arr: Value = serde_json::from_str(&text).map_err(|e| format!("LRCLIB 模糊JSON解析: {e}"))?;
    if let Some(items) = arr.as_array() {
        for item in items {
            let lrc = extract_synced_lyrics(item);
            if !lrc.is_empty() {
                return Ok(lrc);
            }
        }
    }
    Err("未找到歌词".into())
}

/// 解析 LRCLIB 单个对象响应（get 接口返回单对象）
fn parse_lrclib_single(text: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("LRCLIB JSON解析: {e}"))?;
    let lrc = extract_synced_lyrics(&v);
    if lrc.is_empty() {
        Err("LRCLIB 无同步歌词".into())
    } else {
        Ok(lrc)
    }
}

/// 从 LRCLIB 对象提取同步歌词（优先 syncedLyrics，其次 plainLyrics）
fn extract_synced_lyrics(v: &Value) -> String {
    if let Some(s) = v.get("syncedLyrics").and_then(|s| s.as_str()) {
        if !s.trim().is_empty() {
            return s.to_string();
        }
    }
    if let Some(s) = v.get("plainLyrics").and_then(|s| s.as_str()) {
        if !s.trim().is_empty() {
            return s.to_string();
        }
    }
    String::new()
}

/// URL 编码（UTF-8 字节级，正确处理中文）
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// base64 解码（标准 + URL-safe 兼容），失败返回 None
fn decode_base64_lrc(s: &str) -> Option<String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine.decode(s.trim()).ok()?;
    String::from_utf8(bytes).ok()
}
