# 架构地图 v0.4

> 生成时间: 2026-07-23 07:57
> 原则: 最小模块 + 显式标注，出问题能快速定位

---

## 大事记

- **v0.4** (2026-07-23): ✅ 新增酷狗适配器（含登录/搜索/播放）；网络收藏取消独立一级，改为各平台二级菜单；Sidebar 改为可展开结构
- **v0.3** (2026-07-22): B站音频适配器；多平台路由

---

## Rust 后端

```
src-tauri/src/
│
├── main.rs                          [官方骨架，不碰]
│
├── lib.rs                   32行    [入口编排，只组装模块]
│
├── types.rs                 60行    [全局数据类型]
│   ├── Song         结构体  ← 歌曲（source 字段标平台）
│   ├── PlayUrlResult 结构体 ← play_url 返回值
│   ├── PlayState    结构体  ← emit 到所有窗口
│   ├── AppState     结构体  ← 全局状态（client + db）
│   └── KugouAuth    结构体  ← 酷狗登录凭证 (v0.4新增)
│
├── db.rs                    50行    [数据库初始化]
│       表: favorites_net / categories / song_categories
│             / favorites_local / settings
│             / kugou_auth (v0.4新增)
│
├── commands/
│   ├── mod.rs              5行
│   ├── search.rs           28行    [多平台路由搜索]
│   │       "kuwo" → kuwo::search()
│   │       "bili" → bili::search()
│   │       "kugou" → kugou::search()         ← v0.4
│   │       TODO: "qq"
│   │
│   ├── play.rs             35行    [song_id 前缀路由播放]
│   │       au{MUSIC_ID} → bili::play_url()
│   │       bv{BVID}     → bili::play_url()
│   │       kugou:{hash} → kugou::play_url()  ← v0.4
│   │       纯数字       → kuwo::play_url()
│   │       TODO: qq 前缀
│   │
│   ├── kugou_login.rs       ???行   [酷狗密码登录] ← v0.4 新建
│   │       AES-128-CBC 加密密码
│   │       RSA PKCS1v15 加密 AES Key
│   │       POST /v9/login_by_pwd
│   │       解密响应 → {token, userid, vip_token, vip_type}
│   │       写入 kugou_auth 表 + 更新 AppState
│   │
│   └── window.rs            93行   [播放条窗口管理]
│
└── platform/
    ├── mod.rs              4行    [音源入口]
    │
    ├── kuwo.rs             190行   [酷我适配器] ✅ 稳定
    │
    ├── bili.rs            320行   [B站音频适配器] ✅ v0.2
    │
    └── kugou.rs           ???行   [酷狗适配器] ← v0.4 新建
            search()  → GET /v3/search/song (概念版 appid=3116)
            play_url() → POST /v2/get_res_privilege/lite
            fetch_playlists() → POST /v7/get_all_list
            fetch_tracks()   → GET /pubsongs/v2/get_other_list_file_nofilt
```

---

## Vue 前端

```
src/
│
├── App.vue                   [主窗口编排] ✅
│   ├── SOURCE_MAP: section → platform source 映射
│   └── 导航联动: 切换平台自动重搜
│
├── player_main.ts             纯 bootstrap
│
├── composables/
│   ├── usePlayer.ts          [播放状态 + IPC] ✅
│   ├── useSearch.ts          [多平台搜索] ✅
│   └── useKugou.ts           [酷狗登录] ← v0.4 新建
│       login(username, password)
│       logout()
│
└── components/
    ├── PlayerBar.vue          312行 ✅ 稳定
    ├── Sidebar.vue            [各平台 + 平台内收藏] ← v0.4 架构调整
    ├── SearchBar.vue           72行 ✅ 稳定
    └── SongList.vue           111行 ✅ 稳定
```

---

## song_id 命名规范（v0.4）

| 格式 | 平台 | 来源 |
|---|---|---|
| `123456`（纯数字） | 酷我 | MUSIC_RID |
| `au{MUSIC_ID}` | B站音乐区 | 纯音频 |
| `bv{BVID}` | B站视频区 | 音频版视频 |
| `kugou:{FileHash}` | 酷狗 | 歌曲 Hash |
| `qq:{mid}` | QQ音乐（待接入）| |

---

## 快速定位表（v0.4）

| 问题现象 | 定位文件 |
|---|---|
| 酷我搜索/播放失败 | `platform/kuwo.rs` |
| B站搜索/播放失败 | `platform/bili.rs` |
| 酷狗搜索失败 | `platform/kugou.rs` → `search()` |
| 酷狗播放失败 | `platform/kugou.rs` → `play_url()` |
| 酷狗登录失败 | `commands/kugou_login.rs` |
| 切换平台后不重搜 | `App.vue` → `navigate()` + `SOURCE_MAP` |
| 播放条窗口不弹出 | `commands/window.rs` |
| 路由逻辑找不到平台 | `commands/search.rs` / `commands/play.rs` |

---

## 待接入功能

| 文件 | 内容 | 优先级 |
|---|---|---|
| `commands/kugou_login.rs` | 酷狗密码登录（AES+RSA）| ✅ |
| `platform/kugou.rs` | 酷狗搜索+播放+歌单（MVP 全部）| ⭐⭐⭐ |
| `commands/search.rs` | "kugou" 分支 | ⭐⭐⭐ |
| `commands/play.rs` | kugou 前缀路由 | ⭐⭐⭐ |
| `composables/useKugou.ts` | 前端登录状态 | ⭐⭐⭐ |
| `types.rs` | KugouAuth 结构 | ⭐⭐⭐ |
| `db.rs` | kugou_auth 表 | ⭐⭐⭐ |
| `Cargo.toml` | aes/cbc/rsa/rand 依赖 | ⭐⭐⭐ |
| `Sidebar.vue` | 取消"网络收藏"一级，平台收藏并入平台 | ⭐⭐ |
| `commands/play.rs` | qq 前缀 | ⭐⭐ |
| `composables/usePlayer.ts` | 上一首/下一首队列 | ⭐ |
| `db.rs` | favorites_net/local 增删改查 | ⭐ |
| `platform/bili.rs` | LRCLIB 歌词集成 | ⭐ |
