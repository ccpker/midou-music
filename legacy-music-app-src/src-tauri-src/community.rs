// ════════════════════════════════════════════════
// 音楽自由 — 社区源市场
// ────────────────────────────────────────────
// 收录已知的第三方音源脚本/服务端信息
// 嵌入二进制，前端通过 /api/community/sources 查询
//
// 数据来源:
//   - LX Music 自定义源（lx-music-source）
//   - MusicFree 插件生态
//   - 其他公开音源
//
// 字段说明:
//   - source_url: 可直接下载/导入的脚本地址
//   - api_type: 协议类型（决定未来如何集成）
//     * "lx-music"     → LX Music 自定义源 JS 脚本
//     * "musicfree"    → MusicFree 插件（.js/.json）
//     * "generic-http" → 标准 HTTP API 端点
//   - status: active(社区活跃)/inactive(已失效)/unknown(待验证)
// ════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── 社区源数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySource {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub api_type: String,
    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<String>,

    pub added_at: String,
}

// ── 社区源总览（嵌入 JSON） ──

/// 内置社区源目录
/// 数据定期更新，万一哪天需要可随时查阅、启用
pub fn builtin_catalog() -> Vec<CommunitySource> {
    serde_json::from_str(BUILTIN_CATALOG_JSON).unwrap_or_default()
}

/// 按 api_type 筛选
pub fn filter_by_type(sources: &[CommunitySource], api_type: &str) -> Vec<CommunitySource> {
    sources.iter()
        .filter(|s| s.api_type == api_type)
        .cloned()
        .collect()
}

/// 按 status 筛选
pub fn filter_by_status(sources: &[CommunitySource], status: &str) -> Vec<CommunitySource> {
    sources.iter()
        .filter(|s| s.status == status)
        .cloned()
        .collect()
}

// ════════════════════════════════════════════════
// 内置目录 JSON
// ════════════════════════════════════════════════

const BUILTIN_CATALOG_JSON: &str = r#"[
  {
    "id": "sixyin",
    "name": "六音音源",
    "description": "LX Music 社区音源，聚合多平台搜索/播放，需自建或使用公开服务端",
    "version": "v1.2.1",
    "author": "六音",
    "api_type": "lx-music",
    "status": "active",
    "homepage": "https://www.sixyin.com",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/sixyin/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "六音提供的是代理服务器地址，需配合 lx-music-api-server 使用。JS 文件本身是混淆的，指向一台 Python/Node 后端服务器。本项目可通过 HttpProxyPlugin 对接。"
  },
  {
    "id": "huibq",
    "name": "Huibq 音源",
    "description": "LX Music 社区音源，同作者有小枸/小秋/小蜗/小蜜/小芸系列（MusicFree 版）",
    "version": "latest",
    "author": "Huibq",
    "api_type": "lx-music",
    "status": "active",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/huibq/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "同六音架构。该作者也发布了 MusicFree 插件（xiaogou/xiaoqiu 系列），覆盖两个生态。"
  },
  {
    "id": "flower",
    "name": "野花音源",
    "description": "LX Music 社区音源，早期较流行的第三方源",
    "version": "latest",
    "author": "野花",
    "api_type": "lx-music",
    "status": "unknown",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/flower/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "近况不明，需验证是否仍可用。"
  },
  {
    "id": "ikun",
    "name": "ikun 音源",
    "description": "LX Music 社区音源，已永久失效（作者关停服务端）",
    "version": "latest",
    "author": "ikun",
    "api_type": "lx-music",
    "status": "inactive",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/ikun/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "已确认失效。收录于此作为参考，也提醒：社区代理服务器有跑路风险，本项目自建直连方案更可靠。"
  },
  {
    "id": "grass",
    "name": "Grass 音源",
    "description": "LX Music 社区音源",
    "version": "latest",
    "author": "Grass",
    "api_type": "lx-music",
    "status": "unknown",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/grass/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": ""
  },
  {
    "id": "lx-official",
    "name": "LX Music 官方源",
    "description": "LX Music 官方维护的音源脚本，相对稳定",
    "version": "latest",
    "author": "LX Music",
    "api_type": "lx-music",
    "status": "active",
    "repository": "https://github.com/pdone/lx-music-source",
    "source_url": "https://raw.githubusercontent.com/pdone/lx-music-source/main/lx/latest.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "LX Music 官方源，API 协议与六音等相同，需要服务端。"
  },
  {
    "id": "musicfree-maotoumao",
    "name": "MusicFree 插件合集（猫头猫）",
    "description": "MusicFree 官方推荐的插件仓库，包含多平台音源插件（.js/.json）",
    "version": "latest",
    "author": "maotoumao",
    "api_type": "musicfree",
    "status": "active",
    "repository": "https://github.com/maotoumao/MusicFreePlugins",
    "source_url": "https://gitee.com/maotoumao/MusicFreePlugins/raw/master/plugins.json",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "MusicFree 插件协议: search(query,page,type)→ISearchResult, getMediaSource(item,quality)→IMediaSourceResult, getLyric(item)→ILyricSource。本项目可通过 JsPluginBridge（QuickJS 嵌入）直接加载这些插件。"
  },
  {
    "id": "musicfree-xiaogou",
    "name": "小枸音乐（MusicFree）",
    "description": "MusicFree 社区插件，Huibq 出品",
    "version": "latest",
    "author": "Huibq",
    "api_type": "musicfree",
    "status": "active",
    "source_url": "https://raw.niuma666bet.buzz/Huibq/keep-alive/master/Music_Free/xiaogou.js",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "MusicFree 插件，可直接在 MusicFree 中导入。本项目通过 JsPluginBridge 支持。"
  },
  {
    "id": "musicfree-plugins-hub",
    "name": "MusicFree 插件中心",
    "description": "第三方维护的 MusicFree 插件聚合索引",
    "version": "latest",
    "author": "社区",
    "api_type": "musicfree",
    "status": "active",
    "source_url": "https://musicfreepluginshub.2020818.xyz/plugins.json",
    "last_checked": "2026-07-17",
    "added_at": "2026-07-17",
    "notes": "第三方聚合站，稳定性和安全性需自行评估。"
  },
  {
    "id": "lx-music-api-server-python",
    "name": "LX Music API Server（Python）",
    "description": "LX Music 自定义源的后端服务端实现（Python），可自建",
    "version": "latest",
    "author": "MeoProject",
    "api_type": "generic-http",
    "status": "active",
    "repository": "https://github.com/MeoProject/lx-music-api-server",
    "added_at": "2026-07-17",
    "notes": "这是服务端而非脚本。如果本项目要对接 LX Music 源，可用这个作为参考实现，或直接复用其 API 协议。协议端点: /musicUrl, /lyric, /pic, /search。"
  },
  {
    "id": "lx-source-go",
    "name": "LX Music 自定义解析源（Go）",
    "description": "LX Music 自定义源的服务端 Go 实现，支持缓存、多工作模式",
    "version": "latest",
    "author": "ZxwyWebSite",
    "api_type": "generic-http",
    "status": "active",
    "repository": "https://github.com/832570/lx-source",
    "added_at": "2026-07-17",
    "notes": "Go 语言实现，比 Python 版性能更好。内置多源支持，支持本地缓存。相同协议端点。"
  }
]"#;
