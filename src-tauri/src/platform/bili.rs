// ════════════════════════════════════════════════
// 模块: platform::bili
// 路径: src-tauri/src/platform/bili.rs
// ────────────────────────────────────────────
// 功能: B站音频搜索 + 播放
// 战法: 历史记录 "B站音频战法"（spi → search → view → playurl）
//
// 音频类型 A（音乐区）:
//   → search → song/infonet → audio_url
//
// 音频类型 B（视频区 / 音频版）:
//   → search → view → playurl(fnval=4048)
//
// 零登录，仅需 buvid3/buvid4 访客 Cookie
// 歌词: B站不带歌词，依赖 LRCLIB（上层处理）
//
// 依赖: reqwest, serde_json
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;
use crate::debug_log;
use crate::types::Song;

// ── 常量 ────────────────────────────────────────

const BILI_SEARCH: &str = "https://api.bilibili.com/x/web-interface/search/type";
const BILI_VIEW: &str = "https://api.bilibili.com/x/player/pagelist";
const BILI_PLAYURL: &str = "https://api.bilibili.com/x/player/v2";
const BILI_SONG_INFO: &str = "https://www.bilibili.com/music-service-c/song/infonet";
const BILI_AUDIO_URL: &str = "https://www.bilibili.com/music-service-c/audioUrl";
const BILI_SPI: &str = "https://api.bilibili.com/x/frontend/finger/spi";

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

/// 从 spi 接口获取真实的 buvid3/buvid4
async fn get_buvids(client: &Client) -> Result<(String, String), String> {
    let resp = client
        .get(BILI_SPI)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("B站 spi 请求失败: {e}"))?;
    
    let text = resp.text().await.map_err(|e| format!("B站 spi 响应失败: {e}"))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("B站 spi JSON 解析失败: {e}"))?;
    
    let data = root.get("data").ok_or("spi 返回无 data 字段")?;
    let buvid3 = data.get("b_3")
        .and_then(|v| v.as_str())
        .ok_or("spi 返回无 b_3 字段")?
        .to_string();
    let buvid4 = data.get("b_4")
        .and_then(|v| v.as_str())
        .ok_or("spi 返回无 b_4 字段")?
        .to_string();
    
    debug_log::info("bili_spi", &format!("buvid3={}, buvid4={}", buvid3, buvid4));
    Ok((buvid3, buvid4))
}

// ── 搜索 ────────────────────────────────────────

