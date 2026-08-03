// ════════════════════════════════════════════════
// 模块: kugou_sidecar
// ────────────────────────────────────────────
// 管理 KuGouMusicApi 子进程 (localhost:6521)
//
// 架构: Tauri Rust → HTTP → KuGouMusicApi (Node)
// 酷狗所有 API 统一走 sidecar，不再自己签名。
//
// 启动: node app.js --platform=lite --port=6521
//
// 自愈机制: 每次 HTTP 调用前探活,
//   死 → 自动重启 → 重试一次（全程 ≤ 3s）
// ════════════════════════════════════════════════

use crate::debug_log;
use serde_json::Value;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::time::sleep;

const SIDECAR_PORT: u16 = 6521;
const SIDECAR_BASE: &str = "http://127.0.0.1:6521";

/// 进程锁
static SIDECAR_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
/// 启动路径（spawn 时设，restart 时复用）
static SIDECAR_APP_BASE: OnceLock<String> = OnceLock::new();

// ════════════════════════════════════════════════
// 生命周期
// ════════════════════════════════════════════════

/// 启动 KuGouMusicApi sidecar
pub fn spawn_sidecar(app_base: &Path) -> Result<(), String> {
    let api_dir = app_base.join("kugou-api");
    let app_js = api_dir.join("app.js");

    if !app_js.exists() {
        return Err(format!("Sidecar not found: {}", app_js.display()));
    }

    // 保存路径供 restart 复用
    let _ = SIDECAR_APP_BASE.set(app_base.to_string_lossy().to_string());

    debug_log::info("sidecar", &format!("启动: node app.js --platform=lite --port={}", SIDECAR_PORT));

    let child = Command::new("node")
        .arg("app.js")
        .arg("--platform=lite")
        .arg(format!("--port={}", SIDECAR_PORT))
        .current_dir(&api_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Sidecar spawn failed: {e}"))?;

    let mut guard = SIDECAR_PROCESS.lock().unwrap();
    if let Some(ref mut old) = *guard {
        let _ = old.kill();
    }
    *guard = Some(child);

    Ok(())
}

/// 等待 sidecar 就绪（轮询 / 最多 15s）
pub async fn wait_ready() -> Result<(), String> {
    let client = reqwest::Client::new();
    for i in 0..30 {
        match client.get(SIDECAR_BASE).timeout(Duration::from_secs(2)).send().await {
            Ok(resp) if resp.status().is_success() => {
                debug_log::info("sidecar", "✅ 就绪");
                return Ok(());
            }
            _ => {
                if i == 0 {
                    debug_log::info("sidecar", "等待就绪...");
                }
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
    Err("Sidecar 启动超时".into())
}

/// 清理: 杀死 sidecar 进程
pub fn kill_sidecar() {
    if let Ok(mut guard) = SIDECAR_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            debug_log::info("sidecar", "🔌 手动停止");
            let _ = child.kill();
        }
        *guard = None;
    }
}

/// 手动重启 sidecar（供前端按钮调用）
pub async fn restart_sidecar() -> Result<(), String> {
    debug_log::info("sidecar", "🔄 手动重启...");
    kill_sidecar();

    let base = SIDECAR_APP_BASE.get()
        .ok_or("Sidecar 路径未初始化")?;
    let base_path = Path::new(base);
    spawn_sidecar(base_path)?;
    wait_ready().await
}

// ════════════════════════════════════════════════
// 健康检查
// ════════════════════════════════════════════════

/// 快速探活（2s 超时）
pub async fn health_check() -> bool {
    let client = reqwest::Client::new();
    match client.get(SIDECAR_BASE).timeout(Duration::from_secs(2)).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// ════════════════════════════════════════════════
// 自愈 HTTP 封装（所有请求统一入口）
// ════════════════════════════════════════════════

/// 带自愈的 HTTP GET — 死 → 重启 → 重试一次
async fn sidecar_get(url: &str) -> Result<String, String> {
    // 先探活
    if !health_check().await {
        debug_log::warn("sidecar", "❌ 无响应，尝试重启...");
        kill_sidecar();
        if let Some(base) = SIDECAR_APP_BASE.get() {
            let base_path = Path::new(base);
            spawn_sidecar(base_path)?;
            wait_ready().await?;
        } else {
            return Err("Sidecar 路径未初始化".into());
        }
    }

    let client = reqwest::Client::new();
    match client.get(url).timeout(Duration::from_secs(10)).send().await {
        Ok(resp) => resp.text().await.map_err(|e| format!("body read: {e}")),
        Err(e) => {
            // 第一次失败 — 再重启重试一次
            debug_log::warn("sidecar", &format!("⚡ 请求失败: {e}，再次重启..."));
            kill_sidecar();
            if let Some(base) = SIDECAR_APP_BASE.get() {
                let bp = Path::new(base);
                spawn_sidecar(bp)?;
                wait_ready().await?;
            }
            // 重试
            client.get(url).timeout(Duration::from_secs(10)).send().await
                .map_err(|e2| format!("重试失败: {e2}"))?
                .text().await
                .map_err(|e2| format!("重试 body: {e2}"))
        }
    }
}

// ════════════════════════════════════════════════
// Cookie 拼接
// ════════════════════════════════════════════════

fn with_cookie(base_url: &str, token: &str, userid: &str, dfid: &str) -> String {
    if token.is_empty() {
        base_url.to_string()
    } else {
        let sep = if base_url.contains('?') { "&" } else { "?" };
        if dfid.is_empty() {
            format!("{base_url}{sep}cookie=token%3D{token}%3Buserid%3D{userid}")
        } else {
            format!("{base_url}{sep}cookie=token%3D{token}%3Buserid%3D{userid}%3Bdfid%3D{dfid}")
        }
    }
}

// ════════════════════════════════════════════════
// API 方法
// ════════════════════════════════════════════════

/// 搜索 (keywords 复数！)
pub async fn search(keyword: &str, page: u32, token: &str, userid: &str) -> Result<Value, String> {
    let url = with_cookie(
        &format!("{SIDECAR_BASE}/search?keywords={kw}&page={page}&pagesize=30&platform=AndroidFilter",
            kw = url_escape(keyword)),
        token, userid, "",
    );
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("search json: {e} | {:.200}", body))
}

pub async fn user_playlists(token: &str, userid: &str, dfid: &str) -> Result<Value, String> {
    let url = with_cookie(&format!("{}/user/playlist", SIDECAR_BASE), token, userid, dfid);
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("playlist json: {e} | {:.200}", body))
}

pub async fn playlist_songs(list_id: u64, token: &str, userid: &str, dfid: &str) -> Result<Value, String> {
    let url = with_cookie(
        &format!("{SIDECAR_BASE}/playlist/track/all/new?listid={list_id}&pagesize=100"),
        token, userid, dfid,
    );
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("playlist songs json: {e} | {:.200}", body))
}

