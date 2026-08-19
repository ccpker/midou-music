// ═══════════════════════════════════════════════════════════════
// 音楽自由 — 酷狗扫码登录（自建 Rust，直连酷狗，零外部依赖）
// ═══════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use base64::Engine as _;

use crate::db;
use crate::types::{AppState, KugouAuth};

const SIGN_SALT: &str = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
const SRC_APPID: i32 = 2919;
const CLIENT_VER: i32 = 11309;
const DEFAULT_DFID: &str = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";

// ── 签名 ────────────────────────────────────────────

fn md5_hex(s: &str) -> String {
    format!("{:x}", md5::compute(s.as_bytes()))
}

fn sign_web(params: &[(&str, String)]) -> String {
    let mut keys: Vec<&&str> = params.iter().map(|(k, _)| k).collect();
    keys.sort();
    let mut pairs = Vec::with_capacity(params.len());
    for k in keys {
        for (pk, pv) in params {
            if pk == k {
                pairs.push(format!("{k}={pv}"));
                break;
            }
        }
    }
    let joined = pairs.join("");
    let input = format!("{SIGN_SALT}{joined}{SIGN_SALT}");
    md5_hex(&input)
}

fn default_params(appid: i32, token: Option<&str>, userid: u64) -> Vec<(&'static str, String)> {
    let dfid = DEFAULT_DFID;
    let mid = md5_hex(dfid);
    let uuid = md5_hex(&format!("{dfid}{mid}"));
    let clienttime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string();

    let mut v = vec![
        ("dfid", dfid.to_string()),
        ("mid", mid),
        ("uuid", uuid),
        ("appid", appid.to_string()),
        ("clientver", CLIENT_VER.to_string()),
        ("userid", userid.to_string()),
        ("clienttime", clienttime),
    ];
    if let Some(t) = token {
        if !t.is_empty() { v.push(("token", t.to_string())); }
    }
    v
}

async fn kugou_get(
    client: &reqwest::Client, base_url: &str, path: &str,
    extra_params: &[(&str, String)], appid: i32, token: Option<&str>, userid: u64,
) -> Result<serde_json::Value, String> {
    let mut params = default_params(appid, token, userid);
    for (k, v) in extra_params { params.push((k, v.to_string())); }
    let signature = sign_web(&params);

    let url = format!("{base_url}{path}");
    log::info!("[kugou] GET {url}");

    let query: Vec<_> = params.iter().map(|(k, v)| (*k, v.as_str()))
        .chain(std::iter::once(("signature", signature.as_str()))).collect();

    let ctime = params.iter().find(|(k,_)| *k == "clienttime").map(|(_,v)| v.as_str()).unwrap_or("");
    let ctime_hdr = String::from(ctime);
    let mid_val = params.iter().find(|(k,_)| *k == "mid").map(|(_,v)| v.as_str()).unwrap_or("");

    let resp = client.get(&url)
        .query(&query)
        .header("dfid", DEFAULT_DFID)
        .header("clienttime", ctime_hdr)
        .header("mid", mid_val)
        .timeout(Duration::from_secs(10))
        .send().await.map_err(|e| format!("请求失败: {e}"))?;

    let sc = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| format!("读取: {e}"))?;
    log::info!("[kugou] HTTP {sc} body(200): {:.200}", text);

    if sc == 404 { return Err(format!("酷狗 404: {path}")); }
    if !(200..300).contains(&sc) { return Err(format!("HTTP {sc}")); }

    serde_json::from_str(&text).map_err(|e| format!("JSON: {e}"))
}

// ── 生成 PNG 二维码 → base64 data URL ──────────────

fn qrcode_to_png_data_url(text: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|e| format!("qr: {e}"))?;
    let width = code.width() as u32;
    let q_size = 4;
    let img_w = width * q_size + 16;

    let mut buf: Vec<u8> = vec![0xFF; (img_w * img_w) as usize];

    for y in 0..code.width() {
        for x in 0..code.width() {
            if code[(x, y)] != qrcode::Color::Light {
                let px = x as u32 * q_size + 8;
                let py = y as u32 * q_size + 8;
                for dy in 0..q_size {
                    for dx in 0..q_size {
                        let idx = ((py + dy) * img_w + (px + dx)) as usize;
                        if idx < buf.len() { buf[idx] = 0; }
                    }
                }
            }
        }
    }

    let img = image::GrayImage::from_raw(img_w, img_w, buf).ok_or("图片构造失败")?;
    let mut png_buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_buf), image::ImageFormat::Png)
        .map_err(|e| format!("PNG编码: {e}"))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_buf);
    Ok(format!("data:image/png;base64,{b64}"))
}

// ── 1️⃣ 获取二维码 ──────────────────────────────────

