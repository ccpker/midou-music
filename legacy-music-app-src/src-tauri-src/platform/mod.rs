// ════════════════════════════════════════════════
// 音楽自由 — 音源插件框架
// ────────────────────────────────────────────
// 设计目标:
//   - 每个音源实现 PlatformPlugin trait
//   - 通过 PlatformRegistry 集中管理和路由
//   - 加新源 = 新建一个文件 + 实现 trait + 注册一行
//   - routes.rs 不再硬编码各源分支
//
// 参考:
//   - LX Music 的 JS 脚本插件体系
//   - MusicFree 的插件化架构
//   - Nuclear 的内置插件商店
//
// 框架: 结构优于配置，trait 是合约，注册中心是路由
// ════════════════════════════════════════════════

pub mod kugou;
pub mod kuwo;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

use crate::models::Song;

// ── 插件能力声明 ──

#[derive(Debug, Clone)]
pub struct PluginCapability {
    /// 支持搜索
    pub search: bool,
    /// 支持获取播放地址
    pub play_url: bool,
    /// 支持获取歌词
    pub lyrics: bool,
    /// 播放URL可直接用于浏览器 <audio>
    pub browser_playable: bool,
    /// 是否需要登录/Cookie
    pub needs_auth: bool,
    /// 稳定性评级 (0-100)
    pub stability: u8,
}

impl Default for PluginCapability {
    fn default() -> Self {
        Self {
            search: true, play_url: true, lyrics: true,
            browser_playable: true, needs_auth: false, stability: 80,
        }
    }
}

// ── 插件元信息 ──

#[derive(Debug, Clone)]
pub struct PluginMeta {
    /// 插件id (小写下划线, 如 "kuwo", "bilibili")
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 简短描述
    pub description: String,
    /// 版本
    pub version: String,
    /// 能力声明
    pub capability: PluginCapability,
}

// ── 核心 Trait: 音源插件接口 ──

/// 每个音源适配器必须实现此 trait
///
/// # 生命周期
/// - 编译时通过 registry 注册
/// - 运行时通过 registry 路由调用
///
/// # 约定
/// - search 返回 Vec<Song>, source 字段填 meta.id
/// - play_url 返回 { "url": "...", "quality": "..." }
/// - lyrics 返回原始歌词文本 (LRC 或纯文本), 无歌词返回 Ok(None)
#[async_trait]
pub trait PlatformPlugin: Send + Sync {
    /// 插件元信息
    fn meta(&self) -> PluginMeta;

    /// 搜索歌曲
    async fn search(&self, client: &Client, keyword: &str, page: usize, page_size: usize)
        -> Result<Vec<Song>, String>;

    /// 获取播放地址
    async fn play_url(&self, client: &Client, song_id: &str, quality: &str)
        -> Result<Value, String>;

    /// 获取歌词 (可选)
    /// 默认返回 None, 子类可 override
    async fn lyrics(&self, _client: &Client, _song_id: &str, _artist: &str, _title: &str)
        -> Result<Option<String>, String>
    {
        Ok(None)
    }

    /// 从搜索结果中提取 song_id (各平台格式不同)
    /// 默认按 source 字段格式处理, 子类可 override
    fn extract_id_from_url(&self, _url: &str) -> Option<String> { None }
}

// ── 插件注册中心 ──

/// 全局音源插件注册中心
///
/// # 使用方式
/// ```
/// let mut reg = PlatformRegistry::new();
/// reg.register(Arc::new(KuwoPlugin::new()));
/// reg.register(Arc::new(BilibiliPlugin::new()));
///
/// // 搜索: 按 id 路由
/// let songs = reg.search("kuwo", &client, "周杰伦", 0, 20).await;
///
/// // 枚举所有可用插件
/// for meta in reg.list_plugins() { ... }
/// ```
pub struct PlatformRegistry {
    plugins: Vec<Arc<dyn PlatformPlugin>>,
}

impl PlatformRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// 注册一个插件
    pub fn register(&mut self, plugin: Arc<dyn PlatformPlugin>) {
        let meta = plugin.meta();
        crate::log_info!("platform", "注册插件: {} v{} — {}", meta.id, meta.version, meta.description);
        self.plugins.push(plugin);
    }

    /// 列出所有已注册插件
    pub fn list(&self) -> Vec<PluginMeta> {
        self.plugins.iter().map(|p| p.meta()).collect()
    }

    /// 按 id 查找插件
    pub fn find(&self, id: &str) -> Option<&Arc<dyn PlatformPlugin>> {
        self.plugins.iter().find(|p| p.meta().id == id)
    }

    /// 通过插件搜索
    pub async fn search(
        &self, plugin_id: &str, client: &Client,
        keyword: &str, page: usize, page_size: usize,
    ) -> Result<Vec<Song>, String> {
        let plugin = self.find(plugin_id).ok_or_else(|| format!("未知插件: {plugin_id}"))?;
        plugin.search(client, keyword, page, page_size).await
    }

    /// 通过插件获取播放地址
    pub async fn play_url(
        &self, plugin_id: &str, client: &Client,
        song_id: &str, quality: &str,
    ) -> Result<Value, String> {
        let plugin = self.find(plugin_id).ok_or_else(|| format!("未知插件: {plugin_id}"))?;
        plugin.play_url(client, song_id, quality).await
    }

    /// 通过插件获取歌词
    pub async fn lyrics(
        &self, plugin_id: &str, client: &Client,
        song_id: &str, artist: &str, title: &str,
    ) -> Result<Option<String>, String> {
        let plugin = self.find(plugin_id).ok_or_else(|| format!("未知插件: {plugin_id}"))?;
        plugin.lyrics(client, song_id, artist, title).await
    }

    /// 批量跨源搜索: 同时搜索所有支持搜索的插件, 合并结果
    pub async fn search_all(
        &self, client: &Client, keyword: &str, page: usize, page_size: usize,
    ) -> Vec<Song> {
        let mut all = Vec::new();
        for plugin in &self.plugins {
            if plugin.meta().capability.search {
                match plugin.search(client, keyword, page, page_size).await {
                    Ok(mut songs) => all.append(&mut songs),
                    Err(e) => {
                        crate::log_info!("platform", "{} 搜索失败: {e}", plugin.meta().id);
                    }
                }
            }
        }
        all
    }
}

impl Default for PlatformRegistry {
    fn default() -> Self { Self::new() }
}

// ── 宏: 简化插件注册 ──

/// 批量注册插件的便捷宏
///
/// 用法:
/// ```
/// register_plugins!(registry,
///     KuwoPlugin,
///     BilibiliPlugin,
/// );
/// ```
#[macro_export]
macro_rules! register_plugins {
    ($reg:expr, $($plugin_type:ty),+ $(,)?) => {
        $(
            $reg.register(std::sync::Arc::new(<$plugin_type>::new()));
        )+
    };
}