/// B站搜索
///
/// search_type = 3 → 音频（不含视频）
/// 仅返回 UP主上传的纯音频（music_area）
///
/// song_id 格式:
///   - 音乐区:  "au{MUSIC_ID}"   （纯音频，有封面）
///   - 视频区:  "bv{BVID}"       （音频版视频，封面取自视频帧）
pub async fn search(
    client: &Client,
    keyword: &str,
    _page: usize,
    page_size: usize,
) -> Result<Vec<Song>, String> {
    // ★ 先获取真实的 buvid3/buvid4
    let (buvid3, buvid4) = get_buvids(client).await?;
    debug_log::info("bili_search", &format!("获取 buvids 成功，开始搜索: {}", keyword));
    
    let resp = client
        .get(BILI_SEARCH)
        .query(&[
            ("search_type", "video"),    // video = 视频区（含音乐视频），3 = 音乐区（已停用）
            ("keyword", keyword),
            ("page", "1"),
            ("pagesize", &page_size.to_string()),
        ])
        .header("User-Agent", UA)
        .header("Referer", "https://search.bilibili.com/")
        .header("Origin", "https://search.bilibili.com")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-site")
        .header("Sec-Fetch-Dest", "empty")
        .header("Cookie", &format!("buvid3={}; buvid4={}", buvid3, buvid4))
        .send()
        .await
        .map_err(|e| format!("B站搜索请求失败: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("B站响应失败: {e}"))?;
    debug_log::info("bili_search", &format!("响应长度: {} 字节", text.len()));
    
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("B站 JSON 解析失败: {e} / 原始: {:.100}", text))?;

    let code = root.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let msg = root.get("message").and_then(|v| v.as_str()).unwrap_or("unknown");
    debug_log::info("bili_search", &format!("code={}, msg={}", code, msg));
    
    if code != 0 {
        return Err(format!("B站返回 code={code}: {}", msg));
    }

    let songs = root
        .get("data")
        .and_then(|v| v.get("result"))
        .and_then(|v| v.as_array())
        .map_or(vec![], |items| {
            debug_log::info("bili_search", &format!("result 数组长度: {}", items.len()));
            items
                .iter()
                .filter_map(|item| {
                    let title = item.get("title")?.as_str()?.to_string();
                    // 标题格式: "<em>关键词</em> 歌曲名 - 歌手名"，去掉 em 标签
                    let title = title.replace("<em>", "").replace("</em>", "");

                    // 取 arcurl 中的 av 号或音频 ID
                    let bvid = item.get("bvid").and_then(|v| v.as_str());
                    let music_id = item.get("music_id").and_then(|v| v.as_str());

                    // song_id 优先用 music_id（纯音频），否则用 bvid（音频版视频）
                    let song_id = music_id
                        .map(|id| format!("au{}", id))
                        .or_else(|| bvid.map(|bv| format!("bv{}", bv)))?;

                    // 时长: 取不到则估算
                    let duration = item
                        .get("duration")
                        .and_then(|v| v.as_str())
                        .and_then(|s| parse_duration(s))
                        .unwrap_or(0);

                    Some(Song {
                        song_id,
                        name: extract_title(&title),
                        singer: extract_singer(&title),
                        album: item.get("author").and_then(|v| v.as_str()).unwrap_or("B站音频").to_string(),
                        duration,
                        source: "bili".to_string(),
                        cover_url: item.get("cover").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        lyric_id: None,
                    })
                })
                .collect()
        });

    Ok(songs)
}

// ── 播放 URL ────────────────────────────────────

/// B站获取音频流地址
///
/// 路由:
///   au{MUSIC_ID}  → 音乐区纯音频（music-service-c/audioUrl）
///   bv{BVID}      → 视频区音频版（DASH audio，取 audio 流非 video）
///
/// 返回: { "url": "...", "quality": "..." }
pub async fn play_url(
    client: &Client,
    song_id: &str,
    _quality: &str,
) -> Result<Value, String> {
    if song_id.starts_with("au") {
        // ── 音乐区纯音频 ───────────────────────────
        let music_id = &song_id[2..];

        // 获取歌曲详情（含真实 musicId + cover）
        let info_resp = client
            .get(BILI_SONG_INFO)
            .query(&[
                ("musicId", music_id),
                ("privilege", "2"),
                ("upgrade", "1"),
            ])
            .header("User-Agent", UA)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .map_err(|e| format!("B站歌曲信息请求失败: {e}"))?;

        let info_text = info_resp.text().await.map_err(|e| format!("B站歌曲信息响应失败: {e}"))?;
        let info: Value = serde_json::from_str(&info_text)
            .map_err(|e| format!("B站歌曲信息 JSON 解析失败: {e}"))?;

        let real_id = info
            .get("data")
            .and_then(|d| d.get("musicId"))
            .and_then(|v| v.as_str())
            .ok_or("B站歌曲信息 missing musicId")?;

        // 获取音频 URL
        let url_resp = client
            .get(BILI_AUDIO_URL)
            .query(&[
                ("musicId", real_id),
                ("privilege", "2"),
                ("upgrade", "1"),
            ])
            .header("User-Agent", UA)
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await
            .map_err(|e| format!("B站音频URL请求失败: {e}"))?;

        let url_text = url_resp.text().await.map_err(|e| format!("B站音频URL响应失败: {e}"))?;
        let url_data: Value = serde_json::from_str(&url_text)
            .map_err(|e| format!("B站音频URL JSON 解析失败: {e}"))?;

        let audio_url = url_data
            .get("data")
            .and_then(|d| d.get("cdns"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!(
                    "B站音频URL为空 cdns=NULL: {:.100}",
                    url_text
                )
            })?;

        Ok(serde_json::json!({
            "url": audio_url,
            "quality": "high",
        }))

    } else if song_id.starts_with("bv") {
        // ── 视频区音频版 ────────────────────────────
        let bvid = &song_id[2..];

        // 获取分P列表
        let view_resp = client
            .get(BILI_VIEW)
            .query(&[("bvid", bvid)])
            .header("User-Agent", UA)
            .header("Referer", &format!("https://www.bilibili.com/video/{}", bvid))
            .send()
            .await
            .map_err(|e| format!("B站分P列表请求失败: {e}"))?;

        let view_text = view_resp.text().await.map_err(|e| format!("B站分P列表响应失败: {e}"))?;
        let view_data: Value = serde_json::from_str(&view_text)
            .map_err(|e| format!("B站分P列表 JSON 解析失败: {e}"))?;

        let cid = view_data
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|p| p.get("cid"))
            .and_then(|v| v.as_i64())
            .ok_or("B站 cid not found")?;

        // 获取播放地址（fnval=4048 = 全格式，含 audio 流）
        let play_resp = client
            .get(BILI_PLAYURL)
            .query(&[
                ("bvid", bvid),
                ("cid", &cid.to_string()),
                ("fnval", "4048"),
                ("fnver", "0"),
                ("fourk", "1"),
            ])
            .header("User-Agent", UA)
            .header("Referer", &format!("https://www.bilibili.com/video/{}", bvid))
            .send()
            .await
            .map_err(|e| format!("B站播放地址请求失败: {e}"))?;

        let play_text = play_resp.text().await.map_err(|e| format!("B站播放地址响应失败: {e}"))?;
        let play_data: Value = serde_json::from_str(&play_text)
            .map_err(|e| format!("B站播放地址 JSON 解析失败: {e}"))?;

        // 从 DASH.audio[0].baseUrl 取音频
        let audio_url = extract_dash_audio(&play_data)
            .ok_or_else(|| "B站音频流地址为空 (dash.audio + dash.video 均无)".to_string())?;

        Ok(serde_json::json!({
            "url": audio_url,
            "quality": "high",
        }))
    } else {
        Err(format!("未知的 B站 song_id 格式: {}", song_id))
    }
}

