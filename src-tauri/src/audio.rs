// ════════════════════════════════════════════════
// 模块: audio.rs — 米豆音乐 v2.0
// 关键修复（2026-07-27）:
//   Player 线程改用 debug_log::info() 写日志（不走 eprintln!）
//   加 Cmd::Ping 确认线程存活
//   download() 在 Player 线程里做（test4 验证 OK）
// ════════════════════════════════════════════════

use crate::debug_log;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── 命令通道 ─────────────────────────────────────

enum Cmd {
    /// 播放 URL（Player 线程内下载）
    PlayUrl(String),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    GetState(mpsc::Sender<AudioState>),
    /// 确认线程存活
    Ping(mpsc::Sender<bool>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub position: f64,
    pub duration: f64,
}

// ── 对外句柄（Send） ──────────────────────────

pub struct AudioHandle {
    sender: mpsc::Sender<Cmd>,
}

impl AudioHandle {
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Cmd>();

        std::thread::Builder::new()
            .name("midou-audio".into())
            .spawn(move || run_player(rx))
            .map_err(|e| format!("启动音频线程失败: {e}"))?;

        debug_log::info("audio", "AudioHandle 创建完成");
        Ok(Self { sender: tx })
    }

    pub fn play(&self, url: String) -> Result<(), String> {
        self.sender.send(Cmd::PlayUrl(url))
            .map_err(|_| "音频线程已退出".to_string())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.sender.send(Cmd::Pause)
            .map_err(|_| "音频线程已退出".to_string())
    }

    pub fn resume(&self) -> Result<(), String> {
        self.sender.send(Cmd::Resume)
            .map_err(|_| "音频线程已退出".to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        self.sender.send(Cmd::Stop)
            .map_err(|_| "音频线程已退出".to_string())
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.sender.send(Cmd::SetVolume(volume))
            .map_err(|_| "音频线程已退出".to_string())
    }

    pub fn get_state(&self) -> Option<AudioState> {
        let (tx, rx) = mpsc::channel();
        self.sender.send(Cmd::GetState(tx)).ok()?;
        rx.recv_timeout(Duration::from_millis(100)).ok()
    }

    /// 确认 Player 线程是否还活着
    pub fn ping(&self) -> bool {
        let (tx, rx) = mpsc::channel();
        if self.sender.send(Cmd::Ping(tx)).is_err() {
            return false;
        }
        rx.recv_timeout(Duration::from_secs(1)).ok() == Some(true)
    }
}

// ── Player ────────────────────────────────────

struct Player {
    _stream: OutputStream,
    sink: Sink,
    start_time: Option<Instant>,
    paused_at: Option<Duration>,
    duration: Option<Duration>,
}

impl Player {
    fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| format!("无法创建音频输出设备: {e}"))?;
        let sink = Sink::try_new(&handle)
            .map_err(|e| format!("无法创建音频接收器: {e}"))?;
        
        sink.play(); // ★ 确保 sink 不在 paused 状态
        
        debug_log::info("midou-audio", "Player 创建成功");