#[tauri::command]
pub async fn kugou_qr_key(state: tauri::State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    log::info!("[kugou] ═══ kugou_qr_key (Rust直连) ═══");

    let extra = vec![
        ("type", "1".to_string()),
        ("plat", "4".to_string()),
        ("srcappid", SRC_APPID.to_string()),
        ("qrcode_txt", "https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=3116&".to_string()),
    ];

    let body = kugou_get(&state.client, "https://login-user.kugou.com", "/v2/qrcode", &extra, 1001, None, 0).await?;

    let ec = body.get("error_code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if ec != 0 {
        let msg = body.get("error_msg").and_then(|v| v.as_str()).unwrap_or("?");
        log::warn!("[kugou] ❌ ec={ec} msg={msg}");
        return Err(format!("酷狗: {msg} (code={ec})"));
    }

    let data = body.get("data").ok_or("无data")?;
    let key = data.get("qrcode").and_then(|v| v.as_str()).ok_or("无qrcode")?;
    let qr_url = format!("https://h5.kugou.com/apps/loginQRCode/html/index.html?qrcode={key}");
    let png = qrcode_to_png_data_url(&qr_url)?;

    log::info!("[kugou] ✅ key={:.8} png_len={}", &key[..8.min(key.len())], png.len());
    Ok(serde_json::json!({ "qrcode_key": key, "qrcode_img": png }))
}

// ── 2️⃣ 轮询 ────────────────────────────────────────

#[tauri::command]
pub async fn kugou_qr_check(state: tauri::State<'_, Arc<AppState>>, qrcode_key: String) -> Result<serde_json::Value, String> {
    log::info!("[kugou] check {:.8}", &qrcode_key[..8.min(qrcode_key.len())]);

    let extra = vec![
        ("plat", "4".to_string()),
        ("srcappid", SRC_APPID.to_string()),
        ("qrcode", qrcode_key),
    ];

    let body = kugou_get(&state.client, "https://login-user.kugou.com", "/v2/get_userinfo_qrcode", &extra, 3116, None, 0).await?;
    let data = body.get("data");
    let status = data.and_then(|v| v.get("status")).and_then(|v| v.as_i64()).unwrap_or(-1);

    Ok(serde_json::json!({
        "status": status,
        "status_text": match status { 0=>"已过期", 1=>"等待扫码", 2=>"已扫码待确认", 4=>"登录成功", _=>"未知" },
        "token": data.and_then(|v| v.get("token")).and_then(|v| v.as_str()).filter(|s| !s.is_empty()),
        "userid": data.and_then(|v| v.get("userid")).map(|v| match v { serde_json::Value::Number(n) => n.to_string(), _ => v.as_str().unwrap_or("").to_string() }).filter(|s| !s.is_empty()),
        "nickname": data.and_then(|v| v.get("nickname")).and_then(|v| v.as_str()).filter(|s| !s.is_empty()),
    }))
}

// ── 3️⃣ 保存 ────────────────────────────────────────

#[tauri::command]
pub async fn kugou_save_qr_token(
    app_handle: tauri::AppHandle, state: tauri::State<'_, Arc<AppState>>,
    token: String, userid: String, nickname: String,
) -> Result<serde_json::Value, String> {
    let uid: u64 = userid.parse().unwrap_or(0);
    log::info!("[kugou] save uid={uid} nick={nickname}");

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let auth = KugouAuth { logged_in: true, dfid: DEFAULT_DFID.to_string(), token, userid: uid, vip_token: String::new(), vip_type: 0, token_expires: now + 86400 };
    {
        let db = state.db.lock().map_err(|e| format!("DB: {e}"))?;
        db::save_kugou_auth(&db, &auth)?;
    }
    {
        let mut c = state.kugou_auth.lock().map_err(|e| format!("Auth: {e}"))?;
        *c = auth;
    }
    let _ = app_handle.emit("kugou_auth_updated", serde_json::json!({ "logged_in": true, "userid": userid, "nickname": nickname }));
    Ok(serde_json::json!({ "saved": true, "userid": userid, "nickname": nickname }))
}

// ── 4️⃣ 状态 ────────────────────────────────────────

#[tauri::command]
pub async fn kugou_auth_status(state: tauri::State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let a = state.kugou_auth.lock().map_err(|e| format!("锁: {e}"))?;
    Ok(serde_json::json!({ "logged_in": a.logged_in, "userid": a.userid, "nickname": "" }))
}

// ── 5️⃣ 登出 ────────────────────────────────────────

#[tauri::command]
pub async fn kugou_logout(app_handle: tauri::AppHandle, state: tauri::State<'_, Arc<AppState>>,) -> Result<(), String> {
    log::info!("[kugou] 登出");
    { let db = state.db.lock().map_err(|e| format!("DB: {e}"))?; db::delete_kugou_auth(&db)?; }
    { let mut c = state.kugou_auth.lock().map_err(|e| format!("Auth: {e}"))?; *c = KugouAuth::default_fallback(); }
    let _ = app_handle.emit("kugou_auth_updated", serde_json::json!({ "logged_in": false }));
    Ok(())
}
