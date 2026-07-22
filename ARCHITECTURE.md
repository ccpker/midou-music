# 米豆音乐 — ARCHITECTURE.md

> 功能地图 v0.1.1（2026-07-22）
> 修改代码前必须查此地图，确认影响范围。

---

## 整体架构（Mermaid）

```mermaid
graph TD
    subgraph "桌面层 Tauri v2 (官方骨架)"
        MAIN[main.rs<br/>入口: app_lib::run()<br/>官方生成, 不动]
        LIB[lib.rs<br/>#[tauri::command]<br/>search / play_url]
        MW[主窗口<br/>800×600<br/>csp: null]
    end

    subgraph "音源层 platform/"
        KUWO[kuwo.rs<br/>search.kuwo.cn<br/>mobi.kuwo.cn<br/>VIP全通 零Cookie]
    end

    subgraph "前端 Vue 3 (官方 Vite 模板)"
        VAPP[App.vue<br/>搜索+列表+播放器]
        SEARCH_BAR[SearchBar.vue<br/>emit: search]
        SONG_LIST[SongList.vue<br/>props: songs[]<br/>emit: play]
    end

    VAPP -->|invoke('search')| LIB
    VAPP -->|invoke('play_url')| LIB
    LIB --> KUWO
    KUWO -->|reqwest HTTP| KUWO
    MAIN --> LIB
    MAIN -->|create_window| MW
    MW -->|加载| VAPP
    VAPP --> SEARCH_BAR
    VAPP --> SONG_LIST
```

**没有 warp、没有 HTTP 路由、没有 8899 端口。**

---

## 通信方式

```
前端 Vue          Tauri IPC               Rust 后端
─────────         ────────                ─────────
invoke('search',  ───────────►  #[tauri::command]
  { keyword })                  fn search(...)
                  ◄───────────  Vec<Song>

invoke('play_url',──────────►  #[tauri::command]
  { songId })                   fn play_url(...)
                  ◄───────────  { url, source }

<audio src="https://cdn..."  ← 酷我 CDN 直连，不经代理
```

来源：https://v2.tauri.app/develop/calling-rust/

---

## 节点详情

### main.rs
| 字段 | 值 |
|------|-----|
| 路径 | `src-tauri/src/main.rs` |
| 行数 | 10 行 |
| 规则 | 官方原话：don't modify, modify lib.rs instead |
| 依赖 | app_lib |

### lib.rs
| 字段 | 值 |
|------|-----|
| 路径 | `src-tauri/src/lib.rs` |
| 行数 | ~60 行（v0.1.0 仅两个 command） |
| command | `search(keyword) → Vec<Song>` |
| command | `play_url(song_id) → PlayUrlResult` |
| 状态 | AppState { client: reqwest::Client } |
| 依赖 | platform/kuwo.rs |

### kuwo.rs
| 字段 | 值 |
|------|-----|
| 路径 | `src-tauri/src/platform/kuwo.rs` |
| 来源 | 从音楽自由旧项目迁入，代码不变 |
| 端 | `search.kuwo.cn/r.s` — 搜索 |
| 端点 | `mobi.kuwo.cn/mobi.s` — 播放 URL |
| 特性 | 零登录零Cookie，VIP全通 |

### App.vue
| 字段 | 值 |
|------|-----|
| 路径 | `src/App.vue` |
| 功能 | 搜索框 + 歌曲列表 + 底部 `<audio>` 播放器 |
| 通信 | `invoke()` 调 Rust command |

### SearchBar.vue
| 字段 | 值 |
|------|-----|
| 路径 | `src/components/SearchBar.vue` |
| props | 无 |
| emits | `search(keyword)` |

### SongList.vue
| 字段 | 值 |
|------|-----|
| 路径 | `src/components/SongList.vue` |
| props | `songs: Song[]` |
| emits | `play(song)` |

---

## 数据流（一次搜索→播放）

```
用户输入 "晴天"
  → SearchBar emit('search', '晴天')
  → App.vue invoke('search', { keyword: '晴天' })
  → lib.rs #[tauri::command] fn search
  → kuwo.rs::search() → reqwest GET search.kuwo.cn
  → 解析 JSON → Vec<Song>
  → App.vue 更新 songs[]
  → SongList 渲染列表

用户点击第1首
  → SongList emit('play', song)
  → App.vue invoke('play_url', { songId: '123456' })
  → lib.rs #[tauri::command] fn play_url
  → kuwo.rs::play_url() → reqwest GET mobi.kuwo.cn
  → { url: "https://..." }
  → <audio src="https://..."  />  ← CDN直连，不经代理
```

---

## 目录结构（对齐 Tauri 官方 project-structure）

```
midou-music/
├── index.html              ← Tauri 前端入口
├── package.json            ← 前端依赖 (vue, @tauri-apps/api)
├── vite.config.ts          ← Vite 配置
├── tsconfig.json           ← TypeScript 配置
├── app-icon.png            ← 图标源 (1240×1240, 给 tauri icon 用)
│
├── src/                    ← Vue 前端源码
│   ├── main.ts
│   ├── App.vue
│   ├── style.css
│   ├── vite-env.d.ts
│   └── components/
│       ├── SearchBar.vue
│       └── SongList.vue
│
├── src-tauri/              ← Rust 后端
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/              ← tauri icon 生成
│   └── src/
│       ├── main.rs         ← 官方生成，不动
│       ├── lib.rs          ← #[tauri::command] 全部写这里
│       └── platform/
│           ├── mod.rs
│           └── kuwo.rs
│
├── test-pages/             ← 独立测试页面
├── scripts/                ← 构建辅助脚本
├── docs/
│   ├── 编制规则_v1.md
│   └── 研发计划.md
└── ARCHITECTURE.md         ← 本文件
```

---

## 修订记录

| 日期 | 版本 | 说明 |
|------|------|------|
| 2026-07-22 | v0.1.0 | 初始（warp 架构） |
| 2026-07-22 | v0.1.1 | 废弃 warp，改为 Tauri IPC 官方架构 |