// ── 工具函数 ────────────────────────────────────

/// 从 "歌曲名 - 歌手名" 格式提取歌名
fn extract_title(s: &str) -> String {
    s.split(" - ").next().unwrap_or(s).trim().to_string()
}

/// 从 "歌曲名 - 歌手名" 格式提取歌手
fn extract_singer(s: &str) -> String {
    s.split(" - ").nth(1).unwrap_or("B站音频").trim().to_string()
}

/// 从 play_data 中提取 DASH audio URL
/// 优先: dash.audio[0].baseUrl
/// 降级: dash.video[0].baseUrl（部分音频版无独立 audio 流）
fn extract_dash_audio(play_data: &Value) -> Option<String> {
    let audio_url = play_data
        .get("data")
        .and_then(|d| d.get("dash"))
        .and_then(|d| d.get("audio"))
        .and_then(|a| a.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|a| a.get("baseUrl"))
        .and_then(|v| v.as_str());
    if let Some(u) = audio_url {
        return Some(u.to_string());
    }
    // 降级取 video 流
    play_data
        .get("data")
        .and_then(|d| d.get("dash"))
        .and_then(|d| d.get("video"))
        .and_then(|a| a.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|a| a.get("baseUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 解析 "3:45" 格式时长 → 秒
fn parse_duration(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: u32 = parts[0].parse().ok()?;
            let s: u32 = parts[1].parse().ok()?;
            Some(m * 60 + s)
        }
        3 => {
            let h: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let s: u32 = parts[2].parse().ok()?;
            Some(h * 3600 + m * 60 + s)
        }
        _ => None,
    }
}
