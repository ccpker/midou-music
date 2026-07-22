// 音楽自由 — B站音源适配器
// 搜索B站视频→提取音频流,作为酷我的兜底音源
// 零登录,仅需buvid3/buvid4访客Cookie

use crate::log_info;
use crate::models::Song;
use reqwest::Client;
use serde_json::Value;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
const FINGER_URL: &str = "https://api.bilibili.com/x/frontend/finger/spi";
const SEARCH_URL: &str = "https://api.bilibili.com/x/web-interface/search/type";
const VIEW_URL: &str = "https://api.bilibili.com/x/web-interface/view";

/// 获取访客Cookie (buvid3, buvid4)
pub async fn bili_fetch_buvid(client: &Client) -> Result<(String, String), String> {
    let resp: Value = client.get(FINGER_URL)
        .header("User-Agent", UA)
        .send().await.map_err(|e| format!("buvid请求: {e}"))?
        .json().await.map_err(|e| format!("buvid JSON: {e}"))?;
    let data = &resp["data"];
    let b3 = data["b_3"].as_str().unwrap_or("").to_string();
    let b4 = data["b_4"].as_str().unwrap_or("").to_string();
    if b3.is_empty() || b4.is_empty() { return Err("buvid响应为空".into()); }
    Ok((b3, b4))
}

/// B站搜索
/// 返回Song列表, song_id="bvid_cid" (cid已预取,避免播放时再查)
pub async fn bili_search(client: &Client, keyword: &str, page: usize,
    buvid3: &str, buvid4: &str) -> Result<Vec<Song>, String>
{
    let cookie = format!("buvid3={}; buvid4={}", buvid3, buvid4);
    let resp: Value = client.get(SEARCH_URL)
        .query(&[("keyword", keyword), ("search_type", "video"),
            ("page", &(page + 1).to_string()), ("page_size", "20"), ("order", "")])
        .header("User-Agent", UA)
        .header("Cookie", &cookie)
        .header("Referer", "https://search.bilibili.com/")
        .send().await.map_err(|e| format!("B站搜索: {e}"))?
        .json().await.map_err(|e| format!("B站搜索JSON: {e}"))?;

    let results = resp["data"]["result"].as_array()
        .ok_or_else(|| {
            // 诊断: 打印实际数据结构
            let s = serde_json::to_string(&resp).unwrap_or_default();
            log_info!("bili", "响应结构异常: {}...", &s[..s.len().min(300)]);
            "B站搜索无结果".to_string()
        })?;

    log_info!("bili", "搜索[{}]原始命中 {} 条", keyword, results.len());

    let mut songs = vec![];
    let mut skip_dur = 0u32;
    let mut skip_black = 0u32;
    let mut skip_cid = 0u32;
    let mut skip_empty = 0u32;

    for item in results {
        let bvid = item["bvid"].as_str().unwrap_or("");
        if bvid.is_empty() { skip_empty += 1; continue; }

        let mut title = item["title"].as_str().unwrap_or("").to_string();
        title = title.replace("<em class=\"keyword\">", "").replace("</em>", "");
        let author = item["author"].as_str().unwrap_or("").to_string();

        // ── 宽松过滤: 30秒~15分钟 ──
        let dur_str = item["duration"].as_str().unwrap_or("0:00");
        let dur_sec = parse_duration(dur_str);
        if dur_sec < 30 || dur_sec > 900 { skip_dur += 1; continue; }

        // ── 只过滤明确的非音乐内容 ──
        let title_lower = title.to_lowercase();
        let black = ["教学", "教程", "鬼畜", "前方高能", "慢摇", "伴奏",
            "piano sheet", "guitar cover"];
        let mut skip = false;
        for w in &black { if title_lower.contains(w) { skip = true; break; } }
        if skip { skip_black += 1; continue; }

        let play = item["play"].as_u64().unwrap_or(0);
        // 播放量过滤已禁用，避免误杀冷门好歌

        // ── 预取cid ──
        let cid = match bili_get_cid(client, bvid, buvid3, buvid4).await {
            Ok(c) => c,
            Err(_) => { skip_cid += 1; continue; }
        };

        let (name, singer) = parse_title(&title, &author);

        let cover = item["pic"].as_str().unwrap_or("");
        let cover_url = if cover.starts_with("//") {
            format!("http:{}", cover)
        } else {
            cover.to_string()
        };

        songs.push(Song {
            song_id: format!("{}|{}", bvid, cid),
            name,
            // B站以UP主为歌手; parse_title 失败时 fallback 到 author
            singer: if singer.is_empty() { author.clone() } else { singer },
            album: "B站视频".to_string(),
            duration: dur_sec,
            source: "bilibili".into(),
            mbid: None, mb_duration: None,
            migu_copyright: None, migu_duration: None,
            score: Some(((play.min(999999) / 100) as u32).max(10)),
            bili_cid: Some(cid.to_string()),
            bili_cover: Some(cover_url),
        });
    }

    log_info!("bili", "过滤后剩余 {} 首 (时长{} | 黑名{} | cid{} | 空bvid{})",
        songs.len(), skip_dur, skip_black, skip_cid, skip_empty);

    if songs.is_empty() { return Err("B站无符合条件的音频".into()); }
    Ok(songs)
}

