// 独立测试：酷狗 v5/v6 播放 URL
// 源码: KuGouMusicApi (MakcRe)
// v5: GET /v5/url (notSign=true, 随机dfid)
// v6: POST /v6/priv_url (加密body, 需要tracker_param.key)
use reqwest::Client;
use serde_json::Value;
use std::collections::BTreeMap;

const LITE_APPID: u64 = 3116;
const LITE_CLIENTVER: u64 = 11430;
const SIGNKEY_SALT: &str = "185672dd44712f60bb1736df5a377e82";
const SIGN_SALT: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
const TRACKER: &str = "http://trackercdn.kugou.com";
const GATEWAY_TRACKER: &str = "http://tracker.kugou.com";
const UA: &str = "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi";

fn md5_hex(s: &str) -> String {
    format!("{:x}", md5::compute(s.as_bytes()))
}

/// 生成随机 dfid（24字符，大写字母+数字）
fn random_dfid() -> String {
    use rand::Rng;
    let chars: Vec<char> = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
    let mut rng = rand::thread_rng();
    let mut dfid = String::new();
    for _ in 0..24 {
        let idx = rng.gen_range(0..chars.len());
        dfid.push(chars[idx]);
    }
    dfid
}

/// calculateMid: MD5(guid) → hex → u128 → 十进制字符串
fn calculate_mid(guid: &str) -> String {
    let hex = md5_hex(guid);
    let val = u128::from_str_radix(&hex, 16).unwrap_or(0);
    val.to_string()
}

/// signKey: MD5(hash + SIGNKEY_SALT + appid + mid_decimal + userid)
fn sign_key(hash: &str, mid_decimal: &str, userid: u64) -> String {
    md5_hex(&format!("{}{}{}{}{}", hash, SIGNKEY_SALT, LITE_APPID, mid_decimal, userid))
}

/// signatureAndroidParams: MD5(SIGN_SALT + sorted_kv + SIGN_SALT)
/// 用于 v6 POST body 的签名
fn signature_android(body_json: &str, params: &BTreeMap<String, String>) -> String {
    let kv_str: String = params.iter()
        .map(|(k, v)| format!("{}{}", k, v))
        .collect();
    md5_hex(&format!("{}{}{}{}", SIGN_SALT, kv_str, body_json, SIGN_SALT))
}

async fn search_song(keyword: &str) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let client = Client::builder().danger_accept_invalid_certs(true).build()?;
    let resp = client
        .get("https://songsearch.kugou.com/song_search_v2")
        .query(&[
            ("keyword", keyword), ("platform", "WebFilter"), ("format", "json"),
            ("page", "1"), ("pagesize", "3"), ("userid", "-1"), ("privilege_filter", "0"),
        ])
        .header("User-Agent", UA)
        .header("Referer", "https://www.kugou.com/")
        .send().await?;
    let root: Value = serde_json::from_str(&resp.text().await?)?;
    let lists = root["data"]["lists"].as_array().ok_or("无搜索结果")?;
    if lists.is_empty() { return Err("搜索结果为空".into()); }
    let item = &lists[0];
    let hash = item["FileHash"].as_str().unwrap_or("").to_lowercase();
    let name = item["FileName"].as_str().unwrap_or("?").to_string();
    let album_id = item["AlbumID"].as_str().unwrap_or("0").to_string();
    Ok((hash, name, album_id))
}

// ═══════════════════════════════════════
// 测试1: v5 URL（notSign=true 版本）
// 源码: song_url.js
// ═══════════════════════════════════════
async fn test_v5(hash: &str, album_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试1: GET /v5/url (notSign=true) ===");
    let dfid = random_dfid();
    let mid_dec = calculate_mid(&dfid);
    let client = Client::builder().danger_accept_invalid_certs(true).build()?;

    // 构建 v5 URL 参数
    // request.js 注入的默认参数: dfid/mid/uuid/appid/clientver/clienttime
    let clienttime = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string();
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    params.insert("album_audio_id".into(), "0".into());
    params.insert("album_id".into(), album_id.into());
    params.insert("appid".into(), LITE_APPID.to_string());
    params.insert("area_code".into(), "1".into());
    params.insert("behavior".into(), "play".into());
    params.insert("cdnBackup".into(), "1".into());
    params.insert("clienttime".into(), clienttime.clone());
    params.insert("clientver".into(), LITE_CLIENTVER.to_string());
    params.insert("cmd".into(), "26".into());
    params.insert("dfid".into(), dfid.clone());
    params.insert("hash".into(), hash.to_lowercase());
    params.insert("mid".into(), mid_dec.clone());
    params.insert("page_id".into(), "967177915".into());
    params.insert("pid".into(), "411".into());
    params.insert("pidversion".into(), "3001".into());
    params.insert("ppage_id".into(), "356753938,823673182,967485191".into());
    params.insert("quality".into(), "128".into());
    params.insert("srcappid".into(), "2919".into());
    params.insert("ssa_flag".into(), "is_fromtrack".into());
    params.insert("uuid".into(), "-".into());
    params.insert("version".into(), "11430".into());

    // signKey: MD5(hash + SIGNKEY_SALT + liteAppid + mid + userid)
    let key = sign_key(hash, &mid_dec, 0);

    // 构建 URL
    let mut url = format!("{}/v5/url?", TRACKER);
    for (k, v) in &params {
        url.push_str(&format!("{}={}&", k, v));
    }
    url.push_str(&format!("key={}", key));

    println!("dfid={}", dfid);
    println!("mid (dec)={}", &mid_dec[..mid_dec.len().min(30)]);
    println!("URL: {}...", &url[..url.len().min(200)]);

    let resp = client
        .get(&url)
        .header("User-Agent", UA)
        .header("dfid", &dfid)
        .header("mid", &mid_dec)
        .header("clienttime", &clienttime)
        .header("x-router", "trackercdn.kugou.com")
        .header("kg-rc", "1")
        .header("kg-thash", "5d816a0")
        .header("kg-rec", "1")
        .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F")
        .send().await?;

    let text = resp.text().await?;
    let root: Value = serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}));
    println!("响应: {}", &text[..text.len().min(400)]);

    let errcode = root.get("errcode").map(|v| v.as_i64().unwrap_or(-1)).unwrap_or(-1);
    if errcode == 0 {
        if let Some(url_val) = root.get("url").and_then(|v| v.as_str()) {
            if !url_val.is_empty() {
                println!("\n✅ v5 成功! 播放URL: {}", url_val);
                return Ok(());
            }
        }
    }
    println!("\n❌ v5 失败: errcode={}", errcode);
    Ok(())
}

