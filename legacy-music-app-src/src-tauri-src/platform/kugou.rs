// ════════════════════════════════════════════════
// 音楽自由 — 酷狗音乐概念版插件
// ────────────────────────────────────────────
// 实现 PlatformPlugin trait
// 基于酷狗概念版 lite API (appid=3116, clientver=11440)
// MVP: 搜索 + 播放URL (无登录/设备注册)
// ════════════════════════════════════════════════

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::log_info;
use crate::models::Song;
use crate::platform::{PlatformPlugin, PluginCapability, PluginMeta};

// ── 常量 ──

const APPID: &str = "3116";
const CLIENTVER: &str = "11440";
/// Android 签名 salt（概念版）
const SALT: &str = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA";
const GATEWAY: &str = "https://gateway.kugou.com";

/// 硬编码 dfid（MVP 不实现设备注册，后续可接入 r_register_dev）
const DFID: &str = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";

const UA: &str = "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

// ── 辅助函数 ──

fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 生成 mid: UUID → MD5(hex 32位) → u128 → 十进制字符串
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
fn compute_signature(params: &[(&str, &str)], body: &str) -> String {
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

// ── 底层搜索实现 ──

pub async fn search_impl(
    client: &Client,
    keyword: &str,
    page: usize,
    page_size: usize,
    auth: &crate::config::KugouAuth,
) -> Result<Vec<Song>, String> {
    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();
    // kugou page 是 1-based
    let kugou_page = page + 1;

    // P8-2 C2: 登录态时用注册 dfid 替换硬编码
    let dfid = if auth.logged_in { &auth.dfid } else { DFID };

    let kugou_page_str = kugou_page.to_string();
    let page_size_str = page_size.to_string();
    let clienttime_str = clienttime.to_string();
    let params: Vec<(&str, &str)> = vec![
        ("keyword", keyword),
        ("page", &kugou_page_str),
        ("pagesize", &page_size_str),
        ("platform", "AndroidFilter"),
        ("albumhide", "0"),
        ("iscorrection", "1"),
        ("nocollect", "0"),
        ("dfid", dfid),
        ("mid", &mid),
        ("uuid", &uuid),
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
    ];

    let signature = compute_signature(&params, "");
    let mut all_params: Vec<(&str, &str)> = params;
    all_params.push(("signature", &signature));

    let url = format!("{GATEWAY}/v3/search/song");
    let resp = client
        .get(&url)
        .query(&all_params)
        .header("x-router", "complexsearch.kugou.com")
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("搜索请求失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("搜索响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("搜索JSON解析失败: {e}, body={:.300}", text))?;

    let status = root
        .get("status")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if status != 1 {
        let errmsg = root
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
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
                // song_id = FileHash|AlbumID（竖线分隔，供 play_url 解析）
                let song_id = if album_id.is_empty() {
                    file_hash.clone()
                } else {
                    format!("{file_hash}|{album_id}")
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
                    album: String::new(),
                    duration: item
                        .get("Duration")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    source: "kugou".to_string(),
                    mbid: None,
                    mb_duration: None,
                    migu_copyright: None,
                    migu_duration: None,
                    score: None,
                    bili_cid: None,
                    bili_cover: None,
                })
            })
            .collect()
    }))
}

// ── 底层播放URL实现 ──