/// 获取视频cid(分P音频标识)
pub async fn bili_get_cid(client: &Client, bvid: &str,
    buvid3: &str, buvid4: &str) -> Result<u64, String>
{
    let cookie = format!("buvid3={}; buvid4={}", buvid3, buvid4);
    let resp: Value = client.get(VIEW_URL)
        .query(&[("bvid", bvid)])
        .header("User-Agent", UA)
        .header("Cookie", &cookie)
        .send().await.map_err(|e| format!("B站view: {e}"))?
        .json().await.map_err(|e| format!("B站view JSON: {e}"))?;
    resp["data"]["cid"].as_u64()
        .ok_or_else(|| format!("未找到cid: {bvid}"))
}

/// 获取音频流URL (fnval=4048 → DASH独立音频)
/// 返回 (url, host域名)
pub async fn bili_get_audio_url(client: &Client, bvid: &str, cid: u64,
    buvid3: &str, buvid4: &str) -> Result<(String, String), String>
{
    let cookie = format!("buvid3={}; buvid4={}", buvid3, buvid4);
    let resp: Value = client.get("https://api.bilibili.com/x/player/playurl")
        .query(&[("bvid", bvid), ("cid", &cid.to_string()),
            ("fnval", "4048"), ("platform", "html5")])
        .header("User-Agent", UA)
        .header("Cookie", &cookie)
        .header("Referer", format!("https://www.bilibili.com/video/{bvid}"))
        .send().await.map_err(|e| format!("B站playurl: {e}"))?
        .json().await.map_err(|e| format!("B站playurl JSON: {e}"))?;

    if resp["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(resp["message"].as_str().unwrap_or("未知错误").into());
    }

    // 优先DASH独立音频
    if let Some(audio_list) = resp["data"]["dash"]["audio"].as_array() {
        if !audio_list.is_empty() {
            // 选最高码率(id=30280 > 30232 > 30216)
            let mut best = &audio_list[0];
            let mut best_br: u64 = 0;
            for aud in audio_list {
                let br = aud["bandwidth"].as_u64().unwrap_or(0);
                if br > best_br { best_br = br; best = aud; }
            }
            let url = best["base_url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() {
                let host = url.split("://").nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("upos-sz.bilivideo.com")
                    .to_string();
                return Ok((url, host));
            }
        }
    }

    // 降级到durl(混合流,无DASH的老视频)
    if let Some(durl) = resp["data"]["durl"].as_array() {
        if !durl.is_empty() {
            let url = durl[0]["url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() {
                let host = url.split("://").nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("upos-sz.bilivideo.com")
                    .to_string();
                return Ok((url, host));
            }
        }
    }

    Err("未找到音频流".into())
}

/// 获取视频流URL (fnval=4048 → DASH独立视频)
/// 返回 (url, host域名)
pub async fn bili_get_video_url(client: &Client, bvid: &str, cid: u64,
    buvid3: &str, buvid4: &str) -> Result<(String, String), String>
{
    let cookie = format!("buvid3={}; buvid4={}", buvid3, buvid4);
    let resp: Value = client.get("https://api.bilibili.com/x/player/playurl")
        .query(&[("bvid", bvid), ("cid", &cid.to_string()),
            ("fnval", "4048"), ("platform", "html5")])
        .header("User-Agent", UA)
        .header("Cookie", &cookie)
        .header("Referer", format!("https://www.bilibili.com/video/{bvid}"))
        .send().await.map_err(|e| format!("B站playurl: {e}"))?
        .json().await.map_err(|e| format!("B站playurl JSON: {e}"))?;

    if resp["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(resp["message"].as_str().unwrap_or("未知错误").into());
    }

    // 优先DASH独立视频
    if let Some(video_list) = resp["data"]["dash"]["video"].as_array() {
        if !video_list.is_empty() {
            // 选最高码率视频
            let mut best = &video_list[0];
            let mut best_br: u64 = 0;
            for vid in video_list {
                let br = vid["bandwidth"].as_u64().unwrap_or(0);
                if br > best_br { best_br = br; best = vid; }
            }
            let url = best["base_url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() {
                let host = url.split("://").nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("upos-sz.bilivideo.com")
                    .to_string();
                return Ok((url, host));
            }
        }
    }

    // 降级到durl(混合流)
    if let Some(durl) = resp["data"]["durl"].as_array() {
        if !durl.is_empty() {
            let url = durl[0]["url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() {
                let host = url.split("://").nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("upos-sz.bilivideo.com")
                    .to_string();
                return Ok((url, host));
            }
        }
    }

    Err("未找到视频流".into())
}