        Ok(Self {
            _stream: stream,
            sink,
            start_time: None,
            paused_at: None,
            duration: None,
        })
    }

    fn play_bytes(&mut self, data: &[u8]) -> Result<(), String> {
        debug_log::info("midou-audio", &format!("play_bytes 入口: {} 字节", data.len()));
        
        self.sink.stop();
        self.sink.clear();

        if data.is_empty() {
            return Err("音频数据为空".to_string());
        }

        let cursor = Cursor::new(data.to_vec());
        debug_log::info("midou-audio", "开始解码...");
        let source = Decoder::new(cursor)
            .map_err(|e| format!("解码失败: {e}"))?;
        debug_log::info("midou-audio", "解码成功");

        self.duration = source.total_duration();
        self.sink.append(source);
        self.sink.play(); // ★ 强制播放
        self.start_time = Some(Instant::now());
        self.paused_at = None;

        debug_log::info("midou-audio", "playback started");
        
        // ★ 播放后立刻测试设备存活
        std::thread::sleep(Duration::from_millis(100));
        debug_log::info("midou-audio", &format!(
            "sink empty={}, len={}, paused={}",
            self.sink.empty(),
            self.sink.len(),
            self.sink.is_paused()
        ));
        
        Ok(())
    }

    /// ★ 下载+播放（在 Player 线程里执行）
    fn play_url(&mut self, url: &str) {
        debug_log::info("midou-audio", &format!("开始下载: {}", url));

        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                debug_log::error("midou-audio", &format!("HTTP 客户端创建失败: {e}"));
                return;
            }
        };

        match client.get(url).send() {
            Ok(response) => {
                let status = response.status().as_u16();
                debug_log::info("midou-audio", &format!("HTTP {}", status));
                match response.bytes() {
                    Ok(data) => {
                        let len = data.len();
                        debug_log::info("midou-audio", &format!("下载完成 {} 字节", len));
                        if let Err(e) = self.play_bytes(&data) {
                            debug_log::error("midou-audio", &format!("播放失败: {e}"));
                        }
                    }
                    Err(e) => debug_log::error("midou-audio", &format!("读取数据失败: {e}")),
                }
            }
            Err(e) => debug_log::error("midou-audio", &format!("下载失败: {e}")),
        }
    }

    fn pause(&mut self) {
        if !self.sink.is_paused() {
            if let Some(start) = self.start_time {
                self.paused_at = Some(self.elapsed_from(start));
            }
            self.sink.pause();
            debug_log::info("midou-audio", "paused");
        }
    }

    fn resume(&mut self) {
        if self.sink.is_paused() {
            self.sink.play();
            self.start_time = Some(Instant::now());
            debug_log::info("midou-audio", "resumed");
        }
    }

    fn stop(&mut self) {
        self.sink.stop();
        self.sink.clear();
        self.start_time = None;
        self.paused_at = None;
        self.duration = None;
        debug_log::info("midou-audio", "stopped");
    }

    fn get_state(&self) -> AudioState {
        let is_paused = self.sink.is_paused();
        let duration = self.duration.map(|d| d.as_secs_f64()).unwrap_or(0.0);

        let position = if is_paused {
            self.paused_at.map(|d| d.as_secs_f64()).unwrap_or(0.0)
        } else if let Some(start) = self.start_time {
            self.elapsed_from(start).as_secs_f64()
        } else {
            0.0
        };

        AudioState {
            is_playing: !is_paused && self.sink.len() > 0,
            is_paused,
            position,
            duration,
        }
    }

    fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }

    fn elapsed_from(&self, start: Instant) -> Duration {
        let base = self.paused_at.unwrap_or(Duration::ZERO);
        base + start.elapsed()
    }
}

// ── 音频线程主循环 ─────────────────────────────

fn run_player(rx: mpsc::Receiver<Cmd>) {
    debug_log::info("midou-audio", "线程启动");

    let mut player = match Player::new() {
        Ok(p) => p,
        Err(e) => {
            debug_log::error("midou-audio", &format!("初始化失败: {e}"));
            return;
        }
    };

    loop {
        let cmd = match rx.recv() {
            Ok(c) => c,
            Err(_) => {
                debug_log::info("midou-audio", "通道断开，退出");
                break;
            }
        };

        match cmd {
            Cmd::PlayUrl(url) => {
                // ★ 关键：下载在 Player 线程里
                player.play_url(&url);
            }
            Cmd::Pause => player.pause(),
            Cmd::Resume => player.resume(),
            Cmd::Stop => player.stop(),
            Cmd::SetVolume(v) => player.set_volume(v),
            Cmd::GetState(tx) => {
                let _ = tx.send(player.get_state());
            }
            Cmd::Ping(tx) => {
                let _ = tx.send(true);
            }
        }
    }
}
