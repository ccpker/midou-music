// ════════════════════════════════════════════════
// 模块: platform::kugou
// 路径: src-tauri/src/platform/kugou.rs
// ────────────────────────────────────────────
// 功能: 酷狗音乐适配器（概念版 API）
// MVP: 搜索 + 播放 URL + 歌单
// appid=3116 / clientver=11440 / salt=LnT6xpN3khm36zse0QzvmgTZ3waWdRSA
// 依赖: reqwest, serde_json
// ════════════════════════════════════════════════

use md5;
use reqwest::Client;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{KugouAuth, Song};

// ── 常量 ────────────────────────────────────────

pub const APPID: &str = "3116";
pub const CLIENTVER: &str = "11440";
/// 概念版签名 salt
pub const SALT: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
const GATEWAY: &str = "https://gateway.kugou.com";

/// 硬编码设备标识（MVP fallback）
pub const DFID: &str = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";

const UA: &str = "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

// ── 工具函数 ───────────────────────────────────

fn current_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 生成 mid: UUID → MD5(hex) → u128 → 十进制字符串
fn generate_mid() -> String {
    let uuid = uuid::Uuid::new_v4().to_string();
    let digest = md5::compute(uuid.as_bytes());
    let hex_str = format!("{:x}", digest);
    u128::from_str_radix(&hex_str, 16)
        .unwrap_or(0)
        .to_string()
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 计算酷狗 Android 签名
/// 公式: MD5(salt + 排序后的key=value串 + body + salt)
pub fn compute_signature(params: &[(&str, &str)], body: &str) -> String {
    let mut sorted: Vec<_> = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let param_str: String = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("");
    let raw = format!("{SALT}{param_str}{body}{SALT}");
    format!("{:x}", md5::compute(raw.as_bytes()))
}

/// 获取当前凭证（从 AppState 内存）
fn get_auth(auth: &KugouAuth) -> &KugouAuth {
    auth
}

// ── 搜索 ────────────────────────────────────────

pub async fn search(
    client: &Client,
    keyword: &str,
    page: usize,
    auth: &KugouAuth,
) -> Result<Vec<Song>, String> {
    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();
    // 酷狗 page 是 1-based
    let kugou_page = page + 1;

    let kugou_page_str = kugou_page.to_string();
    let page_size_str = "20".to_string();
    let clienttime_str = clienttime.to_string();

    let userid_str = auth.userid.to_string();
    let mut params: Vec<(&str, &str)> = vec![
        ("keyword", keyword),
        ("page", &kugou_page_str),
        ("pagesize", &page_size_str),
        ("platform", "AndroidFilter"),
        ("albumhide", "0"),
        ("iscorrection", "1"),
        ("nocollect", "0"),
        ("dfid", &auth.dfid),
        ("mid", &mid),
        ("uuid", &uuid),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
    ];
    // 登录态：注入 token/userid（匿名态不注入，避免空值污染签名）
    if auth.logged_in && !auth.token.is_empty() {
        params.push(("token", &auth.token));
        params.push(("userid", &userid_str));
    }

    let signature = compute_signature(&params, "");
    let mut all_params = params;
    all_params.push(("signature", &signature));

    let url = format!("{}/v3/search/song", GATEWAY);
    let resp = client
        .get(&url)
        .query(&all_params)
        .header("x-router", "complexsearch.kugou.com")
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("搜索请求失败: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("搜索响应失败: {}", e))?;

    let root: Value =
        serde_json::from_str(&text).map_err(|e| format!("搜索JSON解析失败: {}, body={:.300}", e, text))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let errmsg = root.get("error").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("API错误 status={}: {}", status, errmsg));
    }

    let lists = root
        .get("data")
        .and_then(|v| v.get("lists"))
        .and_then(|v| v.as_array());

    Ok(lists.map_or(vec![], |items| {
        items
            .iter()
            .filter_map(|item| {
                let file_hash = item
                    .get("FileHash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let album_id = item
                    .get("AlbumID")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if file_hash.is_empty() {
                    return None;
                }
                // song_id = kugou:{FileHash}|{AlbumID}
                let song_id = if album_id.is_empty() {
                    format!("kugou:{}", file_hash)
                } else {
                    format!("kugou:{}|{}", file_hash, album_id)
                };
                Some(Song {
                    song_id,
                    name: item
                        .get("FileName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    singer: item
                        .get("SingerName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    album: item
                        .get("AlbumName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    duration: item
                        .get("Duration")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    source: "kugou".to_string(),
                    cover_url: None,
                })
            })
            .collect()
    }))
}

// ── 播放 URL（/v5/url，moekoe song_url.js 同款）────────────────
//
// 关键：moekoe 播歌走 GET /v5/url（x-router=trackercdn.kugou.com），
// 用「扫码 token + dfid」即可拿到 VIP 完整播放 URL，无需 vip_token。
// 签名 = android 概念版签名 + key(signKey)，参数集合见下方。

/// 计算 signKey: MD5(hash + 盐 + appid + mid + userid)
fn sign_key(hash_lower: &str, mid: &str, userid: u64, appid: &str) -> String {
    // 概念版 signKey 盐
    const KEY_SALT: &str = "185672dd44712f60bb1736df5a377e82";
    let raw = format!("{hash_lower}{KEY_SALT}{appid}{mid}{userid}");
    format!("{:x}", md5::compute(raw.as_bytes()))
}

pub async fn play_url(
    client: &Client,
    encoded_id: &str,
    auth: &KugouAuth,
    quality: &str,
) -> Result<String, String> {
    // 解析 song_id: "kugou:{FileHash}|{AlbumID}" 或 "kugou:{FileHash}"
    let id = encoded_id.strip_prefix("kugou:").unwrap_or(encoded_id);
    let (file_hash, album_id) = if let Some(pos) = id.find('|') {
        (&id[..pos], &id[pos + 1..])
    } else {
        (id, "0")
    };

    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();
    let hash_lower = file_hash.to_lowercase();

    let clienttime_str = clienttime.to_string();
    let userid_str = auth.userid.to_string();

    // 参数集合（精确对齐 moekoe song_url.js + request.js）
    // 注意：dataMap 里 clientver 覆盖为 11430，不是默认 11440
    let mut params: Vec<(&str, String)> = vec![
        ("appid", APPID.to_string()),
        ("clientver", "11430".to_string()),
        ("clienttime", clienttime_str),
        ("dfid", auth.dfid.clone()),
        ("mid", mid.clone()),
        ("uuid", uuid),
        ("album_id", album_id.to_string()),
        ("area_code", "1".to_string()),
        ("hash", hash_lower.clone()),
        ("ssa_flag", "is_fromtrack".to_string()),
        ("version", "11430".to_string()),
        ("page_id", "967177915".to_string()), // lite
        ("quality", quality.to_string()),
        ("album_audio_id", "0".to_string()),
        ("behavior", "play".to_string()),
        ("pid", "411".to_string()), // lite
        ("cmd", "26".to_string()),
        ("pidversion", "3001".to_string()),
        ("IsFreePart", "0".to_string()),
        ("ppage_id", "356753938,823673182,967485191".to_string()), // lite
        ("cdnBackup", "1".to_string()),
        ("module", String::new()),
    ];
    // 登录态：注入 token/userid（扫码 token 即可拿到 VIP 完整 URL）
    if auth.logged_in && !auth.token.is_empty() {
        params.push(("token", auth.token.clone()));
        params.push(("userid", userid_str));
    }

    // encryptKey: true → key = signKey(hash, mid, userid, appid)
    let key = sign_key(&hash_lower, &mid, auth.userid, APPID);
    params.push(("key", key));

    // 签名（GET，body 为空）
    let params_ref: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let signature = compute_signature(&params_ref, "");
    params.push(("signature", signature));

    let url = format!("{}/v5/url", GATEWAY);

    let resp = client
        .get(&url)
        .query(&params)
        .header("x-router", "trackercdn.kugou.com")
        .header("User-Agent", "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .send()
        .await
        .map_err(|e| format!("获取播放URL失败: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("播放URL响应失败: {}", e))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("播放URL JSON解析失败: {}, body={:.300}", e, text))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let errcode = root.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
        let errmsg = root.get("errmsg").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("API错误 status={} errcode={}: {}", status, errcode, errmsg));
    }

    // 播放 URL 在顶层 url[] 数组（backupUrl[] 是备用）
    let play_url = root
        .get("url")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if play_url.is_empty() {
        Err("播放URL为空".to_string())
    } else {
        Ok(play_url.to_string())
    }
}

// ── 歌单 ────────────────────────────────────────

/// 获取用户歌单列表
///
/// 关键：签名必须用「完整参数集」（dfid/mid/uuid/appid/clientver/clienttime
/// + plat/userid/token），与参考库 request.js 一致。
/// 之前只拿 plat/userid/token 三个参数签名，导致 20006 err signature。
pub async fn fetch_playlists(
    client: &Client,
    auth: &KugouAuth,
) -> Result<Vec<Value>, String> {
    if !auth.logged_in || auth.token.is_empty() {
        return Err("请先登录酷狗".to_string());
    }

    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();

    let body = serde_json::json!({
        "userid": auth.userid,
        "token": auth.token,
        "total_ver": 979,
        "type": 2,
        "page": 1,
        "pagesize": 100
    });
    let body_str = body.to_string();
    let userid_str = auth.userid.to_string();
    let clienttime_str = clienttime.to_string();

    // 完整参数集（与参考库 request.js 默认参数 + 自定义参数一致）
    let mut params: Vec<(&str, &str)> = vec![
        ("plat", "1"),
        ("userid", &userid_str),
        ("token", &auth.token),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", &auth.dfid),
        ("mid", &mid),
        ("uuid", &uuid),
    ];
    // 签名基于完整参数集
    let signature = compute_signature(&params, &body_str);
    params.push(("signature", &signature));

    let url = format!("{}/v7/get_all_list", GATEWAY);
    let resp = client
        .post(&url)
        .query(&params)
        .header("x-router", "cloudlist.service.kugou.com")
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("获取歌单失败: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("歌单响应失败: {}", e))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("歌单JSON解析失败: {}", e))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let ec = root.get("error_code").and_then(|v| v.as_i64()).unwrap_or(0);
        return Err(format!("获取歌单失败 status={} error_code={}", status, ec));
    }

    let playlists = root
        .get("data")
        .and_then(|v| v.get("info"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(playlists)
}

/// 获取歌单内的歌曲列表
///
/// 接口：GET /pubsongs/v2/get_other_list_file_nofilt（参考 playlist_track_all.js）
/// 返回：Vec<Song>，song_id = kugou:{hash}|{album_id}
/// 分页：酷狗 begin_idx 是 0-based，每页 pagesize 首
pub async fn fetch_playlist_songs(
    client: &Client,
    auth: &KugouAuth,
    global_collection_id: &str,
    page: usize,
) -> Result<Vec<Song>, String> {
    if !auth.logged_in || auth.token.is_empty() {
        return Err("请先登录酷狗".to_string());
    }

    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();

    let pagesize: usize = 100;
    let begin_idx = page * pagesize;
    let pagesize_str = pagesize.to_string();
    let begin_idx_str = begin_idx.to_string();
    let clienttime_str = clienttime.to_string();
    let userid_str = auth.userid.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("area_code", "1"),
        ("begin_idx", &begin_idx_str),
        ("plat", "1"),
        ("type", "1"),
        ("mode", "1"),
        ("personal_switch", "1"),
        ("extend_fields", "abtags,hot_cmt,popularization"),
        ("pagesize", &pagesize_str),
        ("global_collection_id", global_collection_id),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", &auth.dfid),
        ("mid", &mid),
        ("uuid", &uuid),
        ("token", &auth.token),
        ("userid", &userid_str),
    ];
    let signature = compute_signature(&params, "");
    params.push(("signature", &signature));

    let url = format!("{}/pubsongs/v2/get_other_list_file_nofilt", GATEWAY);
    let resp = client
        .get(&url)
        .query(&params)
        .header("User-Agent", "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .send()
        .await
        .map_err(|e| format!("获取歌单歌曲失败: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("歌单歌曲响应失败: {}", e))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("歌单歌曲JSON解析失败: {}", e))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let ec = root.get("error_code").and_then(|v| v.as_i64()).unwrap_or(0);
        return Err(format!("获取歌单歌曲失败 status={} error_code={}", status, ec));
    }

    let songs = root
        .get("data")
        .and_then(|v| v.get("songs"))
        .and_then(|v| v.as_array());

    Ok(songs.map_or(vec![], |items| {
        items
            .iter()
            .filter_map(|item| {
                let file_hash = item
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let album_id = item
                    .get("album_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if file_hash.is_empty() {
                    return None;
                }
                let song_id = if album_id.is_empty() {
                    format!("kugou:{}", file_hash)
                } else {
                    format!("kugou:{}|{}", file_hash, album_id)
                };
                // 歌名带歌手后缀（如 "刘珂矣 - 半壶纱"），拆开更规范
                let raw_name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let (name, singer_from_title) = split_song_name(&raw_name);
                // 歌手优先取 singerinfo[].name，否则用歌名里拆出的
                let singer = item
                    .get("singerinfo")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| {
                        arr.iter()
                            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join("、")
                            .into()
                    })
                    .filter(|s: &String| !s.is_empty())
                    .unwrap_or(singer_from_title);
                Some(Song {
                    song_id,
                    name,
                    singer,
                    album: item
                        .get("albuminfo")
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    duration: item
                        .get("timelen")
                        .and_then(|v| v.as_u64())
                        .map(|ms| (ms / 1000) as u32)
                        .unwrap_or(0),
                    source: "kugou".to_string(),
                    cover_url: None,
                })
            })
            .collect()
    }))
}

/// 拆分 "歌手 - 歌名" 格式（酷狗歌单歌曲 name 常带歌手前缀）
fn split_song_name(raw: &str) -> (String, String) {
    if let Some(pos) = raw.find(" - ") {
        let singer = raw[..pos].trim().to_string();
        let name = raw[pos + 3..].trim().to_string();
        (name, singer)
    } else {
        (raw.to_string(), String::new())
    }
}

// ── VIP 签到 ─────────────────────────────────────
//
// moekoe 机制：扫码拿 web token → 签到领 VIP（receive_vip_listen_song）
// → 账号获得概念版 tvip（1 天）→ 就能播 VIP 完整版。
// 已实测：扫码 token 直接签到成功（status:1 error_code:0）。
// 签名 = android 概念版 + 完整参数集（同 fetch_playlists）。

/// 获取上海时区的当天日期（YYYY-MM-DD，签到 receive_day 用）
fn today_ymd_shanghai() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // UTC+8
    let total = secs + 8 * 3600;
    let days = total / 86400;
    let rem = total % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let _ = rem;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 从 1970-01-01 起的天数反推年月日（Howard Hinnant 算法）
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 签到领 1 天 VIP（youth_day_vip）
/// 接口: POST /youth/v1/recharge/receive_vip_listen_song
/// params: source_id=90139, receive_day=YYYY-MM-DD（走 query）
pub async fn sign_vip(client: &Client, auth: &KugouAuth) -> Result<serde_json::Value, String> {
    if !auth.logged_in || auth.token.is_empty() {
        return Err("请先登录酷狗".to_string());
    }

    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();
    let clienttime_str = clienttime.to_string();
    let userid_str = auth.userid.to_string();
    let receive_day = today_ymd_shanghai();

    // 完整参数集（与参考库 request.js 默认参数 + 业务参数一致）
    let mut params: Vec<(&str, &str)> = vec![
        ("source_id", "90139"),
        ("receive_day", &receive_day),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", &auth.dfid),
        ("mid", &mid),
        ("uuid", &uuid),
        ("token", &auth.token),
        ("userid", &userid_str),
    ];
    let signature = compute_signature(&params, "");
    params.push(("signature", &signature));

    let url = format!("{}/youth/v1/recharge/receive_vip_listen_song", GATEWAY);
    let resp = client
        .post(&url)
        .query(&params)
        .header("User-Agent", "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await
        .map_err(|e| format!("签到请求失败: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("签到响应失败: {e}"))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("签到JSON解析失败: {e}, body={:.300}", text))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let ec = root.get("error_code").and_then(|v| v.as_i64()).unwrap_or(0);
        let msg = root.get("error_msg").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!("签到失败 status={} error_code={} {}", status, ec, msg));
    }

    Ok(root)
}

/// 查询当前账号 VIP 状态（get_union_vip）
/// 接口: GET https://kugouvip.kugou.com/v1/get_union_vip
/// 关键: VIP 权益藏在 busi_vip[] 数组（product_type=tvip 且 is_vip=1）
pub async fn get_vip_status(client: &Client, auth: &KugouAuth) -> Result<serde_json::Value, String> {
    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();
    let clienttime_str = clienttime.to_string();
    let userid_str = auth.userid.to_string();

    let mut params: Vec<(&str, &str)> = vec![
        ("busi_type", "concept"),
        ("opt_product_types", "dvip,qvip"),
        ("product_type", "svip"),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", &auth.dfid),
        ("mid", &mid),
        ("uuid", &uuid),
    ];
    if auth.logged_in && !auth.token.is_empty() {
        params.push(("token", &auth.token));
        params.push(("userid", &userid_str));
    }
    let signature = compute_signature(&params, "");
    params.push(("signature", &signature));

    let url = "https://kugouvip.kugou.com/v1/get_union_vip";
    let resp = client
        .get(url)
        .query(&params)
        .header("User-Agent", "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi")
        .send()
        .await
        .map_err(|e| format!("查询VIP请求失败: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("查询VIP响应失败: {e}"))?;
    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("查询VIP JSON解析失败: {e}, body={:.300}", text))?;

    Ok(root)
}