pub async fn play_url_impl(
    client: &Client,
    encoded_id: &str,
    _quality: &str,
    auth: &crate::config::KugouAuth,
) -> Result<Value, String> {
    // 解析 song_id: "FileHash" 或 "FileHash|AlbumID"
    let (file_hash, album_id) = if let Some(pos) = encoded_id.find('|') {
        (&encoded_id[..pos], &encoded_id[pos + 1..])
    } else {
        (encoded_id, "0")
    };

    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();

    // P8-2 C2: 登录态时用注册 dfid
    let dfid = if auth.logged_in { &auth.dfid } else { DFID };

    let body_json = serde_json::json!({
        "appid": 3116,
        "area_code": 1,
        "behavior": "play",
        "clientver": 11440,
        "need_hash_offset": 1,
        "relate": 1,
        "support_verify": 1,
        "resource": [{
            "type": "audio",
            "page_id": 0,
            "hash": file_hash,
            "album_id": album_id
        }],
        "qualities": ["128", "320", "flac", "high", "viper_atmos", "viper_tape", "viper_clear", "super", "multitrack"]
    });

    let body_str = body_json.to_string();

    let clienttime_str = clienttime.to_string();

    let mut all_params: Vec<(&str, &str)> = vec![
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", dfid),
        ("mid", &mid),
        ("uuid", &uuid),
    ];

    let signature = compute_signature(&all_params, &body_str);
    all_params.push(("signature", &signature));

    let url = format!("{GATEWAY}/v2/get_res_privilege/lite");

    log_info!("kugou", "play_url hash={file_hash} album_id={album_id}");

    let resp = client
        .post(&url)
        .query(&all_params)
        .header("x-router", "media.store.kugou.com")
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .json(&body_json)
        .send()
        .await
        .map_err(|e| format!("获取播放URL失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("播放URL响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("播放URL JSON解析失败: {e}, body={:.300}", text))?;

    let status = root
        .get("status")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if status != 1 {
        let errmsg = root
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("API错误 status={}: {}", status, errmsg));
    }

    let audio_list = root
        .get("data")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("relate_audio"))
        .and_then(|v| v.as_array());

    let best = audio_list
        .into_iter()
        .flatten()
        .max_by_key(|v| v.get("bitrate").and_then(|b| b.as_u64()).unwrap_or(0));

    match best {
        None => Err("未找到可用音频".to_string()),
        Some(item) => {
            let play_url = item
                .get("url")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if play_url.is_empty() {
                Err("播放URL为空".to_string())
            } else {
                let bitrate = item
                    .get("bitrate")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                log_info!("kugou", "play_url 成功 bitrate={bitrate} url_len={}", play_url.len());
                Ok(serde_json::json!({"url": play_url, "quality": bitrate.to_string()}))
            }
        }
    }
}

// ── P10: 酷狗下载 — 复用 play_url_impl 获取 URL ──

/// 获取酷狗下载链接（复用 play_url_impl 返回的音频 URL）
pub async fn download_url_impl(
    client: &Client,
    encoded_id: &str,
    quality: &str,
    auth: &crate::config::KugouAuth,
) -> Result<String, String> {
    let play_result = play_url_impl(client, encoded_id, quality, auth).await?;
    let url = play_result.get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("").to_string();
    if url.is_empty() { return Err("酷狗播放URL为空".into()); }
    log_info!("kugou", "download_url 获取成功 {}", url.len());
    Ok(url)
}

// ── P9: 酷狗歌词搜索 (Step1 + Step2) ──