pub async fn song_url(hash: &str, _album_audio_id: u64, token: &str, userid: &str, dfid: &str) -> Result<String, String> {
    // ⚠️ 不传 album_audio_id——部分歌传了反而触发 status=2 不返 URL
    let v = try_song_url(hash, 0, token, userid, dfid).await?;
    extract_url(&v)
}

async fn try_song_url(hash: &str, album_audio_id: u64, token: &str, userid: &str, dfid: &str) -> Result<Value, String> {
    let url = if dfid.is_empty() {
        format!("{SIDECAR_BASE}/song/url?cookie=token%3D{token}%3Buserid%3D{userid}&hash={hash}&album_audio_id={album_audio_id}&quality=128")
    } else {
        format!("{SIDECAR_BASE}/song/url?cookie=token%3D{token}%3Buserid%3D{userid}%3Bdfid%3D{dfid}&hash={hash}&album_audio_id={album_audio_id}&quality=128")
    };
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("song_url json: {e} | {:.300}", body))
}

fn extract_url(v: &Value) -> Result<String, String> {
    v["url"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|u| u.as_str())
        .or_else(|| v["url"].as_str())
        .or_else(|| v["data"]["url"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("no url in: {:.300}", v))
}

pub async fn user_info(token: &str, userid: &str) -> Result<Value, String> {
    let url = format!("{SIDECAR_BASE}/user/detail?cookie=token%3D{token}%3Buserid%3D{userid}");
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("user_info json: {e} | {:.200}", body))
}

pub async fn favorite_songs(token: &str, userid: &str) -> Result<Value, String> {
    let url = format!("{SIDECAR_BASE}/youth/user/song?cookie=token%3D{token}%3Buserid%3D{userid}");
    let body = sidecar_get(&url).await?;
    serde_json::from_str(&body).map_err(|e| format!("fav songs json: {e} | {:.200}", body))
}

pub async fn register_device() -> Result<String, String> {
    let url = format!("{SIDECAR_BASE}/register/dev");
    let body = sidecar_get(&url).await?;
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| format!("register_dev json: {e} | {:.200}", body))?;
    let dfid = v["data"]["dfid"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("no dfid in: {:.200}", body))?;
    debug_log::info("sidecar", &format!("设备注册成功: dfid={}", dfid));
    Ok(dfid)
}

pub async fn login_qr_key() -> Result<(String, String), String> {
    let url = format!("{SIDECAR_BASE}/login/qr/key");
    let body = sidecar_get(&url).await?;
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| format!("qr_key json: {e} | {:.200}", body))?;
    let key = v["data"]["qrcode"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("no qrcode in: {:.200}", body))?;
    let img = v["data"]["qrcode_img"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();
    Ok((key, img))
}

pub async fn login_qr_check(key: &str) -> Result<(i64, Option<(String, u64, String)>), String> {
    let url = format!("{SIDECAR_BASE}/login/qr/check?key={key}&_t={}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let body = sidecar_get(&url).await?;
    let v: Value = serde_json::from_str(&body)
        .map_err(|e| format!("qr_check json: {e} | {:.200}", body))?;

    let status = v["data"]["status"].as_i64().unwrap_or(0);
    let auth_info = if status == 4 {
        let token = v["data"]["token"].as_str().unwrap_or("").to_string();
        let userid = v["data"]["userid"].as_u64().unwrap_or(0);
        let nickname = v["data"]["nickname"].as_str().unwrap_or("").to_string();
        debug_log::info("sidecar", &format!("QR login ok: userid={} nickname={}", userid, nickname));
        Some((token, userid, nickname))
    } else {
        None
    };
    Ok((status, auth_info))
}

fn url_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}
