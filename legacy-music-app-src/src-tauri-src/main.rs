// ════════════════════════════════════════════════
// 音楽自由 v0.4.0 — 程序入口
// ────────────────────────────────────────────
// 功能:   初始化配置 → 启动 HTTP 服务 → 创建 Tauri 窗口
// 依赖:   routes, config, models
// 备注:   仅有main()一个函数,所有业务逻辑在路由层
// ════════════════════════════════════════════════

#![windows_subsystem = "windows"]
#![recursion_limit = "256"]

use std::sync::{Arc, Mutex};

#[macro_use]
mod platform;
mod bilibili;
mod community;
mod config;
mod download;
mod library;
mod lyrics;
mod models;
mod routes;
mod search;
mod utils;

use crate::platform::PlatformRegistry;
use tauri::Manager; // needed for .manage() and .get_webview_window() in IPC commands

// ── 入口 ──

fn main() {
    // Phase 1: warp 固定端口 8899（Vite proxy 约定端口，打包期同端口服务前端）
    let port: u16 = 8899;
    // 若 8899 被占用则 fallback 找随机可用端口（保持向后兼容）
    let actual_port = if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
        log_info!("launcher", "8899 已被占用，动态分配端口...");
        routes::find_free_port()
    } else {
        port
    };

    // ── 音源插件注册中心 ──
    let kugou_auth = Arc::new(Mutex::new(config::load_kugou_auth()));
    let mut plugin_registry = PlatformRegistry::new();
    plugin_registry.register(std::sync::Arc::new(platform::kuwo::KuwoPlugin::new()));
    plugin_registry.register(std::sync::Arc::new(platform::kugou::KugouPlugin::new(kugou_auth.clone())));
    let plugin_registry = Arc::new(Mutex::new(plugin_registry));
    log_info!("launcher", "已注册 {} 个音源插件", plugin_registry.lock().unwrap().list().len());

    let cfg = Arc::new(Mutex::new(config::load_config()));
    let local_idx = Arc::new(Mutex::new(config::load_local_index(
        &cfg.lock().unwrap(),
    )));
    let lib_index = Arc::new(Mutex::new(library::load_libraries_index()));
    let lib_index_thread = lib_index.clone();

    log_info!("launcher", "音楽自由 v0.4.0 启动 port={}", actual_port);
    log_info!(
        "launcher",
        "下载目录: {:?}",
        cfg.lock().unwrap().download_dir
    );
    log_info!(
        "launcher",
        "本地歌曲: {} 首",
        local_idx.lock().unwrap().songs.len()
    );

    // ── 歌词窗口状态(跨 warp/tauri 共享) ──
    let app_handle_arc: Arc<Mutex<Option<tauri::AppHandle>>> = Arc::new(Mutex::new(None));
    let lyrics_pinned: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let ah_arc = app_handle_arc.clone();
    let lp_arc = lyrics_pinned.clone();

    // ── 启动 HTTP 服务(独立线程) ──
    // 复用变量 actual_port 保持一致
    let warp_port = actual_port;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(routes::warp_main(
            warp_port,
            cfg.clone(),
            local_idx.clone(),
            ah_arc,
            lp_arc,
            plugin_registry.clone(),
            kugou_auth.clone(),
            lib_index_thread.clone(),
        ));
    });

    // ── 等待 warp 就绪 ──
    let timeout = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();
    loop {
        if std::net::TcpStream::connect(format!("127.0.0.1:{warp_port}"))
            .is_ok()
        {
            log_info!("launcher", "warp 就绪,端口 {warp_port}");
            break;
        }
        if start.elapsed() > timeout {
            panic!("warp 未在 {timeout:?} 内启动");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // ── 填充 app_handle_arc(供 warp 路由 & IPC 命令使用) ──
    let ah_for_setup = app_handle_arc.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(lib_index)
        // Phase 1+3: Tauri IPC 命令 — 供前端 Vue 调用
        .invoke_handler(tauri::generate_handler![
            open_lyrics_window_cmd,
            // ── Phase 3: 库管理 ──
            library::create_library,
            library::switch_library,
            library::delete_library,
            library::merge_library,
            library::list_libraries,
            library::set_master_library,
            library::add_song_to_folder,
            library::create_folder,
            library::delete_folder,
            library::move_to_trash,
            library::restore_from_trash,
            library::empty_trash,
            library::save_lyrics,
            library::get_lyrics,
            library::get_active_library,
            library::add_song_to_library,
        ])
        .setup(move |app| {
            ah_for_setup
                .lock()
                .unwrap()
                .replace(app.handle().clone());

            // Phase 1: 保存 warp_port 到 app 存储（供 IPC 命令读取）
            app.manage(Arc::new(Mutex::new(warp_port)));

            use tauri::webview::WebviewWindowBuilder;
            use tauri::WebviewUrl;

            // Phase 1: 生产模式 — Tauri 加载 warp 静态文件服务
            // 使用 path-only URL（查询串走 Warp 时会乱码）
            let dev_url = format!("http://127.0.0.1:{}/", warp_port);
            log_info!("launcher", "创建主窗口 url={dev_url}");

            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(dev_url.parse().unwrap()),
            )
            .title("")
            .inner_size(820.0, 700.0)
            .min_inner_size(520.0, 480.0)
            .build()?;

            // ── Phase 4: 歌词独立窗口 ──
            // 先加载测试页面验证拖拽/按钮是否正常
            let test_lyrics_url = format!("http://127.0.0.1:{}/test-lyrics.html", warp_port);
            log_info!("launcher", "创建歌词窗口(测试模式) url={test_lyrics_url}");

            WebviewWindowBuilder::new(
                app,
                "lyrics",
                WebviewUrl::External(test_lyrics_url.parse().unwrap()),
            )
            .title("歌词 - 音楽自由")
            .inner_size(500.0, 600.0)
            .position(100.0, 100.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .visible(true)
            .focused(true)
            .build()?;

            log_info!("launcher", "歌词窗口创建完成");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Phase 4: 主窗口关闭 → 歌词窗口自动关闭
            if window.label() == "main" {
                if let tauri::WindowEvent::Destroyed = event {
                    if let Some(lyrics_win) = window.app_handle().get_webview_window("lyrics") {
                        let _ = lyrics_win.close();
                        crate::log_info!("launcher", "主窗口关闭，已关闭歌词窗口");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Phase 4: Tauri IPC 命令 — 打开/聚焦歌词独立窗口
/// （window 已在 setup 创建，此命令仅聚焦已有窗口）
#[tauri::command]
fn open_lyrics_window_cmd(
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("lyrics") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        crate::log_info!("lyrics", "歌词窗口已聚焦");
        return Ok(());
    }

    Err("歌词窗口未找到（应在启动时自动创建）".into())
}
