use reqwest::Client;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::models::Song;
use crate::utils::strip_try;
use crate::log_info;

const SEARCH_URL: &str = "http://search.kuwo.cn/r.s";
const PLAY_URL: &str = "https://mobi.kuwo.cn/mobi.s";
const UA_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

fn gen_user_param() -> String {
    let now = std::time::SystemTime::now();
    let ts = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut h = DefaultHasher::new();
    now.hash(&mut h);
    let r = h.finish() % 99999;
    format!("C_APK_guanwang_{}{}", ts, r)
}

pub async fn search(client: &Client, keyword: &str, page: usize, page_size: usize) -> Result<Vec<Song>, String> {
    let resp = client.get(SEARCH_URL)
        .query(&[
            ("all", keyword), ("ft", "music"), ("itemset", "web_2013"),
            ("client", "kt"), ("pn", &(page * page_size).to_string()),
            ("rn", &page_size.to_string()), ("rformat", "json"), ("encoding", "utf8"),
        ])
        .header("User-Agent", UA_CHROME)
        .send().await.map_err(|e| format!("搜索: {e}"))?;

    let text = strip_try(&resp.text().await.map_err(|e| format!("响应: {e}"))?)
        .replace('\'', "\"");
    let root: Value = serde_json::from_str(&text).map_err(|e| format!("JSON: {e}"))?;

    Ok(root.get("abslist").and_then(|v| v.as_array()).map_or(vec![], |items| {
        items.iter().filter_map(|item| {
            let rid = item.get("MUSICRID").and_then(|v| v.as_str()).unwrap_or("").replace("MUSIC_", "");
            if rid.is_empty() { return None; }
            Some(Song {
                song_id: rid,
                name: item.get("NAME").and_then(|v| v.as_str()).unwrap_or("").replace("&nbsp;", " ").trim().to_string(),
                singer: item.get("ARTIST").and_then(|v| v.as_str()).unwrap_or("").replace("\\u0026", "&"),
                album: item.get("ALBUM").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                duration: item.get("DURATION").and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                source: "kuwo".to_string(),
                mbid: None, mb_duration: None, migu_copyright: None, migu_duration: None,
                score: None, bili_cid: None, bili_cover: None,
            })
        }).collect()
    }))
}

pub async fn play_url(client: &Client, rid: &str, br: &str) -> Result<Value, String> {
    let user = gen_user_param();
    log_info!("kuwo", "play_url rid={} br={}", rid, br);

    let resp = client.get(PLAY_URL)
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
        .send().await.map_err(|e| format!("请求: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("响应: {e}"))?;

    if !status.is_success() {
        log_info!("kuwo", "play_url status={} body={:.200}", status, &text);
        return Err(format!("HTTP {}", status));
    }

    match serde_json::from_str::<Value>(&text) {
        Ok(v) => {
            let code = v.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            if code != 200 {
                log_info!("kuwo", "play_url code={}", code);
                return Err(format!("code={}", code));
            }
            let url = v.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str()).unwrap_or("");
            if url.is_empty() {
                log_info!("kuwo", "play_url url为空");
                return Err("url为空".to_string());
            }
            log_info!("kuwo", "play_url 成功 len={}", url.len());
            Ok(serde_json::json!({"url": url, "quality": br}))
        }
        Err(e) => {
            log_info!("kuwo", "play_url JSON失败: {} body={:.150}", e, &text);
            for line in text.lines() {
                if let Some(u) = line.strip_prefix("url=") {
                    if !u.trim().is_empty() {
                        return Ok(serde_json::json!({"url": u.trim(), "quality": br}));
                    }
                }
            }
            Err(format!("解析失败: {:.200}", text))
        }
    }
}

pub fn extract_rid_from_search(text: &str) -> Result<String, String> {
    if let Some(pos) = text.find("MUSIC_") {
        let start = pos + 6;
        let end = text[start..].find(|c: char| !c.is_alphanumeric()).unwrap_or(text.len() - start);
        Ok(text[start..start + end].to_string())
    } else {
        Err("未找到RID".to_string())
    }
}
