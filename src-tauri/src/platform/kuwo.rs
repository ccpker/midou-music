// ════════════════════════════════════════════════
// 模块: platform::kuwo
// 路径: src-tauri/src/platform/kuwo.rs
// ────────────────────────────────────────────
// 功能: 酷我车载 API 适配器 — 搜索 + 播放
// 输入: reqwest::Client, keyword: &str, page: usize, page_size: usize
// 输出: Result<Vec<Song>, String> / Result<Value, String>
// 依赖: reqwest, serde_json
// 测试: curl 手动验证 (搜索: search.kuwo.cn, 播放: mobi.kuwo.cn)
// 接口: search(), play_url()
// 来源: 从音楽自由旧项目 search/kuwo.rs 迁入
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::types::Song;

const SEARCH_URL: &str = "http://search.kuwo.cn/r.s";
const PLAY_URL: &str = "https://mobi.kuwo.cn/mobi.s";
const UA_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

fn gen_user_param() -> String {
    let now = std::time::SystemTime::now();
    let ts = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut h = DefaultHasher::new();
    now.hash(&mut h);
    let r = h.finish() % 99999;
    format!("C_APK_guanwang_{}{}", ts, r)
}

/// 酷我搜索
///
/// 端点: search.kuwo.cn/r.s
/// 无需 Cookie，返回 JSON (单引号，需替换)
pub async fn search(
    client: &Client,
    keyword: &str,
    page: usize,
    page_size: usize,
) -> Result<Vec<Song>, String> {
    let resp = client
        .get(SEARCH_URL)
        .query(&[
            ("all", keyword),
            ("ft", "music"),
            ("itemset", "web_2013"),
            ("client", "kt"),
            ("pn", &(page * page_size).to_string()),
            ("rn", &page_size.to_string()),
            ("rformat", "json"),
            ("encoding", "utf8"),
        ])
        .header("User-Agent", UA_CHROME)
        .send()
        .await
        .map_err(|e| format!("搜索请求失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("搜索响应失败: {e}"))?;

    // 酷我返回单引号 JSON
    let text = text.replace('\'', "\"");
    let root: Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON 解析失败: {e}"))?;

    let songs = root
        .get("abslist")
        .and_then(|v| v.as_array())
        .map_or(vec![], |items| {
            items
                .iter()
                .filter_map(|item| {
                    let rid = item
                        .get("MUSICRID")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .replace("MUSIC_", "");
                    if rid.is_empty() {
                        return None;
                    }
                    Some(Song {
                        song_id: rid,
                        name: item
                            .get("NAME")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .replace("&nbsp;", " ")
                            .trim()
                            .to_string(),
                        singer: item
                            .get("ARTIST")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .replace("\\u0026", "&"),
                        album: item
                            .get("ALBUM")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        duration: item
                            .get("DURATION")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0),
                        source: "kuwo".to_string(),
                        cover_url: None,
                        lyric_id: None,
                    })
                })
                .collect()
        });

    Ok(songs)
}

/// 酷我获取播放地址
///
/// 端点: mobi.kuwo.cn/mobi.s
/// 车载 API，零登录零 Cookie，VIP 全通
/// 返回 { "url": "https://...", "quality": "320kmp3" }
pub async fn play_url(
    client: &Client,
    rid: &str,
    br: &str,
) -> Result<Value, String> {
    let user = gen_user_param();
    crate::debug_log::info("kuwo_play", &format!("rid={rid}, br={br}"));

    let resp = client
        .get(PLAY_URL)
        .query(&[
            ("f", "web"),
            ("source", "kwplayercar_ar_6.0.0.9_B_jiakong_vh.apk"),
            ("from", "PC"),
            ("type", "convert_url_with_sign"),
            ("br", br),
            ("rid", rid),
            ("user", &user),
        ])
        .header("User-Agent", UA_CHROME)
        .send()
        .await
        .map_err(|e| {
            crate::debug_log::error("kuwo_play", &format!("请求失败: {e}"));
            format!("播放请求失败: {e}")
        })?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| {
        crate::debug_log::error("kuwo_play", &format!("读取响应失败: {e}"));
        format!("播放响应失败: {e}")
    })?;

    crate::debug_log::info("kuwo_play", &format!("HTTP {status}, 长度={}", text.len()));

    if !status.is_success() {
        crate::debug_log::error("kuwo_play", &format!("HTTP错误 {status}, 响应前200字符: {}", &text[..text.len().min(200)]));
        return Err(format!("HTTP {status}"));
    }

    // 优先 JSON 解析
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => {
            let code = v.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            crate::debug_log::info("kuwo_play", &format!("JSON解析成功, code={code}"));
            if code != 200 {
                crate::debug_log::error("kuwo_play", &format!("业务错误 code={code}, 响应: {v}"));
                return Err(format!("code={code}"));
            }
            let url = v
                .get("data")
                .and_then(|d| d.get("url"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            if url.is_empty() {
                crate::debug_log::error("kuwo_play", &format!("URL为空, 完整响应: {v}"));
                return Err("播放 URL 为空".to_string());
            }
            crate::debug_log::info("kuwo_play", &format!("成功! URL长度={}", url.len()));
            Ok(serde_json::json!({"url": url, "quality": br}))
        }
        // fallback: key=value 纯文本（旧版 API 格式）
        Err(_) => {
            crate::debug_log::warn("kuwo_play", &format!("JSON解析失败, 尝试key=value格式"));
            for line in text.lines() {
                if let Some(u) = line.strip_prefix("url=") {
                    if !u.trim().is_empty() {
                        crate::debug_log::info("kuwo_play", &format!("key=value格式解析成功, URL长度={}", u.trim().len()));
                        return Ok(serde_json::json!({"url": u.trim(), "quality": br}));
                    }
                }
            }
            crate::debug_log::error("kuwo_play", &format!("key=value格式也失败, 响应前200字符: {}", &text[..text.len().min(200)]));
            Err(format!("无法解析播放响应: {:.200}", text))
        }
    }
}