/// P9: 酷狗歌词搜索 (Step1 + Step2)
///
/// # 参数
/// - name: 歌名
/// - artist: 歌手名
/// - duration_ms: 歌曲时长(毫秒)，搜索时必传
/// - file_hash: FileHash（选填，提高命中率）
///
/// # 返回
/// - Ok(lrc_text): LRC 格式歌词
/// - Err(msg): 失败原因
pub async fn lyrics_impl(
    client: &Client,
    name: &str,
    artist: &str,
    duration_ms: u32,
    file_hash: &str,
) -> Result<String, String> {
    // Step 1: 搜索歌词
    let search_url = format!(
        "http://lyrics.kugou.com/search?ver=1&man=yes&client=pc&keyword={}&duration={}&hash={}",
        urlencoding(name),
        duration_ms,
        file_hash,
    );
    let resp = client.get(&search_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("酷狗歌词搜索请求: {e}"))?;

    let text = resp.text().await
        .map_err(|e| format!("酷狗歌词搜索响应: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("酷狗歌词JSON解析: {e}"))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 200 {
        return Err("酷狗歌词未找到".into());
    }

    let candidates = root.get("candidates").and_then(|v| v.as_array());
    let first = candidates
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
    let dl_resp = client.get(&dl_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("酷狗歌词下载: {e}"))?;

    let lrc = dl_resp.text().await
        .map_err(|e| format!("酷狗歌词内容: {e}"))?;

    if lrc.trim().is_empty() {
        return Err("酷狗歌词内容为空".into());
    }

    log_info!("kugou", "歌词获取成功: {} - {}, {} 行", name, artist, lrc.lines().count());
    Ok(lrc)
}

/// 简单 URL 编码（中文 → %XX）
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

// ── 扫码登录 (纯 HTTP GET，零加密) ──

/// 获取二维码 key
pub async fn qr_key(client: &Client) -> Result<String, String> {
    let url = format!(
        "https://login-user.kugou.com/v2/qrcode?appid=1001&type=1&plat=4&qrcode_txt=https://h5.kugou.com/apps/loginQRCode/html/index.html%3Fappid%3D3116&srcappid=2919"
    );
    let resp = client
        .get(&url)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("获取二维码key失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取二维码key响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("二维码key JSON解析失败: {e}"))?;

    root
        .get("data")
        .and_then(|v| v.get("qrcode"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("响应中未找到qrcode: {text}"))
}

/// 轮询扫码状态
/// status: 0=过期 1=等扫码 2=已扫待确认 4=成功
pub async fn qr_check(client: &Client, key: &str) -> Result<Value, String> {
    let url = format!(
        "https://login-user.kugou.com/v2/get_userinfo_qrcode?plat=4&appid=3116&srcappid=2919&qrcode={key}"
    );
    let resp = client
        .get(&url)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("查询扫码状态失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取扫码状态响应失败: {e}"))?;

    serde_json::from_str(&text)
        .map_err(|e| format!("扫码状态JSON解析失败: {e}"))
}

// ── 密码登录辅助函数 ──

use hex;
use base64::Engine;
use rand::Rng;
use aes::Aes128;
use cbc::Encryptor;
use cbc::cipher::{KeyIvInit, BlockEncryptMut};
use cbc::cipher::block_padding::Pkcs7;
use rsa::{RsaPublicKey, pkcs1v15, pkcs8::DecodePublicKey};

type Aes128CbcEnc = Encryptor<Aes128>;

fn random_aes_key() -> String {
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| rng.gen_range(b'a'..=b'z') as char)
        .collect()
}

/// AES-128-CBC 加密 + PKCS7 padding → base64(iv || ciphertext)
/// key_str: 6位小写字母，零填充到16字节
fn aes_encrypt(key_str: &str, plaintext: &str) -> String {
    let mut key = [0u8; 16];
    let kb = key_str.as_bytes();
    let len = kb.len().min(16);
    key[..len].copy_from_slice(&kb[..len]);

    let mut iv = [0u8; 16];
    rand::thread_rng().fill(&mut iv);

    let pt = plaintext.as_bytes();
    let buf_len = pt.len() + 16;
    let mut buf = vec![0u8; buf_len];
    buf[..pt.len()].copy_from_slice(pt);

    let cipher = Aes128CbcEnc::new(&key.into(), &iv.into());
    let ct_len = cipher.encrypt_padded_mut::<Pkcs7>(&mut buf, pt.len())
        .map(|ct| ct.len())
        .unwrap_or(0);

    let mut result = iv.to_vec();
    result.extend_from_slice(&buf[..ct_len]);
    base64::engine::general_purpose::STANDARD.encode(&result)
}

/// RSA PKCS1 v1.5 加密 → HEX
fn rsa_encrypt(pub_key: &RsaPublicKey, data: &str) -> Result<String, String> {
    let mut rng = rand::thread_rng();
    let enc = pub_key
        .encrypt(&mut rng, pkcs1v15::Pkcs1v15Encrypt, data.as_bytes())
        .map_err(|e| format!("RSA加密失败: {e}"))?;
    Ok(hex::encode(enc))
}

/// 硬编码 RSA 公钥（PKCS8 PEM）
fn kugou_public_key() -> Result<RsaPublicKey, String> {
    let pem = "-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDIAG7QOELSYoIJvTFJhMpe1s/gbjDJX51HBNnEl5HXqTW6lQ7LC8jr9fWZTwusknp+sVGzwd40MwP6U5yDE27M/X1+UR4tvOGOqp94TJtQ1EPnWGWXngpeIW5GxoQGao1rmYWAu6oi1z9XkChrsUdC6DJE5E221wf/4WLFxwAtRQIDAQAB
-----END PUBLIC KEY-----";
    RsaPublicKey::from_public_key_pem(pem).map_err(|e| format!("RSA公钥解析失败: {e}"))
}

/// 设备注册 — 返回 dfid
pub async fn register_device(client: &Client) -> Result<String, String> {
    let uid = generate_uuid();
    let device_params = serde_json::json!({
        "brand": "Redmi",
        "device": "marble",
        "manufacturer": "Xiaomi",
        "availableRamSize": 4983533568u64,
        "availableRomSize": 48114719u64,
        "availableSDSize": 48114717u64,
        "batteryLevel": 100,
        "batteryStatus": 3,
        "basebandVer": "",
        "buildSerial": "unknown",
        "imei": uid,
        "imsi": "",
        "uuid": uid,
        "accelerometer": false, "accelerometerValue": "",
        "gravity": false, "gravityValue": "",
        "gyroscope": false, "gyroscopeValue": "",
        "light": false, "lightValue": "",
        "magnetic": false, "magneticValue": "",
        "orientation": false, "orientationValue": "",
        "pressure": false, "pressureValue": "",
        "step_counter": false, "step_counterValue": "",
        "temperature": false, "temperatureValue": ""
    });

    let device_json = device_params.to_string();

    // 1. 生成随机 AES key + 加密设备参数
    let aes_key_str = random_aes_key();
    let aes_plain_base64 = aes_encrypt(&aes_key_str, &device_json);

    // 2. RSA 加密 aes key info
    let pub_key = kugou_public_key()?;
    let rsa_data = serde_json::json!({"aes": aes_key_str, "uid": 0, "token": ""}).to_string();
    let rsa_hex = rsa_encrypt(&pub_key, &rsa_data)?;

    // 3. 发送请求
    let url = "https://userservice.kugou.com/risk/v2/r_register_dev";
    let resp = client
        .post(url)
        .query(&[("part", "1"), ("platid", "1"), ("p", &rsa_hex)])
        .header("User-Agent", UA)
        .header("Content-Type", "text/plain")
        .body(aes_plain_base64.clone())
        .send()
        .await
        .map_err(|e| format!("设备注册请求失败: {e}"))?;

    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("设备注册响应失败: {e}"))?;

    // 4. AES 解密响应
    let resp_base64 = resp_text.trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(resp_base64)
        .map_err(|e| format!("设备注册响应base64解码失败: {e}"))?;

    // 对 decoded 做 AES 解密（用原 key）
    let decrypted = aes_decrypt(&aes_key_str, &decoded)?;

    let root: Value = serde_json::from_str(&decrypted)
        .map_err(|e| format!("设备注册响应JSON解析失败: {e}, text={decrypted:.300}"))?;

    // 提取 dfid
    root
        .get("dfid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("设备注册响应中未找到dfid: {decrypted}"))
}

/// AES-128-CBC 解密（响应 = iv[16] + ciphertext）
/// key_str: 与加密相同的6位小写字母
fn aes_decrypt(key_str: &str, data: &[u8]) -> Result<String, String> {
    if data.len() < 16 {
        return Err("数据太短".into());
    }
    let mut key = [0u8; 16];
    let kb = key_str.as_bytes();
    let len = kb.len().min(16);
    key[..len].copy_from_slice(&kb[..len]);

    use cbc::Decryptor;
    use cbc::cipher::BlockDecryptMut;
    type Aes128CbcDec = Decryptor<Aes128>;
    let iv = &data[..16];
    let ct = &data[16..];
    let mut buf = ct.to_vec();
    let cipher = Aes128CbcDec::new(&key.into(), iv.into());
    let plaintext = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| format!("AES解密失败: {e}"))?;
    String::from_utf8(plaintext.to_vec())
        .map_err(|e| format!("AES解密结果UTF8解析失败: {e}"))
}