/// 流式代理B站音频(解决CORS + URL过期问题)
pub async fn bili_stream_audio(client: &Client, bvid: &str, cid: u64,
    buvid3: &str, buvid4: &str) -> Result<(Vec<u8>, String), String>
{
    let (url, host) = bili_get_audio_url(client, bvid, cid, buvid3, buvid4).await?;
    let mime = if url.contains(".m4s") { "audio/mp4" } else { "audio/mpeg" };

    let data = client.get(&url)
        .header("User-Agent", UA)
        .header("Referer", format!("https://www.bilibili.com/video/{bvid}"))
        .header("Host", &host)
        .header("Range", "bytes=0-")
        .send().await.map_err(|e| format!("下载音频: {e}"))?
        .bytes().await.map_err(|e| format!("读取音频: {e}"))?
        .to_vec();

    Ok((data, mime.to_string()))
}

// ── 工具函数 ──

fn parse_duration(s: &str) -> u32 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: u32 = parts[0].parse().unwrap_or(0);
            let s: u32 = parts[1].parse().unwrap_or(0);
            m * 60 + s
        }
        3 => {
            let h: u32 = parts[0].parse().unwrap_or(0);
            let m: u32 = parts[1].parse().unwrap_or(0);
            let s: u32 = parts[2].parse().unwrap_or(0);
            h * 3600 + m * 60 + s
        }
        _ => 0,
    }
}

/// 从标题解析歌名+歌手
/// 优先级: 《XX》> " - " 分隔 > 标题=歌名+UP主=歌手
fn parse_title(title: &str, author: &str) -> (String, String) {
    let t = title.trim();
    let t = if let Some(idx) = t.find("】") { &t[idx+3..] } else { t };
    let t = t.trim();

    // ── 1. 《XX》优先：歌名在书名号内 ──
    if let Some(start_byte) = t.rfind('《') {
        let after_start = &t[start_byte+3..];  // 3 = '《'.len_utf8()
        if let Some(end_byte) = after_start.find('》') {
            let song = after_start[..end_byte].trim();
            if !song.is_empty() {
                let before = t[..start_byte].trim();
                let after_right = after_start[end_byte+3..].trim();
                // 尝试取《》前面的部分作为歌手 (如 "周杰伦《青花瓷》")
                let artist_before = trim_artist(before);
                if is_artist_like(artist_before) {
                    return (clean_name(song), clean_name(artist_before));
                }
                // 尝试取《》后面到第一个分隔符 (如 "《青花瓷》- 周杰伦")
                let after_first = after_right.split(|c: char| c == '-' || c == '—' || c == '｜' || c == '|').next().unwrap_or("").trim();
                let after_f = trim_artist(after_first);
                if is_artist_like(after_f) {
                    return (clean_name(song), clean_name(after_f));
                }
                // 都不像 → song来自《》, 歌手用UP主
                return (clean_name(song), author.to_string());
            }
        }
    }

    // ── 2. " - " 分隔: B站惯例 "歌手 - 歌名" ──
    if let Some(idx) = t.rfind(" - ") {
        let left = t[..idx].trim();
        let right = t[idx+3..].trim();
        // 较短的一侧通常是歌手
        if left.len() <= right.len() && is_artist_like(left) {
            return (clean_name(right), clean_name(left));
        }
        if is_artist_like(right) && right.len() < left.len() {
            return (clean_name(left), clean_name(right));
        }
        // 兜底: left=歌手 right=歌名 (B站惯例)
        if is_artist_like(left) {
            return (clean_name(right), clean_name(left));
        }
    }

    // ── 3. 无结构：标题当歌名, UP主当歌手 ──
    (clean_name(t), author.to_string())
}

fn is_artist_like(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && s.len() < 30 && s.chars().any(|c| c as u32 > 127)
}

fn trim_artist(s: &str) -> &str {
    s.trim().trim_matches(|c: char| c == '｜' || c == '-' || c == '—' || c == '\'' || c == '"' || c == '·' || c == '•').trim()
}

fn clean_name(s: &str) -> String {
    let mut s = s.to_string();
    // 去掉 【】「」
    s = s.chars().filter(|c| !matches!(c, '【' | '】' | '「' | '」')).collect();
    // 去掉括号及内容: (MV), [4K修复], （官方版）等
    s = remove_brackets(&s);
    // 去尾部标签
    s = s.trim_end_matches("MV")
         .trim_end_matches("Official")
         .trim_end_matches("official")
         .trim_end_matches("Live")
         .trim()
         .to_string();
    s
}

fn remove_brackets(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut depth: i32 = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '(' || c == '[' || c == '（' {
            depth += 1;
            i += 1;
            continue;
        }
        if c == ')' || c == ']' || c == '）' {
            if depth > 0 { depth -= 1; }
            i += 1;
            continue;
        }
        if depth == 0 { result.push(c); }
        i += 1;
    }
    result.trim().to_string()
}
