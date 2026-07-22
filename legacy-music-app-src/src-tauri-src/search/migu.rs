// ════════════════════════════════════════════════
// 音楽自由 — 咪咕音乐搜索适配器
// ────────────────────────────────────────────
// 功能:   调用咪咕搜索API获取歌曲列表(干净数据)
// 输入:   关键字 + 分页参数
// 输出:   Vec<Song>(含copyright标识)
// 依赖:   crate::models::Song, reqwest::Client, serde_json
// 边界:   咪咕不返回duration,通过rateFormats估算
// 备注:   copyright=1 表示该版本为原唱(纯净搜索关键信号)
// ════════════════════════════════════════════════

use reqwest::Client;
use serde_json::Value;

use crate::models::Song;
use crate::log_info;

// ── 搜索(公开) ──

/// 咪咕音乐搜索
///
/// # 参数
/// - `keyword`: 搜索关键字
/// - `page`: 页码(从0开始,咪咕API不支持分页偏移)
/// - `page_size`: 每页条数
///
/// # 返回
/// - `Ok(Vec<Song>)`: 搜索结果,含 copyright 和 rateFormats 信息
/// - `Err(String)`: 错误描述
///
/// # 备注
/// 咪咕返回数据较干净,歌名和歌手无杂音; duration通过rateFormats估算
pub async fn search(client: &Client, keyword: &str, page: usize, page_size: usize) -> Result<Vec<Song>, String> {
    let page_num = page + 1;
    let search_switch = r#"{"song":1,"album":0,"singer":0,"tagSong":0,"mvSong":0,"songlist":0,"bestShow":1}"#;

    let resp = client.get("https://pd.musicapp.migu.cn/MIGUM2.0/v1.0/content/search_all.do")
        .query(&[
            ("ua", "Android_migu"),
            ("version", "5.0.1"),
            ("text", keyword),
            ("pageNo", &page_num.to_string()),
            ("pageSize", &page_size.to_string()),
            ("searchSwitch", search_switch),
        ])
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15")
        .header("Referer", "https://music.migu.cn/")
        .send().await.map_err(|e| format!("咪咕搜索: {e}"))?;

    let data: Value = resp.json().await.map_err(|e| format!("咪咕 JSON: {e}"))?;
    let mut songs = vec![];

    if let Some(result_data) = data.get("songResultData") {
        if let Some(arr) = result_data.get("result").and_then(|v| v.as_array()) {
            for item in arr {
                let cid = item.get("copyrightId").and_then(|v| v.as_str()).unwrap_or("");
                if cid.is_empty() { continue; }

                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let singers_arr = item.get("singers").and_then(|v| v.as_array());
                let singer = singers_arr
                    .map(|arr| arr.iter()
                        .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>().join(" "))
                    .unwrap_or_default();

                let copyright = item.get("copyright").and_then(|v| v.as_str()).map(|s| s.to_string());

                let albums_arr = item.get("albums").and_then(|v| v.as_array());
                let album = albums_arr
                    .map(|arr| arr.first()
                        .and_then(|a| a.get("name").and_then(|v| v.as_str()))
                        .unwrap_or(""))
                    .unwrap_or_default()
                    .to_string();

                // rateFormats 估算duration: HQ=320kbps=40000B/s
                let dur_s = item.get("rateFormats")
                    .and_then(|rf| rf.as_array())
                    .and_then(|arr| arr.iter().find(|f| {
                        let ft = f.get("formatType").and_then(|v| v.as_str()).unwrap_or("");
                        ft == "HQ" || ft == "ZQ"
                    }))
                    .and_then(|f| f.get("size").and_then(|v| v.as_str()))
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|bytes| (bytes / 40000) as u32)
                    .unwrap_or(240);

                songs.push(Song {
                    song_id: format!("migu_{}", cid),
                    name, singer, album,
                    duration: dur_s,
                    source: "migu".to_string(),
                    mbid: None, mb_duration: None,
                    migu_copyright: copyright,
                    migu_duration: None,
                    score: None,
                    bili_cid: None, bili_cover: None,
                });
            }
        }
    }

    log_info!("migu", "返回 {} 首", songs.len());
    Ok(songs)
}