/// 密码登录 — 返回 {token, userid, vip_token, vip_type}
pub async fn login_by_pwd(
    client: &Client,
    username: &str,
    password: &str,
    _dfid: &str,
) -> Result<Value, String> {
    let clienttime_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // 1. AES 加密密码
    let aes_key_str = random_aes_key();

    let pwd_payload = serde_json::json!({
        "pwd": password,
        "code": "",
        "clienttime_ms": clienttime_ms
    }).to_string();

    let params_base64 = aes_encrypt(&aes_key_str, &pwd_payload);

    // 2. RSA 加密 key info
    let pub_key = kugou_public_key()?;
    let rsa_data = serde_json::json!({
        "clienttime_ms": clienttime_ms,
        "key": aes_key_str
    }).to_string();
    let pk = rsa_encrypt(&pub_key, &rsa_data)?.to_uppercase();

    // 3. 构造请求 body
    let body = serde_json::json!({
        "plat": 1,
        "support_multi": 1,
        "clienttime_ms": clienttime_ms,
        "t1": "562a6f12a6e803453647d16a08f5f0c2ff7eee692cba2ab74cc4c8ab47fc467561a7c6b586ce7dc46a63613b246737c03a1dc8f8d162d8ce1d2c71893d19f1d4b797685a4c6d3d81341cbde65e488c4829a9b4d42ef2df470eb102979fa5adcdd9b4eecfea8b909ff7599abeb49867640f10c3c70fc444effca9d15db44a9a6c907731e2bb0f22cd9b3536380169995693e5f0e2424e3378097d3813186e3fe96bbe7023808a0981b4e2b6135a76faac",
        "t2": "31c4daf4cf480169ccea1cb7d4a209295865a9d2b788510301694db229b87807469ea0d41b4d4b9173c2151da7294aeebfc9738df154bbdf11a4e117bb5dff6a3af8ce5ce333e681c1f29a44038f27567d58992eb81283e080778ac77db1400fdf49b7cf7e26be2e5af4da7830cc3be4",
        "t3": "MCwwLDAsMCwwLDAsMCwwLDA=",
        "username": username,
        "params": params_base64,
        "pk": pk
    });

    let url = "https://gateway.kugou.com/v9/login_by_pwd";
    let resp = client
        .post(url)
        .header("x-router", "login.user.kugou.com")
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("密码登录请求失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("密码登录响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("密码登录JSON解析失败: {e}, body={:.300}", text))?;

    let status = root
        .get("status")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if status != 1 {
        let errmsg = root
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        return Err(format!("登录失败 status={}: {}", status, errmsg));
    }

    // 解析 data.secu_params（AES 加密）
    let secu_params_base64 = root
        .get("data")
        .and_then(|v| v.get("secu_params"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("未找到secu_params: {text}"))?;

    let secu_decoded = base64::engine::general_purpose::STANDARD
        .decode(secu_params_base64)
        .map_err(|e| format!("secu_params base64解码失败: {e}"))?;

    let secu_json = aes_decrypt(&aes_key_str, &secu_decoded)?;

    let secu: Value = serde_json::from_str(&secu_json)
        .map_err(|e| format!("secu_params JSON解析失败: {e}, text={secu_json:.300}"))?;

    let token = secu
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let userid = secu
        .get("userid")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let vip_token = secu
        .get("vip_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let vip_type = secu
        .get("vip_type")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if token.is_empty() {
        return Err("登录成功但未获取到token".to_string());
    }

    log_info!("kugou", "登录成功 userid={userid} vip_type={vip_type}");

    Ok(serde_json::json!({
        "token": token,
        "userid": userid,
        "vip_token": vip_token,
        "vip_type": vip_type
    }))
}

// ── 插件封装 ──

use std::sync::{Arc, Mutex};
use crate::config::KugouAuth;

pub struct KugouPlugin {
    auth: Arc<Mutex<KugouAuth>>,
}

impl KugouPlugin {
    pub fn new(auth: Arc<Mutex<KugouAuth>>) -> Self { Self { auth } }
}

#[async_trait]
impl PlatformPlugin for KugouPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: "kugou".to_string(),
            name: "酷狗音乐".to_string(),
            description: "概念版音源，华语曲库全".to_string(),
            version: "1.0.0".to_string(),
            capability: PluginCapability {
                search: true,
                play_url: true,
                lyrics: false,
                browser_playable: true,
                needs_auth: false,
                stability: 70,
            },
        }
    }

    async fn search(
        &self,
        client: &Client,
        keyword: &str,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<Song>, String> {
        let a = self.auth.lock().unwrap().clone();
        search_impl(client, keyword, page, page_size, &a).await
    }

    async fn play_url(
        &self,
        client: &Client,
        song_id: &str,
        quality: &str,
    ) -> Result<Value, String> {
        let a = self.auth.lock().unwrap().clone();
        play_url_impl(client, song_id, quality, &a).await
    }
}

