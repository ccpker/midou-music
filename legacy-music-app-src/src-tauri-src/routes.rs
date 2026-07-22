// ════════════════════════════════════════════════
// 音楽自由 — Warp HTTP 路由定义
// ────────────────────────────────────────────
// 功能:   定义所有HTTP路由和handler
// 输入:   port + 共享状态(config/index/app_handle/pinned + plugin_registry)
// 输出:   无(启动HTTP服务,阻塞)
// 依赖:   所有search模块, lyrics, download, bilibili, platform
// 边界:   warp serve 阻塞调用,在独立线程运行
// 备注:   调度中心,所有跨模块调用都必须经过此文件
//         v0.5: 新增 plugin_registry, 音源路由逐步迁移至插件中心
// ════════════════════════════════════════════════

use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use reqwest::Client;
use serde::Serialize;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use serde_json::Value;
use tauri::Manager;
use warp::Filter;
use warp::reply::{Reply, Response};

use crate::log_info;

use crate::bilibili::{bili_fetch_buvid, bili_get_audio_url, bili_get_video_url, bili_search};
use crate::community;
use crate::config;
use crate::config::KugouAuth;
use crate::download;
use crate::library::{self, LibrariesIndex, SongImport};
use crate::lyrics;
use crate::models::{AppConfig, LocalIndexData, LocalSong};
use crate::platform::PlatformRegistry;
use crate::search::{kuwo, migu, pure, qq};
use crate::platform::kugou;
use crate::utils::urlencoding;
use crate::utils::sanitize_filename;

// ════════════════════════════════════════════════
// Warp 路由 Filter Helper
// ════════════════════════════════════════════════

fn with_client(client: Arc<Client>) -> impl Filter<Extract = (Arc<Client>,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}
fn with_config(cfg: Arc<Mutex<AppConfig>>) -> impl Filter<Extract = (Arc<Mutex<AppConfig>>,), Error = Infallible> + Clone {
    warp::any().map(move || cfg.clone())
}
fn with_index(idx: Arc<Mutex<LocalIndexData>>) -> impl Filter<Extract = (Arc<Mutex<LocalIndexData>>,), Error = Infallible> + Clone {
    warp::any().map(move || idx.clone())
}
fn with_app_handle(h: Arc<Mutex<Option<tauri::AppHandle>>>) -> impl Filter<Extract = (Arc<Mutex<Option<tauri::AppHandle>>>,), Error = Infallible> + Clone {
    warp::any().map(move || h.clone())
}
fn with_pinned(p: Arc<Mutex<bool>>) -> impl Filter<Extract = (Arc<Mutex<bool>>,), Error = Infallible> + Clone {
    warp::any().map(move || p.clone())
}
fn with_now_playing(np: Arc<Mutex<Option<NowPlaying>>>) -> impl Filter<Extract = (Arc<Mutex<Option<NowPlaying>>>,), Error = Infallible> + Clone {
    warp::any().map(move || np.clone())
}
fn with_kugou_auth(auth: Arc<Mutex<KugouAuth>>) -> impl Filter<Extract = (Arc<Mutex<KugouAuth>>,), Error = Infallible> + Clone {
    warp::any().map(move || auth.clone())
}
fn with_lib_index(idx: Arc<Mutex<LibrariesIndex>>) -> impl Filter<Extract = (Arc<Mutex<LibrariesIndex>>,), Error = Infallible> + Clone {
    warp::any().map(move || idx.clone())
}

// ════════════════════════════════════════════════
// HTTP 响应辅助函数
// ════════════════════════════════════════════════

fn ok_json(data: &serde_json::Value) -> Response {
    warp::reply::with_header(
        warp::reply::with_header(
            data.to_string(),
            "content-type",
            "application/json; charset=utf-8",
        ),
        "access-control-allow-origin",
        "*",
    )
    .into_response()
}

fn ok_json_obj<T: Serialize>(data: &T) -> Response {
    ok_json(&serde_json::to_value(data).unwrap_or_default())
}

fn ok_html(html: String) -> Response {
    warp::reply::with_header(html, "content-type", "text/html; charset=utf-8")
        .into_response()
}

fn err(msg: &str) -> Response {
    warp::reply::with_status(
        ok_json(&serde_json::json!({"error": msg})),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
    .into_response()
}

// ════════════════════════════════════════════════
// NowPlaying 共享状态 (warp ↔ 歌词窗口 ↔ 前端)
// ════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct NowPlaying {
    pub music_id: String,
    pub song_id: String,
    pub title: String,
    pub artist: String,
    pub source: String,
    pub source_id: String,
    pub duration: f64,
}

// ════════════════════════════════════════════════
// 全局常量
// ════════════════════════════════════════════════

fn frontend_html() -> &'static str {
    include_str!("index.html")
}

/// 从文件名猜标题/歌手 (e.g. "周杰伦 - 七里香.mp3" → ("七里香", "周杰伦"))
fn parse_filename_artist(raw: &str) -> (String, String) {
    let cleaned = raw.trim();
    for sep in &[" - ", "-", "—", "–"] {
        if let Some(pos) = cleaned.find(sep) {
            let left = cleaned[..pos].trim();
            let right = cleaned[pos + sep.len()..].trim();
            if !right.is_empty() && right.len() < 60 {
                return (right.to_string(), left.to_string());
            }
            return (cleaned.to_string(), String::new());
        }
    }
    (cleaned.to_string(), String::new())
}

// ──────────────────────────────────────────────────────────────────────────────
// Phase 1: 前端静态资源 — 双模支持
// 开发期: Vite dev server :5173 提供前端 (tauri.conf.json devUrl=5173)
// 打包期: warp serve frontend/dist/ (方案D 单端口)
// ──────────────────────────────────────────────────────────────────────────────

/// 返回 frontend/dist/ 的绝对路径（相对于 cwd）
fn frontend_dist_path() -> PathBuf {
    // 绝对路径 — 避免运行时 cwd 不同导致路径找不到
    // exe 路径: .../src-tauri/target/debug/music-app.exe
    // 需要定位到: .../music-app/frontend/dist
    if let Ok(exe) = std::env::current_exe() {
        // 尝试多级 parent 跳: 2~6 层都试试
        let mut p = exe.clone();
        for _ in 0..6 {
            if let Some(parent) = p.parent() {
                p = parent.to_path_buf();
            } else {
                break;
            }
            let candidate = p.join("frontend").join("dist");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    // fallback — 相对路径
    PathBuf::from("frontend").join("dist")
}

/// 读取 dist/index.html（打包期）；不存在则回退到内嵌的 legacy HTML（开发期）
fn dist_index_html() -> Response {
    let path = frontend_dist_path().join("index.html");
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(html) => ok_html(html),
            Err(e) => {
                log_info!("server", "[Phase1] 读取 dist/index.html 失败: {e}, 回退到内嵌HTML");
                ok_html(frontend_html().to_string())
            }
        }
    } else {
        log_info!("server", "[Phase1] frontend/dist/ 不存在，使用内嵌HTML (开发模式)");
        ok_html(frontend_html().to_string())
    }
}

// Legacy embedded files — 保留用于 dev 模式 fallback
fn app_css_html() -> warp::reply::WithHeader<&'static str> {
    warp::reply::with_header(include_str!("app.css"), "content-type", "text/css; charset=utf-8")
}
fn app_js_html() -> warp::reply::WithHeader<&'static str> {
    warp::reply::with_header(include_str!("app.js"), "content-type", "application/javascript; charset=utf-8")
}

