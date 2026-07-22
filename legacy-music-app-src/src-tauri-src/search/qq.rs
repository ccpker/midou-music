// 音楽自由 — QQ音乐音源适配器
// 搜索使用旧版 soso API (c.y.qq.com), 该 endpoint 仍能返回完整 singer/albumname 字段
// 桌面版 u.y.qq.com/cgi-bin/musicu.fcg 的 DoSearchForQQMusicDesktop 现在返回 500001 (已失效)
use crate::models::Song;
use crate::log_info;
use reqwest::Client;
use serde_json::Value;

const UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1";
const SEARCH_URL: &str = "http://c.y.qq.com/soso/fcgi-bin/search_for_qq_cp";
const VKEY_URL: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";

/// QQ搜索（返回所有结果; 旧版 soso API - singer/albumname 都是字符串）
pub async fn search(client: &Client, keyword: &str, page: usize, page_size: usize) -> Result<Vec<Song>, String> {
    let resp: Value = client
        .get(SEARCH_URL)
        .query(&[
            ("w", keyword),
            ("format", "json"),
            ("p", &(page + 1).to_string()),
            ("n", &page_size.to_string()),
        ])
        .header("User-Agent", UA)
        .header("Referer", "http://m.y.qq.com")
        .send()
        .await
        .map_err(|e| format!("QQ搜索: {e}"))?
        .json()
        .await
        .map_err(|e| format!("QQ搜索JSON: {e}"))?;

    let list = resp
        .get("data")
        .and_then(|d| d.get("song"))
        .and_then(|s| s.get("list"))
        .and_then(|l| l.as_array())
        .ok_or_else(|| "QQ搜索无结果".to_string())?;

    let mut songs = vec![];
    for item in list {
        let mid = item.get("songmid").and_then(|v| v.as_str()).unwrap_or("");
        if mid.is_empty() {
            continue;
        }
        // 标记VIP付费歌曲（pay.payplay==1），不过滤，前端显示VIP徽章
        let payplay = item.pointer("/pay/payplay").and_then(|v| v.as_i64()).unwrap_or(0);
        let is_vip = payplay == 1;

        // singer 数组 → 用 "、" 拼接
        let singer = item
            .get("singer")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join("、")
            })
            .unwrap_or_default();

        songs.push(Song {
            song_id: mid.to_string(),
            name: item
                .get("songname")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            singer,
            album: item
                .get("albumname")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            duration: item
                .get("interval")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u32,
            source: "qq".to_string(),
            mbid: None,
            mb_duration: None,
            migu_copyright: None,
            migu_duration: None,
            score: if is_vip { Some(1) } else { None },
            bili_cid: None,
            bili_cover: None,
        });
    }
    let vip_count = songs.iter().filter(|s| s.score == Some(1)).count();
    log_info!("qq", "QQ搜索返回 {} 首 (其中VIP {} 首)", songs.len(), vip_count);
    Ok(songs)
}

/// 获取QQ音乐播放URL（vkey方案）
pub async fn play_url(client: &Client, song_mid: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "req_1": {
            "module": "vkey.GetVkeyServer",
            "method": "CgiGetVkey",
            "param": {
                "guid": "10000",
                "songmid": [song_mid],
                "songtype": [0],
                "uin": "0",
                "loginflag": 1,
                "platform": "20"
            }
        }
    });
    let resp: Value = client
        .post(VKEY_URL)
        .header("User-Agent", UA)
        .header("Referer", "https://y.qq.com/")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("QQ Vkey: {e}"))?
        .json()
        .await
        .map_err(|e| format!("QQ Vkey JSON: {e}"))?;

    let purl = resp
        .get("req_1")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("midurlinfo"))
        .and_then(|m| m.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("purl"))
        .and_then(|p| p.as_str())
        .unwrap_or("");

    if purl.is_empty() {
        return Err("QQ vkey返回空purl（可能VIP或版权限制）".into());
    }
    Ok(format!("http://ws.stream.qqmusic.qq.com/{purl}"))
}