// ── P8-2: 酷狗收藏同步 API ──

/// 获取用户歌单列表 → [{id, name, img, count, ...}]
pub async fn fetch_user_playlists(
    client: &Client,
    token: &str,
    userid: u64,
) -> Result<Vec<Value>, String> {
    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();

    let body = serde_json::json!({
        "userid": userid,
        "token": token,
        "total_ver": 979,
        "type": 2,
        "page": 1,
        "pagesize": 30
    });
    let body_str = body.to_string();

    let userid_str = userid.to_string();
    let clienttime_str = clienttime.to_string();
    let params: Vec<(&str, &str)> = vec![
        ("plat", "1"),
        ("userid", &userid_str),
        ("token", token),
    ];

    let signature = compute_signature(&params, &body_str);

    let mut all_params = params.clone();
    all_params.extend_from_slice(&[
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", DFID),
        ("mid", &mid),
        ("uuid", &uuid),
        ("signature", &signature),
    ]);

    let url = format!("{GATEWAY}/v7/get_all_list");
    let resp = client
        .post(&url)
        .query(&all_params)
        .header("x-router", "cloudlist.service.kugou.com")
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("获取歌单列表失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("歌单列表响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("歌单列表JSON解析失败: {e}, body={:.300}", text))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let errmsg = root.get("error").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("歌单列表API错误 status={}: {}", status, errmsg));
    }

    let playlists = root
        .get("data")
        .and_then(|v| v.get("info"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    log_info!("kugou", "获取到 {} 个歌单", playlists.len());
    Ok(playlists)
}

/// 获取歌单内歌曲 → [{FileName, FileHash, AlbumID, SingerName, Duration, ...}]
pub async fn fetch_playlist_tracks(
    client: &Client,
    playlist_id: &str,
    page: usize,
    page_size: usize,
) -> Result<Vec<Value>, String> {
    let clienttime = current_timestamp_secs();
    let mid = generate_mid();
    let uuid = generate_uuid();

    let begin_idx = (page.saturating_sub(1)) * page_size;
    let begin_idx_str = begin_idx.to_string();
    let pagesize_str = page_size.to_string();
    let clienttime_str = clienttime.to_string();

    let params: Vec<(&str, &str)> = vec![
        ("area_code", "1"),
        ("begin_idx", &begin_idx_str),
        ("plat", "1"),
        ("type", "1"),
        ("mode", "1"),
        ("personal_switch", "1"),
        ("extend_fields", "abtags,hot_cmt,popularization"),
        ("pagesize", &pagesize_str),
        ("global_collection_id", playlist_id),
    ];

    let signature = compute_signature(&params, "");

    let mut all_params = params.clone();
    all_params.extend_from_slice(&[
        ("appid", APPID),
        ("clientver", CLIENTVER),
        ("clienttime", &clienttime_str),
        ("dfid", DFID),
        ("mid", &mid),
        ("uuid", &uuid),
        ("signature", &signature),
    ]);

    let url = format!("{GATEWAY}/pubsongs/v2/get_other_list_file_nofilt");
    let resp = client
        .get(&url)
        .query(&all_params)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("获取歌单歌曲失败: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("歌单歌曲响应失败: {e}"))?;

    let root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("歌单歌曲JSON解析失败: {e}, body={:.300}", text))?;

    let status = root.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 1 {
        let errmsg = root.get("error").and_then(|v| v.as_str()).unwrap_or("未知错误");
        return Err(format!("歌单歌曲API错误 status={}: {}", status, errmsg));
    }

    let tracks = root
        .get("data")
        .and_then(|v| v.get("info"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(tracks)
}