pub fn find_free_port() -> u16 {
    use std::net::TcpListener;
    if TcpListener::bind("127.0.0.1:8899").is_ok() {
        return 8899;
    }
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

// ════════════════════════════════════════════════
// 主入口: 绑定Warp路由并启动HTTP服务
// ════════════════════════════════════════════════

/// 启动Warp HTTP服务(阻塞)
///
/// # 参数
/// - `port`: 监听端口
/// - `config`: AppConfig 共享引用
/// - `local_idx`: LocalIndexData 共享引用
/// - `app_handle_arc`: Tauri AppHandle 共享引用(歌词窗口用)
/// - `lyrics_pinned`: 歌词窗口置顶状态
pub async fn warp_main(
    port: u16,
    config: Arc<Mutex<AppConfig>>,
    local_idx: Arc<Mutex<LocalIndexData>>,
    app_handle_arc: Arc<Mutex<Option<tauri::AppHandle>>>,
    lyrics_pinned: Arc<Mutex<bool>>,
    plugin_registry: Arc<Mutex<PlatformRegistry>>,
    kugou_auth: Arc<Mutex<KugouAuth>>,
    lib_index: Arc<Mutex<LibrariesIndex>>,
) {
    let c = Arc::new(
        Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap(),
    );
    let config_thread = config.clone();
    let idx_thread = local_idx.clone();

    // ── B站 buvid 全局缓存(懒初始化) ──
    let bili_buvid: Arc<Mutex<Option<(String, String)>>> = Arc::new(Mutex::new(None));
    let now_playing: Arc<Mutex<Option<NowPlaying>>> = Arc::new(Mutex::new(None));

    // ── 路由定义 ──
    let health = warp::path("health").map(|| "OK");
    // Phase 1: 前端 index — 先尝试 dist/index.html，打包后 vite build 产物存在时使用
    let index = warp::path::end().map(dist_index_html);
    let lyrics_view = warp::path!("lyrics-view")
        .map(|| ok_html(include_str!("lyrics-pop.html").to_string()));
    // Phase 4: lyrics.html route — serve from dist/ in prod, fallback empty in dev
    let lyrics_html = warp::path!("lyrics.html").map(|| {
        let path = frontend_dist_path().join("lyrics.html");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(html) => ok_html(html),
                Err(e) => {
                    log_info!("server", "[lyrics.html] 读取失败: {e}");
                    ok_html("<html><body>歌词窗口加载中...</body></html>".to_string())
                }
            }
        } else {
            // Dev mode: served by Vite, this route is a fallback
            ok_html("<html><body>歌词窗口(开发模式请从 Vite:5173 加载)</body></html>".to_string())
        }
    });

    // Phase 1: /dist/* 静态文件路由 — warp serve frontend/dist/ (打包期)
    // warp::path::tail 捕获 /dist/ 之后的所有路径
    let dist_static = warp::path("dist")
        .and(warp::path::tail())
        .map(|tail: warp::path::Tail| {
            let dist_dir = frontend_dist_path();
            let file_path = dist_dir.join(tail.as_str());
            if file_path.exists() && file_path.is_file() {
                match fs::read(&file_path) {
                    Ok(bytes) => {
                        let mime = mime_guess::from_path(&file_path)
                            .first_or_octet_stream();
                        warp::reply::with_header(
                            bytes,
                            "content-type",
                            format!("{}", mime),
                        )
                        .into_response()
                    }
                    Err(e) => {
                        log_info!("server", "[dist static] 读取失败 {}: {e}", file_path.display());
                        warp::reply::with_status(
                            "Not Found",
                            warp::http::StatusCode::NOT_FOUND,
                        )
                        .into_response()
                    }
                }
            } else {
                warp::reply::with_status(
                    "Not Found",
                    warp::http::StatusCode::NOT_FOUND,
                )
                .into_response()
            }
        });

    // Phase 1: /assets/* 静态文件路由 — Vite build 后的资源
    let assets_static = warp::path("assets")
        .and(warp::path::tail())
        .and_then(|tail: warp::path::Tail| async move {
            let dist_dir = frontend_dist_path();
            let file_path = dist_dir.join("assets").join(tail.as_str());
            if file_path.exists() && file_path.is_file() {
                match fs::read(&file_path) {
                    Ok(bytes) => {
                        let mime = mime_guess::from_path(&file_path)
                            .first_or_octet_stream();
                        let resp = warp::reply::with_header(
                            bytes,
                            "content-type",
                            format!("{}", mime),
                        ).into_response();
                        Ok::<_, warp::Rejection>(resp)
                    }
                    Err(_) => Err(warp::reject::not_found()),
                }
            } else {
                Err(warp::reject::not_found())
            }
        });

    // 顶层静态文件（registerSW.js, sw.js, workbox-*.js, manifest.webmanifest 等）
    let root_static = warp::path::tail()
        .and_then(|tail: warp::path::Tail| async move {
            let dist_dir = frontend_dist_path();
            let file_path = dist_dir.join(tail.as_str());
            if file_path.exists() && file_path.is_file() {
                match fs::read(&file_path) {
                    Ok(bytes) => {
                        let mime = mime_guess::from_path(&file_path)
                            .first_or_octet_stream();
                        let resp = warp::reply::with_header(
                            bytes,
                            "content-type",
                            format!("{}", mime),
                        ).into_response();
                        Ok::<_, warp::Rejection>(resp)
                    }
                    Err(_) => Err(warp::reject::not_found()),
                }
            } else {
                Err(warp::reject::not_found())
            }
        });

    // ── 搜索(mode: normal / migu / pure / bilibili / qq / qq-vipfree) ──
    let search = warp::path!("api" / "search")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then({
            let bb = bili_buvid.clone();
            move |q: HashMap<String, String>, c: Arc<Client>, auth: Arc<Mutex<KugouAuth>>| {
                let bb = bb.clone();
                async move {
                    let kw = q.get("keyword").cloned().unwrap_or_default();
                    let page: usize = q.get("page").and_then(|v| v.parse().ok()).unwrap_or(0);
                    let page_size: usize = q.get("page_size").and_then(|v| v.parse().ok()).unwrap_or(20);
                    let mode = q.get("mode").cloned().unwrap_or_else(|| "normal".to_string());

                    let all_songs = match mode.as_str() {
                        // ── B站搜索 ──
                        "bilibili" | "bili-video" => {
                            let buvid = { bb.lock().unwrap().clone() };
                            let (b3, b4) = if let Some(b) = buvid {
                                b
                            } else {
                                match bili_fetch_buvid(&c).await {
                                    Ok(b) => {
                                        *bb.lock().unwrap() = Some(b.clone());
                                        b
                                    }
                                    Err(e) => {
                                        log_info!("bili", "buvid 获取失败: {e}");
                                        return Ok::<_, Infallible>(err(&e));
                                    }
                                }
                            };
                            match bili_search(&c, &kw, page, &b3, &b4).await {
                                Ok(songs) => {
                                    log_info!("bili", "搜索[{}]返回 {} 首", kw, songs.len());
                                    songs
                                }
                                Err(e) => {
                                    log_info!("bili", "搜索失败: {e}");
                                    vec![]
                                }
                            }
                        }
                        // ── 纯净搜索(咪咕+MB+酷我) ──
                        "pure" => pure::search(&c, &kw, page, page_size)
                            .await
                            .unwrap_or_default(),
                        // ── QQ搜索(含VIP过滤) ──
                        "qq" | "qq-vipfree" => {
                            let songs = qq::search(&c, &kw, page, page_size)
                                .await
                                .unwrap_or_default();
                            if mode == "qq-vipfree" {
                                songs.into_iter()
                                    .filter(|s| s.score != Some(1))
                                    .collect()
                            } else {
                                songs
                            }
                        }
                        // ── 咪咕搜索(咪咕→酷我降级) ──
                        "migu" => {
                            let migu_songs = migu::search(&c, &kw, 0, 5)
                                .await
                                .unwrap_or_default();
                            if migu_songs.is_empty() {
                                log_info!("search", "咪咕无结果");
                                vec![]
                            } else {
                                let top = &migu_songs[0];
                                let precise_kw = format!("{} {}", top.name, top.singer);
                                log_info!("search", "咪咕精确: {} - {}", top.name, top.singer);
                                match kuwo::search(&c, &precise_kw, page, page_size).await {
                                    Ok(mut songs) => {
                                        for s in &mut songs {
                                            s.migu_duration = Some(top.duration);
                                        }
                                        songs
                                    }
                                    Err(e) => {
                                        log_info!("search", "咪咕→酷我失败: {e}");
                                        migu_songs
                                    }
                                }
                            }
                        }
                        // ── 酷狗搜索 ──
                        "kugou" => {
                            let a = auth.lock().unwrap().clone();
                            match kugou::search_impl(&c, &kw, page, page_size, &a).await {
                                Ok(songs) => {
                                    log_info!("search", "酷狗搜索[{}]返回 {} 首", kw, songs.len());
                                    songs
                                }
                                Err(e) => {
                                    log_info!("search", "酷狗搜索失败: {e}");
                                    vec![]
                                }
                            }
                        }
                        // ── 普通搜索(酷我原生) ──
                        _ => kuwo::search(&c, &kw, page, page_size)
                            .await
                            .unwrap_or_default(),
                    };

                    Ok::<_, Infallible>(ok_json_obj(&serde_json::json!({
                        "songs": all_songs,
                        "total": all_songs.len(),
                        "page": page,
                        "mode": mode
                    })))
                }
            }
        });

    // ── 酷我播放(本地优先) ──
    let play = warp::path!("api" / "play" / "kuwo" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and_then(
            |rid: String,
             q: HashMap<String, String>,
             c: Arc<Client>,
             cfg: Arc<Mutex<AppConfig>>,
             idx: Arc<Mutex<LocalIndexData>>| async move {
                let name = q.get("name").cloned().unwrap_or_default();
                let singer = q.get("singer").cloned().unwrap_or_default();

                let local_path = {
                    let cfg = cfg.lock().unwrap();
                    let idx = idx.lock().unwrap();
                    download::find_local_url(&cfg, &idx, &rid, &name, &singer)
                };

                if let Some(ref lp) = local_path {
                    log_info!("api", "本地播放: {lp}");
                    let encoded = lp.replace('\\', "/");
                    return Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "url": format!("/local-file?path={}", urlencoding(&encoded)),
                        "local": true,
                        "path": lp
                    })));
                }

                log_info!("kuwo", "播放 rid={}", rid);
                match kuwo::play_url(&c, &rid, "320kmp3").await {
                    Ok(data) => {
                        let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        log_info!("kuwo", "播放成功 url_len={}", url.len());
                        Ok::<_, Infallible>(ok_json(&serde_json::json!({
                            "url": url,
                            "local": false
                        })))
                    }
                    Err(e) => {
                        log_info!("kuwo", "播放失败: {}", e);
                        Ok(err(&e))
                    }
                }
            },
        );

    // ── 酷狗下载 ──
    let kugou_download = warp::path!("api" / "download" / "kugou" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(
            |song_id: String,
             q: HashMap<String, String>,
             c: Arc<Client>,
             cfg: Arc<Mutex<AppConfig>>,
             idx: Arc<Mutex<LocalIndexData>>,
             auth: Arc<Mutex<KugouAuth>>| async move {
                let name = q.get("name").cloned().unwrap_or_else(|| "未知".into());
                let singer = q.get("singer").cloned().unwrap_or_else(|| "未知".into());
                let quality = q.get("quality").cloned().unwrap_or_else(|| "320".into());
                let category = q.get("category").cloned().unwrap_or_else(|| "默认".into());

                let a = auth.lock().unwrap().clone();
                let download_dir = { cfg.lock().unwrap().download_dir.clone() };

                // 1. 获取下载URL
                let url = match kugou::download_url_impl(&c, &song_id, &quality, &a).await {
                    Ok(v) => v,
                    Err(e) => return Ok::<_, Infallible>(err(&format!("获取下载URL失败: {e}"))),
                };

                // 2. 目标目录
                let target_dir = if category == "默认" {
                    download_dir.clone()
                } else {
                    let cat_dir = sanitize_filename(&category);
                    let p = download_dir.join(&cat_dir);
                    tokio::fs::create_dir_all(&p).await.ok();
                    p
                };

                // 3. 文件名（防重名）
                let safe_name = sanitize_filename(&name);
                let safe_singer = sanitize_filename(&singer);
                let ext = if url.contains("flac") { ".flac" } else { ".mp3" };
                let base = format!("{}-{}", safe_name, safe_singer);
                let mut filename = format!("{}{}", base, ext);
                let mut filepath = target_dir.join(&filename);
                let mut counter = 1;
                while filepath.exists() {
                    filename = format!("{}_{}{}", base, counter, ext);
                    filepath = target_dir.join(&filename);
                    counter += 1;
                }

                // 4. 下载文件
                let resp = match c.get(&url)
                    .header("User-Agent", "Mozilla/5.0")
                    .send().await {
                    Ok(r) => r,
                    Err(e) => return Ok::<_, Infallible>(err(&format!("下载失败: {e}"))),
                };

                let bytes = match resp.bytes().await {
                    Ok(b) => b,
                    Err(e) => return Ok::<_, Infallible>(err(&format!("读取响应: {e}"))),
                };

                if let Err(e) = tokio::fs::write(&filepath, &bytes).await {
                    return Ok::<_, Infallible>(err(&format!("写入文件: {e}")));
                }

                // 5. 构建 LocalSong + 入库
                let local_song = LocalSong {
                    song_id: song_id.clone(),
                    name: name.clone(),
                    singer: singer.clone(),
                    quality: quality.clone(),
                    filename: filename.clone(),
                    path: filepath.to_string_lossy().to_string(),
                    size: bytes.len() as u64,
                    category: category.clone(),
                    downloaded_at: crate::utils::timestamp(),
                };

                {
                    let mut idx_locked = idx.lock().unwrap();
                    idx_locked.add_song(local_song.clone());
                    idx_locked.ensure_category(&category);
                    let cfg_lock = cfg.lock().unwrap();
                    config::save_local_index(&cfg_lock, &idx_locked);
                }

                log_info!("download", "酷狗下载完成: {} - {} ({}KB)", name, singer, bytes.len() / 1024);
                Ok::<_, Infallible>(ok_json(&serde_json::json!({
                    "saved": true,
                    "filename": filename,
                    "path": filepath.to_string_lossy(),
                    "quality": quality,
                    "category": category,
                    "file_size": bytes.len()
                })))
            },
        );

    // ── 酷狗播放(本地优先) ──
    let kugou_play = warp::path!("api" / "play" / "kugou" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(
            |song_id: String,
             q: HashMap<String, String>,
             c: Arc<Client>,
             cfg: Arc<Mutex<AppConfig>>,
             idx: Arc<Mutex<LocalIndexData>>,
             auth: Arc<Mutex<KugouAuth>>| async move {
                let name = q.get("name").cloned().unwrap_or_default();
                let singer = q.get("singer").cloned().unwrap_or_default();

                let local_path = {
                    let cfg = cfg.lock().unwrap();
                    let idx = idx.lock().unwrap();
                    download::find_local_url(&cfg, &idx, &song_id, &name, &singer)
                };

                if let Some(ref lp) = local_path {
                    log_info!("api", "酷狗本地播放: {lp}");
                    let encoded = lp.replace('\\', "/");
                    return Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "url": format!("/local-file?path={}", urlencoding(&encoded)),
                        "local": true,
                        "path": lp
                    })));
                }

                log_info!("kugou", "播放 song_id={}", song_id);
                let a = auth.lock().unwrap().clone();
                match kugou::play_url_impl(&c, &song_id, "320", &a).await {
                    Ok(data) => {
                        let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        log_info!("kugou", "播放成功 url_len={}", url.len());
                        Ok::<_, Infallible>(ok_json(&serde_json::json!({
                            "url": url,
                            "local": false
                        })))
                    }
                    Err(e) => {
                        log_info!("kugou", "播放失败: {}", e);
                        Ok(err(&e))
                    }
                }
            },
        );

    // ── QQ播放(本地优先) ──
    let qq_play = warp::path!("api" / "play" / "qq" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and_then(
            |mid: String,
             q: HashMap<String, String>,
             c: Arc<Client>,
             cfg: Arc<Mutex<AppConfig>>,
             idx: Arc<Mutex<LocalIndexData>>| async move {
                let name = q.get("name").cloned().unwrap_or_default();
                let singer = q.get("singer").cloned().unwrap_or_default();

                let local_path = {
                    let cfg = cfg.lock().unwrap();
                    let idx = idx.lock().unwrap();
                    download::find_local_url(&cfg, &idx, &mid, &name, &singer)
                };

                if let Some(ref lp) = local_path {
                    log_info!("api", "QQ本地播放: {lp}");
                    let encoded = lp.replace('\\', "/");
                    return Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "url": format!("/local-file?path={}", urlencoding(&encoded)),
                        "local": true,
                        "path": lp
                    })));
                }

                log_info!("qq", "QQ播放 mid={}", mid);
                match qq::play_url(&c, &mid).await {
                    Ok(url) => {
                        log_info!("qq", "QQ播放成功 url_len={}", url.len());
                        Ok::<_, Infallible>(ok_json(&serde_json::json!({
                            "url": url,
                            "local": false
                        })))
                    }
                    Err(e) => {
                        log_info!("qq", "QQ播放失败: {}", e);
                        Ok(err(&e))
                    }
                }
            },
        );

    // ── B站播放(代理音频流) ──
    let bili_play = warp::path!("api" / "play" / "bilibili")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and_then({
            let bb = bili_buvid.clone();
            move |q: HashMap<String, String>,
                  c: Arc<Client>,
                  cfg: Arc<Mutex<AppConfig>>,
                  idx: Arc<Mutex<LocalIndexData>>| {
                let bb = bb.clone();
                async move {
                    let song_id = q.get("song_id").cloned().unwrap_or_default();
                    let name = q.get("name").cloned().unwrap_or_default();
                    let singer = q.get("singer").cloned().unwrap_or_default();

                    let parts: Vec<&str> = song_id.splitn(2, '|').collect();
                    let (bvid, cid_str) = if parts.len() == 2 {
                        (parts[0].to_string(), parts[1].to_string())
                    } else {
                        (
                            q.get("bvid").cloned().unwrap_or_default(),
                            q.get("cid").cloned().unwrap_or_default(),
                        )
                    };
                    let cid: u64 = cid_str.parse().unwrap_or(0);
                    if bvid.is_empty() || cid == 0 {
                        return Ok::<_, Infallible>(err("缺少bvid或cid"));
                    }

                    // 查本地
                    let local_path = {
                        let cfg = cfg.lock().unwrap();
                        let idx = idx.lock().unwrap();
                        download::find_local_url(&cfg, &idx, &song_id, &name, &singer)
                    };
                    if let Some(ref lp) = local_path {
                        log_info!("api", "B站本地播放: {lp}");
                        let enc = lp.replace('\\', "/");
                        return Ok::<_, Infallible>(ok_json(&serde_json::json!({
                            "url": format!("/local-file?path={}", urlencoding(&enc)),
                            "local": true,
                            "path": lp
                        })));
                    }

                    // 获取 buvid(缓存)
                    let (b3, b4) = {
                        let cached = bb.lock().unwrap().clone();
                        if let Some(b) = cached {
                            b
                        } else {
                            match bili_fetch_buvid(&c).await {
                                Ok(b) => {
                                    *bb.lock().unwrap() = Some(b.clone());
                                    b
                                }
                                Err(e) => {
                                    log_info!("bili", "buvid获取失败: {e}");
                                    return Ok::<_, Infallible>(err(&e));
                                }
                            }
                        }
                    };

                    let proxy_url = format!(
                        "/api/bili-stream?bvid={}&cid={}&buvid3={}&buvid4={}",
                        urlencoding(&bvid),
                        cid,
                        urlencoding(&b3),
                        urlencoding(&b4)
                    );
                    Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "url": proxy_url,
                        "local": false
                    })))
                }
            }
        });

    // ── B站音频流代理（Range 流式，边推边播，不等待完整下载） ──
    // B站流式代理，支持音频(type=audio)和视频(type=video)两种模式
    // 视频模式URL: /api/bili-stream?bvid=&cid=&buvid3=&buvid4=&type=video
    let bili_stream = warp::path!("api" / "bili-stream")
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::optional::<String>("range"))
        .and(with_client(c.clone()))
        .and_then(|q: HashMap<String, String>, range_header: Option<String>, c: Arc<Client>| async move {
            let bvid = q.get("bvid").cloned().unwrap_or_default();
            let cid: u64 = q.get("cid").and_then(|v| v.parse().ok()).unwrap_or(0);
            let buvid3 = q.get("buvid3").cloned().unwrap_or_default();
            let buvid4 = q.get("buvid4").cloned().unwrap_or_default();
            let stream_type = q.get("type").cloned().unwrap_or_else(|| "audio".into());
            if bvid.is_empty() || cid == 0 || buvid3.is_empty() {
                return Ok::<_, Infallible>(
                    warp::reply::with_status("Invalid params", warp::http::StatusCode::BAD_REQUEST)
                        .into_response(),
                );
            }

            // 根据类型获取对应流URL
            let (stream_url, host, mime) = if stream_type == "video" {
                match bili_get_video_url(&c, &bvid, cid, &buvid3, &buvid4).await {
                    Ok((url, h)) => (url, h, "video/mp4".to_string()),
                    Err(e) => {
                        log_info!("bili", "视频流获取失败: {e}");
                        return Ok::<_, Infallible>(
                            warp::reply::with_status(format!("video url error: {e}"), warp::http::StatusCode::BAD_GATEWAY)
                                .into_response(),
                        );
                    }
                }
            } else {
                match bili_get_audio_url(&c, &bvid, cid, &buvid3, &buvid4).await {
                    Ok((url, h)) => {
                        let mime = if url.contains(".m4s") { "audio/mp4".to_string() } else { "audio/mpeg".to_string() };
                        (url, h, mime)
                    },
                    Err(e) => {
                        log_info!("bili", "音频流获取失败: {e}");
                        return Ok::<_, Infallible>(
                            warp::reply::with_status(format!("audio url error: {e}"), warp::http::StatusCode::BAD_GATEWAY)
                                .into_response(),
                        );
                    }
                }
            };

            let log_prefix = if stream_type == "video" { "B站视频" } else { "B站音频" };

            // 构造向 B站 的请求，透传 Range
            let mut req = reqwest::Request::new(reqwest::Method::GET, stream_url.parse().unwrap());
            req.headers_mut().insert(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36".parse().unwrap());
            req.headers_mut().insert("Referer", format!("https://www.bilibili.com/video/{bvid}").parse().unwrap());
            req.headers_mut().insert("Host", host.parse().unwrap());
            if let Some(rh) = &range_header {
                req.headers_mut().insert("Range", rh.parse().unwrap());
            } else {
                req.headers_mut().insert("Range", "bytes=0-".parse().unwrap());
            }

            match c.execute(req).await {
                Ok(resp) => {
                    let status = resp.status();
                    let headers = resp.headers().clone();

                    let body_bytes = match resp.bytes().await {
                        Ok(b) => b.to_vec(),
                        Err(e) => {
                            log_info!("bili", "读取{log_prefix}流失败: {e}");
                            return Ok::<_, Infallible>(
                                warp::reply::with_status("Stream read error", warp::http::StatusCode::BAD_GATEWAY)
                                    .into_response(),
                            );
                        }
                    };

                    let content_len = body_bytes.len();
                    let out_status = if range_header.is_some() && status.as_u16() == 200 {
                        warp::http::StatusCode::PARTIAL_CONTENT
                    } else {
                        warp::http::StatusCode::from_u16(status.as_u16()).unwrap_or(warp::http::StatusCode::OK)
                    };

                    use warp::http::HeaderValue;
                    use warp::http::response::Builder as RespBuilder;
                    let mime_owned = mime;
                    let cr_val = headers.get("content-range").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                    let mime_hv = HeaderValue::from_str(&mime_owned).unwrap_or_else(|_| HeaderValue::from_static("audio/mpeg"));
                    let len_hv = HeaderValue::from_str(&content_len.to_string()).unwrap_or_else(|_| HeaderValue::from_static("0"));
                    let cr_hv = HeaderValue::from_str(&cr_val).unwrap_or_else(|_| HeaderValue::from_static(""));
                    let resp: Result<warp::http::Response<warp::hyper::Body>, _> = RespBuilder::new()
                        .status(out_status)
                        .header("content-type", mime_hv)
                        .header("content-length", len_hv)
                        .header("accept-ranges", HeaderValue::from_static("bytes"))
                        .header("access-control-allow-origin", HeaderValue::from_static("*"))
                        .header("content-range", cr_hv)
                        .body(warp::hyper::Body::from(body_bytes));
                    match resp {
                        Ok(r) => Ok::<_, Infallible>(r.into_response()),
                        Err(e) => Ok::<_, Infallible>(
                            warp::reply::with_status(format!("Response build error: {e}"), warp::http::StatusCode::INTERNAL_SERVER_ERROR)
                                .into_response(),
                        )
                    }
                }
                Err(e) => {
                    log_info!("bili", "{log_prefix}请求失败: {e}");
                    Ok::<_, Infallible>(
                        warp::reply::with_status(format!("Request error: {e}"), warp::http::StatusCode::BAD_GATEWAY)
                            .into_response(),
                    )
                }
            }
        });

    // ── 本地文件播放 ──
    let local_file = warp::path!("local-file")
        .and(warp::query::<HashMap<String, String>>())
        .and_then(|q: HashMap<String, String>| async move {
            let path_str = q.get("path").cloned().unwrap_or_default();
            let path = PathBuf::from(&path_str);
            if !path.exists() || !path.is_file() {
                return Ok::<_, Infallible>(
                    warp::reply::with_status("Not found", warp::http::StatusCode::NOT_FOUND)
                        .into_response(),
                );
            }
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            match fs::read(&path) {
                Ok(data) => {
                    let len = data.len();
                    let resp = warp::reply::with_header(data, "content-type", mime);
                    let resp = warp::reply::with_header(resp, "content-length", len.to_string());
                    let resp = warp::reply::with_header(
                        resp,
                        "access-control-allow-origin",
                        "*",
                    );
                    let resp = warp::reply::with_header(resp, "accept-ranges", "bytes");
                    Ok::<_, Infallible>(resp.into_response())
                }
                Err(e) => Ok::<_, Infallible>(
                    err(&format!("读取文件: {e}")).into_response(),
                ),
            }
        });

    // ── 歌词(多源) ──
    let lyric = warp::path!("api" / "lyric")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and_then(|q: HashMap<String, String>, c: Arc<Client>| async move {
            let name = q.get("name").cloned().unwrap_or_default();
            let artist = q.get("artist").cloned().unwrap_or_default();
            let source = q.get("source").map(|s| s.as_str());
            let duration_ms: Option<u32> = q.get("duration")
                .and_then(|s| s.parse().ok());
            let file_hash = q.get("hash").map(|s| s.as_str());
            match lyrics::fetch(&c, &name, &artist, source, duration_ms, file_hash).await {
                Ok(data) => Ok::<_, Infallible>(ok_json(&data)),
                Err(e) => Ok(err(&e)),
            }
        });

    // ── 下载(FLAC→320→128 降级链) ──
    let download_route = warp::path!("api" / "download" / "kuwo" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and(with_config(config_thread.clone()))
        .and(with_index(idx_thread.clone()))
        .and_then(
            |rid: String,
             q: HashMap<String, String>,
             c: Arc<Client>,
             cfg: Arc<Mutex<AppConfig>>,
             idx: Arc<Mutex<LocalIndexData>>| async move {
                let name = q.get("name").cloned().unwrap_or_default();
                let singer = q.get("singer").cloned().unwrap_or_default();
                let category = q.get("category").cloned().unwrap_or_else(|| "默认".to_string());
                let download_dir = { cfg.lock().unwrap().download_dir.clone() };
                let temp_cfg = AppConfig {
                    download_dir,
                };

                match download::download_with_fallback(
                    &c, &rid, &name, &singer, &category, &temp_cfg,
                )
                .await
                {
                    Ok(song) => {
                        let mut idx = idx.lock().unwrap();
                        idx.add_song(song.clone());
                        idx.ensure_category(&category);
                        config::save_local_index(&temp_cfg, &idx);
                        Ok::<_, Infallible>(ok_json(&serde_json::json!({
                            "saved": true,
                            "filename": song.filename,
                            "path": song.path,
                            "quality": song.quality,
                            "category": category,
                            "file_size": song.size
                        })))
                    }
                    Err(e) => Ok(err(&e)),
                }
            },
        );

    // ── 打开文件夹 ──
    let open_folder = warp::path!("api" / "open-folder")
        .and(with_config(config_thread.clone()))
        .and_then(|cfg: Arc<Mutex<AppConfig>>| async move {
            let dir = cfg.lock().unwrap().download_dir.clone();
            let _ = fs::create_dir_all(&dir);
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(&dir)
                    .spawn();
            }
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "ok": true,
                "folder": dir.to_string_lossy()
            })))
        });

    // ── 获取配置 ──
    let get_config_route = warp::path!("api" / "config")
        .and(with_config(config_thread.clone()))
        .and_then(|cfg: Arc<Mutex<AppConfig>>| async move {
            let c = cfg.lock().unwrap();
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "download_dir": c.download_dir.to_string_lossy()
            })))
        });

    // ── 设置下载目录 ──
    let set_download_dir = warp::path!("api" / "set-download-dir")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_config(config_thread.clone()))
        .and_then(|q: HashMap<String, String>, cfg: Arc<Mutex<AppConfig>>| async move {
            let dir = q.get("dir").cloned().unwrap_or_default();
            if dir.is_empty() {
                return Ok::<_, Infallible>(err("目录为空"));
            }
            let p = PathBuf::from(&dir);
            let valid = if p.exists() {
                true
            } else {
                p.parent().map(|parent| parent.exists()).unwrap_or(false)
                    && fs::create_dir_all(&p).is_ok()
            };
            if !valid {
                return Ok::<_, Infallible>(err("目录无效或无法创建"));
            }
            let mut c = cfg.lock().unwrap();
            c.download_dir = p.clone();
            config::save_config(&c);
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "ok": true,
                "download_dir": dir
            })))
        });

    // ── 浏览文件夹 ──
    let browse_dir = warp::path!("api" / "browse-dir")
        .and_then(|| async move {
            #[cfg(windows)]
            {
                let ps = r#"Add-Type -AssemblyName System.Windows.Forms; $f=New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description='选择下载目录'; $f.ShowNewFolderButton=$true; if($f.ShowDialog() -eq 'OK'){Write-Host $f.SelectedPath}"#;
                match std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-WindowStyle",
                        "Hidden",
                        "-Command",
                        ps,
                    ])
                    .creation_flags(0x08000000)
                    .output()
                {
                    Ok(out) => {
                        let path = String::from_utf8_lossy(&out.stdout)
                            .trim()
                            .to_string();
                        if path.is_empty() {
                            Ok::<_, Infallible>(ok_json(&serde_json::json!({"cancelled": true})))
                        } else {
                            Ok::<_, Infallible>(ok_json(&serde_json::json!({"path": path})))
                        }
                    }
                    Err(_) => Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "error": "无法打开文件夹选择器"
                    }))),
                }
            }
            #[cfg(not(windows))]
            {
                let _ = (); // 非Windows暂不支持
                Ok::<_, Infallible>(ok_json(&serde_json::json!({"error": "不支持"})))
            }
        });

    // ── 本地歌曲列表 ──
    let local_songs_route = warp::path!("api" / "local-songs")
        .and(with_index(idx_thread.clone()))
        .and_then(|idx: Arc<Mutex<LocalIndexData>>| async move {
            let idx = idx.lock().unwrap();
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "songs": idx.songs,
                "categories": idx.categories
            })))
        });

    // ── 分类管理 ──
    let categories_list = warp::path!("api" / "categories")
        .and(with_index(idx_thread.clone()))
        .and_then(|idx: Arc<Mutex<LocalIndexData>>| async move {
            let idx = idx.lock().unwrap();
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "categories": idx.categories
            })))
        });

    let add_category = warp::path!("api" / "categories" / "add")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_index(idx_thread.clone()))
        .and(with_config(config_thread.clone()))
        .and_then(
            |q: HashMap<String, String>,
             idx: Arc<Mutex<LocalIndexData>>,
             cfg: Arc<Mutex<AppConfig>>| async move {
                let cat = q.get("name").cloned().unwrap_or_default();
                if cat.is_empty() {
                    return Ok::<_, Infallible>(err("分类名不能为空"));
                }
                let mut idx = idx.lock().unwrap();
                let cfg_lock = cfg.lock().unwrap();
                let dir = cfg_lock.download_dir.join(&cat);
                let _ = fs::create_dir_all(&dir);
                if idx.add_category(&cat) {
                    config::save_local_index(&cfg_lock, &idx);
                }
                Ok::<_, Infallible>(ok_json(&serde_json::json!({
                    "ok": true,
                    "categories": idx.categories
                })))
            },
        );

    // ── 客户端日志上报 ──
    let client_log = warp::path!("api" / "log")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(|msg: serde_json::Value| async move {
            let module = msg
                .get("module")
                .and_then(|v| v.as_str())
                .unwrap_or("frontend");
            let text = msg.get("msg").and_then(|v| v.as_str()).unwrap_or("");
            log_info!(module, "{text}");
            Ok::<_, Infallible>(ok_json(&serde_json::json!({"ok": true})))
        });

    // ── 歌词窗口(弹出) ──
    let lyrics_window = warp::path!("api" / "lyrics-window")
        .and(warp::query::<HashMap<String, String>>())
        .and(with_app_handle(app_handle_arc.clone()))
        .and_then(
            |q: HashMap<String, String>,
             app_h: Arc<Mutex<Option<tauri::AppHandle>>>| async move {
                let port: u16 = q.get("port").and_then(|v| v.parse().ok()).unwrap_or(8899);
                let url = format!("http://127.0.0.1:{port}/lyrics-view?port={port}");
                if let Some(app) = app_h.lock().unwrap().clone() {
                    if let Some(win) = app.get_webview_window("lyrics") {
                        win.set_focus().ok();
                    } else {
                        use tauri::webview::WebviewWindowBuilder;
                        use tauri::WebviewUrl;
                        match WebviewWindowBuilder::new(
                            &app,
                            "lyrics",
                            WebviewUrl::External(url.parse().unwrap()),
                        )
                        .title("歌词 - 音楽自由")
                        .inner_size(340.0, 600.0)
                        .position(80.0, 50.0)
                        .always_on_top(true)
                        .build()
                        {
                            Ok(_) => log_info!("lyrics", "歌词窗口已创建 (always-on-top)"),
                            Err(e) => log_info!("lyrics", "歌词窗口创建失败: {e}"),
                        }
                    }
                }
                Ok::<_, Infallible>(ok_json(&serde_json::json!({"ok": true})))
            },
        );

    // ── 歌词置顶切换 ──
    let lyrics_pin = warp::path!("api" / "lyrics-pin")
        .and(warp::method())
        .and(with_app_handle(app_handle_arc.clone()))
        .and(with_pinned(lyrics_pinned.clone()))
        .and_then(
            |method: warp::http::Method,
             app_h: Arc<Mutex<Option<tauri::AppHandle>>>,
             pinned: Arc<Mutex<bool>>| async move {
                if method == warp::http::Method::POST {
                    let new_state = {
                        let mut p = pinned.lock().unwrap();
                        *p = !*p;
                        *p
                    };
                    if let Some(app) = app_h.lock().unwrap().clone() {
                        if let Some(win) = app.get_webview_window("lyrics") {
                            win.set_always_on_top(new_state).ok();
                            log_info!("lyrics", "歌词窗口置顶: {new_state}");
                        }
                    }
                    Ok::<_, Infallible>(ok_json(&serde_json::json!({"pinned": new_state})))
                } else {
                    let p = *pinned.lock().unwrap();
                    Ok::<_, Infallible>(ok_json(&serde_json::json!({"pinned": p})))
                }
            },
        );

    // ── API: 插件中心 - 列出所有已注册音源 ──
    let plugins_route = warp::path!("api" / "plugins")
        .and(with_client(c.clone()))
        .and({
            let reg = plugin_registry.clone();
            warp::any().map(move || reg.clone())
        })
        .and_then(
            |_c: Arc<Client>, reg: Arc<Mutex<PlatformRegistry>>| async move {
                let plugins = reg.lock().unwrap().list();
                Ok::<_, Infallible>(ok_json_obj(&serde_json::json!({
                    "plugins": plugins.iter().map(|p| serde_json::json!({
                        "id": p.id,
                        "name": p.name,
                        "description": p.description,
                        "version": p.version,
                        "capability": {
                            "search": p.capability.search,
                            "play_url": p.capability.play_url,
                            "lyrics": p.capability.lyrics,
                            "browser_playable": p.capability.browser_playable,
                            "needs_auth": p.capability.needs_auth,
                            "stability": p.capability.stability,
                        }
                    })).collect::<Vec<_>>(),
                    "total": plugins.len()
                })))
            },
        );

    // ── API: 社区源市场 — 查询内置的第三方音源目录 ──
    let community_sources = warp::path!("api" / "community" / "sources")
        .and(warp::query::<HashMap<String, String>>())
        .and_then(|q: HashMap<String, String>| async move {
            let mut sources = community::builtin_catalog();
            // 筛选 api_type
            if let Some(api_type) = q.get("api_type") {
                sources = community::filter_by_type(&sources, api_type);
            }
            // 筛选 status
            if let Some(status) = q.get("status") {
                sources = community::filter_by_status(&sources, status);
            }
            Ok::<_, Infallible>(ok_json_obj(&serde_json::json!({
                "sources": sources,
                "total": sources.len()
            })))
        });

    // ── API: now-playing — 主窗口更新当前播放，歌词窗口轮询 ──
    let np_state = now_playing.clone();
    let now_playing_set = warp::path!("api" / "now-playing")
        .and(warp::header::optional::<String>("content-type"))
        .and(warp::body::json())
        .and(with_now_playing(np_state.clone()))
        .and_then(
            |_ct: Option<String>,
             body: serde_json::Value,
             np: Arc<Mutex<Option<NowPlaying>>>| async move {
                let music_id = body["music_id"].as_str().unwrap_or("").to_string();
                let song_id = body["song_id"].as_str().unwrap_or("").to_string();
                let title = body["title"].as_str().unwrap_or("").to_string();
                let artist = body["artist"].as_str().unwrap_or("").to_string();
                let source = body["source"].as_str().unwrap_or("").to_string();
                let source_id = body["source_id"].as_str().unwrap_or("").to_string();
                let duration = body["duration"].as_f64().unwrap_or(0.0);
                let info = NowPlaying {
                    music_id,
                    song_id,
                    title,
                    artist,
                    source,
                    source_id,
                    duration,
                };
                *np.lock().unwrap() = Some(info);
                Ok::<_, Infallible>(ok_json(&serde_json::json!({"ok": true})))
            },
        );

    let np_get_state = now_playing.clone();
    let now_playing_get = warp::path!("api" / "now-playing")
        .and(warp::get())
        .and(with_now_playing(np_get_state.clone()))
        .and_then(
            |np: Arc<Mutex<Option<NowPlaying>>>| async move {
                let v = np.lock().unwrap().clone();
                Ok::<_, Infallible>(ok_json_obj(&v))
            },
        );

    // ── API: 导入本地文件夹 ──
    let np_for_import = now_playing.clone();
    let import_folder = warp::path!("api" / "import-folder")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_config(config_thread.clone()))
        .and(warp::any().map(move || np_for_import.clone()))
        .and_then(
            |body: serde_json::Value,
             cfg: Arc<Mutex<AppConfig>>,
             _np: Arc<Mutex<Option<NowPlaying>>>| async move {
                let dir_path = body["path"].as_str().unwrap_or("").to_string();
                let folder_id = body["folder_id"].as_str().map(|s| s.to_string());
                if dir_path.is_empty() {
                    return Ok::<_, Infallible>(err("请提供文件夹路径"));
                }
                let src = std::path::PathBuf::from(&dir_path);
                if !src.exists() || !src.is_dir() {
                    return Ok::<_, Infallible>(err("文件夹不存在"));
                }

                let dl_dir = cfg.lock().unwrap().download_dir.clone();
                let _ = fs::create_dir_all(&dl_dir);

                let audio_exts = ["mp3", "flac", "m4a", "wav", "ogg", "aac", "wma"];
                let mut imported: Vec<serde_json::Value> = vec![];
                let mut skipped = 0u32;

                match std::fs::read_dir(&src) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if !path.is_file() { continue; }
                            let ext = path.extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if !audio_exts.contains(&ext.as_str()) { continue; }

                            let fname = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");
                            let dest = dl_dir.join(fname);
                            // 防重名
                            let mut dest = dest;
                            let mut counter = 1;
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("track");
                            while dest.exists() {
                                dest = dl_dir.join(format!("{}_{}.{}", stem, counter, ext));
                                counter += 1;
                            }

                            match fs::copy(&path, &dest) {
                                Ok(_) => {
                                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                                    // 从文件名推断 title/artist
                                    let raw_name = path.file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("unknown");
                                    let (title, artist) = parse_filename_artist(raw_name);
                                    let source_id = sanitize_filename(&title);
                                    imported.push(serde_json::json!({
                                        "title": title,
                                        "artist": artist,
                                        "filename": dest.file_name().unwrap_or_default().to_string_lossy(),
                                        "path": dest.to_string_lossy(),
                                        "size": size,
                                        "source": "local",
                                        "source_id": source_id,
                                    }));
                                }
                                Err(e) => {
                                    log_info!("import", "复制失败 {}: {e}", path.display());
                                    skipped += 1;
                                }
                            }
                        }
                    }
                    Err(e) => return Ok::<_, Infallible>(err(&format!("读取目录失败: {e}"))),
                }

                Ok::<_, Infallible>(ok_json(&serde_json::json!({
                    "imported": imported,
                    "skipped": skipped,
                    "total": imported.len() + skipped as usize,
                    "folder_id": folder_id,
                })))
            },
        );

    // ── 酷狗登录 API ──

    // GET /api/kugou/qr-key — 获取扫码登录二维码 key
    let kugou_qr_key = warp::path!("api" / "kugou" / "qr-key")
        .and(warp::get())
        .and(with_client(c.clone()))
        .and_then(|c: Arc<Client>| async move {
            match kugou::qr_key(&c).await {
                Ok(key) => {
                    log_info!("kugou", "二维码key获取成功: {key}");
                    Ok::<_, Infallible>(ok_json(&serde_json::json!({"key": key})))
                }
                Err(e) => {
                    log_info!("kugou", "二维码key获取失败: {e}");
                    Ok(err(&e))
                }
            }
        });

    // GET /api/kugou/qr-check?key=xxx — 轮询扫码状态
    let kugou_qr_check = warp::path!("api" / "kugou" / "qr-check")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(with_client(c.clone()))
        .and_then(|q: HashMap<String, String>, c: Arc<Client>| async move {
            let key = q.get("key").cloned().unwrap_or_default();
            if key.is_empty() {
                return Ok::<_, Infallible>(err("缺少key参数"));
            }
            match kugou::qr_check(&c, &key).await {
                Ok(resp) => Ok::<_, Infallible>(ok_json(&resp)),
                Err(e) => {
                    log_info!("kugou", "扫码状态查询失败: {e}");
                    Ok(err(&e))
                }
            }
        });

    // POST /api/kugou/login-pwd — 密码登录
    let kugou_login_pwd = warp::path!("api" / "kugou" / "login-pwd")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_client(c.clone()))
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(|body: serde_json::Value, c: Arc<Client>, auth: Arc<Mutex<KugouAuth>>| async move {
            let username = body["username"].as_str().unwrap_or("").to_string();
            let password = body["password"].as_str().unwrap_or("").to_string();
            if username.is_empty() || password.is_empty() {
                return Ok::<_, Infallible>(err("用户名或密码为空"));
            }

            // 设备注册获取 dfid
            let dfid = match kugou::register_device(&c).await {
                Ok(d) => d,
                Err(e) => {
                    log_info!("kugou", "设备注册失败: {e}");
                    return Ok::<_, Infallible>(err(&e));
                }
            };

            log_info!("kugou", "设备注册成功 dfid={dfid}");

            // 密码登录
            match kugou::login_by_pwd(&c, &username, &password, &dfid).await {
                Ok(result) => {
                    let token = result["token"].as_str().unwrap_or("").to_string();
                    let userid = result["userid"].as_u64().unwrap_or(0);
                    let vip_token = result["vip_token"].as_str().unwrap_or("").to_string();
                    let vip_type = result["vip_type"].as_u64().unwrap_or(0) as u32;

                    // 保存登录态
                    let mut auth = auth.lock().unwrap();
                    auth.dfid = dfid;
                    auth.token = token.clone();
                    auth.userid = userid;
                    auth.vip_token = vip_token;
                    auth.vip_type = vip_type;
                    auth.username = username.clone();
                    auth.logged_in = true;
                    config::save_kugou_auth(&auth);

                    log_info!("kugou", "密码登录成功 userid={userid}");
                    Ok::<_, Infallible>(ok_json(&serde_json::json!({
                        "ok": true,
                        "token": token,
                        "userid": userid,
                        "vip_type": vip_type,
                        "username": username
                    })))
                }
                Err(e) => {
                    log_info!("kugou", "密码登录失败: {e}");
                    Ok(err(&e))
                }
            }
        });

    // GET /api/kugou/status — 查询登录状态
    let kugou_status = warp::path!("api" / "kugou" / "status")
        .and(warp::get())
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(|auth: Arc<Mutex<KugouAuth>>| async move {
            let a = auth.lock().unwrap();
            // 模糊用户名显示（隐藏中间位）
            let display_name = if a.logged_in && !a.username.is_empty() {
                let u = &a.username;
                if u.len() >= 7 {
                    format!("{}****{}", &u[..3], &u[u.len()-4..])
                } else if u.len() >= 2 {
                    format!("{}***", &u[..u.len()-1])
                } else {
                    a.username.clone()
                }
            } else {
                String::new()
            };
            Ok::<_, Infallible>(ok_json(&serde_json::json!({
                "logged_in": a.logged_in,
                "username": display_name,
                "vip_type": a.vip_type
            })))
        });

    // POST /api/kugou/logout — 清除登录态 (C1)
    let kugou_logout = warp::path!("api" / "kugou" / "logout")
        .and(warp::post())
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(|auth: Arc<Mutex<KugouAuth>>| async move {
            let mut a = auth.lock().unwrap();
            *a = KugouAuth::default(); // 重置为默认（保留硬编码 dfid）
            config::save_kugou_auth(&a);
            Ok::<_, Infallible>(ok_json(&serde_json::json!({"ok": true})))
        });

    // ── 前端统一登出（清除酷狗 token + 重置插件 auth）──
    let logout_route = warp::path!("api" / "logout")
        .and(with_kugou_auth(kugou_auth.clone()))
        .and_then(|auth: Arc<Mutex<KugouAuth>>| async move {
            {
                let mut a = auth.lock().unwrap();
                *a = KugouAuth::default();
            }
            log_info!("auth", "登出: token 已清除");
            Ok::<_, Infallible>(ok_json(&serde_json::json!({"ok": true})))
        });

    // POST /api/kugou/sync-playlists — 同步酷狗收藏到本地库 (P8-2 B2)
    let kugou_sync = warp::path!("api" / "kugou" / "sync-playlists")
        .and(warp::post())
        .and(with_client(c.clone()))
        .and(with_kugou_auth(kugou_auth.clone()))
        .and(with_lib_index(lib_index.clone()))
        .and_then(
            |c: Arc<Client>,
             auth: Arc<Mutex<KugouAuth>>,
             libs: Arc<Mutex<LibrariesIndex>>| async move {
                let a = auth.lock().unwrap().clone();
                if !a.logged_in {
                    return Ok::<_, Infallible>(err("请先登录酷狗"));
                }

                // 1. 获取歌单列表
                let playlists = match kugou::fetch_user_playlists(&c, &a.token, a.userid).await {
                    Ok(p) => p,
                    Err(e) => return Ok::<_, Infallible>(err(&e)),
                };

                let mut result_playlists: Vec<serde_json::Value> = vec![];
                let mut total_synced = 0usize;

                // 2. 逐个歌单拉歌曲并同步
                for pl in &playlists {
                    let pl_id = pl.get("id")
                        .and_then(|v| v.as_u64())
                        .or_else(|| pl.get("id").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
                        .map(|v: u64| v.to_string())
                        .unwrap_or_default();
                    let pl_name = pl.get("name").and_then(|v| v.as_str()).unwrap_or("未知歌单").to_string();
                    let pl_count = pl.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    if pl_id.is_empty() || pl_count == 0 {
                        continue;
                    }

                    // 分页拉取全部歌曲
                    let page_size = 30usize;
                    let total_pages = (pl_count + page_size - 1) / page_size;
                    let mut all_tracks: Vec<Value> = vec![];

                    for pg in 1..=total_pages {
                        match kugou::fetch_playlist_tracks(&c, &pl_id, pg, page_size).await {
                            Ok(tracks) => {
                                let fetched = tracks.len();
                                all_tracks.extend(tracks);
                                if fetched < page_size {
                                    break; // 最后一页
                                }
                            }
                            Err(e) => {
                                log_info!("kugou", "歌单 {pl_name} 第{pg}页拉取失败: {e}");
                                break;
                            }
                        }
                    }

                    // 转换歌曲为 SongImport
                    let imports: Vec<SongImport> = all_tracks.iter().filter_map(|t| {
                        let file_hash = t.get("FileHash").and_then(|v| v.as_str()).unwrap_or("");
                        let album_id = t.get("AlbumID").and_then(|v| v.as_str()).unwrap_or("");
                        if file_hash.is_empty() {
                            return None;
                        }
                        let source_id = if album_id.is_empty() {
                            file_hash.to_string()
                        } else {
                            format!("{file_hash}|{album_id}")
                        };
                        Some(SongImport {
                            title: t.get("FileName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            artist: t.get("SingerName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            source: "kugou".to_string(),
                            source_id,
                            file_path: String::new(),
                        })
                    }).collect();

                    // 同步到库
                    let (_, count) = {
                        let mut libs = libs.lock().unwrap();
                        library::sync_playlist_to_library(&mut libs, &pl_name, imports)
                    };
                    library::save_libraries_index(&libs.lock().unwrap());

                    total_synced += count;
                    result_playlists.push(serde_json::json!({
                        "name": pl_name,
                        "count": count,
                    }));

                    log_info!("kugou", "歌单同步: {pl_name} → {count} 首");
                }

                Ok::<_, Infallible>(ok_json(&serde_json::json!({
                    "synced": total_synced,
                    "playlists": result_playlists,
                })))
            },
        );

    // ── 歌词窗口诊断上报 ──
    let lyrics_diag = warp::path!("api" / "lyrics-diag")
        .and(warp::body::json())
        .map(|body: serde_json::Value| {
            log_info!("diagnostic", "歌词窗口上报: {body}");
            warp::reply::json(&serde_json::json!({"ok": true}))
        });

    // ── 合并所有路由 ──
    let routes = health
        .or(plugins_route)
        .or(community_sources)
        .or(kugou_qr_key)
        .or(kugou_qr_check)
        .or(kugou_login_pwd)
        .or(kugou_status)
        .or(kugou_logout)
        .or(logout_route)
        .or(kugou_sync)
        .or(now_playing_set)
        .or(now_playing_get)
        .or(import_folder)
        .or(lyrics_diag)
        .or(index)
        .or(dist_static)   // Phase 1: /dist/* 静态文件 (warp serve frontend/dist/)
        .or(assets_static)  // /assets/* 静态文件 (Vite build 产物)
        .or(root_static)    // 顶层静态文件 (sw.js, registerSW.js, manifest.webmanifest 等)
        .or(warp::path("app.css").map(|| app_css_html()))
        .or(warp::path("app.js").map(|| app_js_html()))
        .or(lyrics_view)
        .or(lyrics_html)
        .or(search)
        .or(play)
        .or(kugou_play)
        .or(qq_play)
        .or(bili_play)
        .or(bili_stream)
        .or(local_file)
        .or(lyric)
        .or(download_route)
        .or(kugou_download)
        .or(open_folder)
        .or(get_config_route)
        .or(set_download_dir)
        .or(browse_dir)
        .or(local_songs_route)
        .or(categories_list)
        .or(add_category)
        .or(client_log)
        .or(lyrics_window)
        .or(lyrics_pin)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_methods(vec!["GET", "POST"])
                .allow_header("content-type"),
        );

    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    log_info!("server", "HTTP 服务启动 http://{addr}");
    warp::serve(routes).run(addr).await;
}