// ═══════════════════════════════════════
// 测试2: v6 priv_url（POST，加密body）
// 源码: song_url_new.js
// ═══════════════════════════════════════
async fn test_v6(hash: &str, kugou_api_mid: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试2: POST /v6/priv_url (加密body) ===");
    let dfid = random_dfid();
    let mid_dec = calculate_mid(&dfid);
    let clienttime_v6 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string();
    let client = Client::builder().danger_accept_invalid_certs(true).build()?;

    // tracker_param.key: MD5(hash + SIGNKEY_SALT + appid + KUGOU_API_MID + userid)
    let tracker_key = md5_hex(&format!(
        "{}{}{}{}{}",
        hash, SIGNKEY_SALT, LITE_APPID, kugou_api_mid, 0
    ));

    let body = serde_json::json!({
        "area_code": "1",
        "behavior": "play",
        "qualities": ["128", "320", "flac", "high", "multitrack"],
        "resource": {
            "album_audio_id": 0,
            "collect_list_id": "3",
            "collect_time": 0,
            "hash": hash.to_lowercase(),
            "id": 0,
            "page_id": 1,
            "type": "audio"
        },
        "token": "",
        "tracker_param": {
            "all_m": 1,
            "auth": "",
            "is_free_part": 0,
            "key": tracker_key,
            "module_id": 0,
            "need_climax": 1,
            "need_xcdn": 1,
            "open_time": "",
            "pid": "411",
            "pidversion": "3001",
            "priv_vip_type": "6",
            "viptoken": ""
        },
        "userid": "0",
        "vip": 0
    });

    let body_str = body.to_string();

    // 构建签名参数（不含 body 字段）
    let mut sign_params: BTreeMap<String, String> = BTreeMap::new();
    sign_params.insert("appid".into(), LITE_APPID.to_string());
    sign_params.insert("clientver".into(), LITE_CLIENTVER.to_string());
    sign_params.insert("dfid".into(), dfid.clone());
    sign_params.insert("mid".into(), mid_dec.clone());

    let sig = signature_android(&body_str, &sign_params);

    println!("dfid={}", dfid);
    println!("mid (dec)={}", &mid_dec[..mid_dec.len().min(30)]);
    println!("tracker_key={}", tracker_key);
    println!("sig={}", sig);
    println!("body: {}", &body_str);

    let resp = client
        .post(&format!("{}/v6/priv_url", GATEWAY_TRACKER))
        .header("User-Agent", UA)
        .header("dfid", &dfid)
        .header("mid", &mid_dec)
        .header("clienttime", clienttime_v6.as_str())
        .header("kg-rc", "1")
        .header("kg-thash", "5d816a0")
        .header("kg-rec", "1")
        .header("kg-rf", "B9EDA08A64250DEFFBCADDEE00F8F25F")
        .header("x-router", "tracker.kugou.com")
        .header("Content-Type", "application/json")
        .body(body_str)
        .send().await?;

    let status = resp.status();
    let text = resp.text().await?;
    let root: Value = serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}));
    println!("HTTP {} 响应: {}", status, &text[..text.len().min(600)]);

    let errcode = root.get("errcode").map(|v| v.as_i64().unwrap_or(-1)).unwrap_or(-1);
    let status_field = root.get("status").map(|v| v.as_i64().unwrap_or(-1)).unwrap_or(-1);

    if status_field == 200 || errcode == 0 {
        if let Some(url) = root.get("url").and_then(|v| v.as_str()) {
            if !url.is_empty() {
                println!("\n✅ v6 成功! 播放URL: {}", url);
                return Ok(());
            }
        }
    }
    println!("\n❌ v6 失败: status={} errcode={}", status_field, errcode);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let (hash, _name, album_id) = if args.len() >= 3 {
        (args[2].clone(), args[1].clone(), "0".into())
    } else {
        println!("=== 步骤1: 搜索 ===");
        let (h, n, a) = search_song("周杰伦 晴天").await?;
        println!("命中: {} (hash={}, album={})", n, h, a);
        (h, n, a)
    };

    // 测试1: v5 URL
    test_v5(&hash, &album_id).await?;

    // 测试2: v6 priv_url (KUGOU_API_MID=0 即未登录)
    test_v6(&hash, "0").await?;

    println!("\n测试完成");
    Ok(())
}
