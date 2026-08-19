# 米豆音乐 — SPEC.md
> 功能规范 v1.0（2026-07-22 16:00）
> 充分调研后的最小功能集，每个模块独立可复用

---

## 一、技术基础

### 数据库
- **选型**：SQLite（`rusqlite` crate）
- **路径**：`{app_data}/midou-music.db`
- **表结构**：

```sql
-- 网络收藏
CREATE TABLE favorites_net (
    id          INTEGER PRIMARY KEY,
    song_id     TEXT NOT NULL,
    source      TEXT NOT NULL,  -- 'kuwo' | 'bilibili' | 'kugou' | 'qq'
    song_json   TEXT NOT NULL,  -- 完整歌曲对象 JSON（title/artist/album/duration等）
    added_at    INTEGER NOT NULL  -- Unix timestamp
);

-- 分类（网络/本地通用）
CREATE TABLE categories (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL,  -- 'net' | 'local'
    source      TEXT,           -- 网络分类时：'kuwo'|'bilibili'|... | NULL=通用
    created_at  INTEGER NOT NULL
);

-- 歌曲-分类关联（多对多）
CREATE TABLE song_categories (
    song_id     TEXT NOT NULL,
    source      TEXT NOT NULL,  -- 'net:{source}' | 'local'
    category_id INTEGER NOT NULL,
    PRIMARY KEY (song_id, source, category_id)
);

-- 本地歌曲
CREATE TABLE favorites_local (
    id          INTEGER PRIMARY KEY,
    file_path   TEXT NOT NULL UNIQUE,
    title       TEXT,
    artist      TEXT,
    album       TEXT,
    duration    REAL,
    lrc_path    TEXT,           -- 关联歌词文件路径
    lrc_offset  INTEGER DEFAULT 0,  -- 歌词偏移（毫秒）
    added_at    INTEGER NOT NULL
);

-- 用户设置
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 下载文件夹结构
```
{dirs::audio_dir()}/midou-music/
├── songs/           -- 下载的音乐文件
│   ├── 晴天 - 周杰伦.mp3
│   └── 稻香 - 周杰伦.flac
├── lyrics/          -- 手动保存/编辑的歌词
│   ├── 晴天 - 周杰伦.lrc
│   └── 稻香 - 周杰伦.lrc
└── lrc-backup/      -- LRCLIB 下载归档（不同步，用作其他播放器共享）
    └── 晴天 - 周杰伦.lrc
```

---

## 二、lib.rs Command 完整清单

### 已实现 ✅（2026-07-25 更新）
| command | 签名 | 说明 |
|---------|------|------|
| `search` | `(keyword, source, page) → Vec<Song>` | 搜索，source='kuwo'/'bilibili'/'kugou' |
| `play_url` | `(song_id, source) → String` | 获取播放 URL |
| `kugou_qr_key` | `() → {qrcode_key, qrcode_img}` | **生成扫码登录二维码** |
| `kugou_qr_check` | `(key) → {status, token, userid}` | 轮询扫码状态 |
| `kugou_auth_status` | `() → {logged_in, userid, nickname, vip}` | 查询登录状态 |
| `kugou_save_qr_token` | `(token, userid) → ()` | 保存扫码 token 到 DB |
| `kugou_logout` | `() → ()` | 退出登录 |

### MVP 新增 🟡

| command | 签名 | 说明 |
|---------|------|------|
| `lyric` | `(song_id, source) → LyricResult` | 获取歌词（优先本地→LRCLIB→源API） |
| `download` | `(song, dest_path) → String` | 下载音频，返回本地路径，自动扫描入库 |
| `scan_library` | `(folder) → Vec<Song>` | 扫描文件夹，解析 ID3，返回歌曲列表 |
| `favorite_add` | `(song, source, category_id) → ()` | 添加收藏（song=JSON字符串） |
| `favorite_remove` | `(song_id, source) → ()` | 移除收藏 |
| `favorite_list` | `(source, category_id) → Vec<Song>` | 获取收藏列表 |
| `category_list` | `(type) → Vec<Category>` | 获取分类列表，type='net'/'local' |
| `category_create` | `(name, type, source) → Category` | 创建分类 |
| `category_delete` | `(id) → ()` | 删除分类（歌曲不移除） |
| `category_add_song` | `(song_id, source, category_id) → ()` | 将歌曲加入分类 |
| `category_remove_song` | `(song_id, source, category_id) → ()` | 从分类移除歌曲 |
| `local_song_info` | `(file_path) → Song` | 获取本地文件信息（ID3） |
| `lrc_load` | `(file_path) → LrcDoc` | 加载 LRC 文件 |
| `lrc_save` | `(file_path, lrc_text) → ()` | 保存 LRC 文件（编辑后） |
| `lrc_match` | `(song_id) → Option<String>` | 查找本地歌词（同名.lrc） |
| `get_setting` | `(key) → Option<String>` | 读取设置 |
| `set_setting` | `(key, value) → ()` | 写入设置 |

---

## 三、前端组件

### App.vue（主窗口）
- 左侧边栏（固定宽度 200px）
- 右侧主面板（flex: 1）
- 底部留空（播放条独立窗口）

### Sidebar.vue
- 展开/折叠逻辑（酷我/B站等可展开二级）
- 右键菜单（自定义右键组件，非系统菜单）
- 激活状态高亮

### ContentPanel.vue
- 根据侧边栏选择，切换不同子面板：
  - `SearchPanel` — 搜索结果
  - `SongDetailPanel` — 歌单/专辑详情
  - `FavoritesPanel` — 收藏列表
  - `LocalPanel` — 本地收藏
  - `SettingsPanel` — 设置

### SongList.vue
- 歌曲列表（封面/歌名/歌手/时长/操作按钮）
- 操作按钮：▶ 播放 / 💚 收藏 / 📥 下载 / ⋯ 更多

### PlayerBar.vue（独立窗口）
- **窗口尺寸**：480 × 120 px
- **默认位置**：屏幕底部居中，距底部 20px
- **置顶**：Always on top
- **内容**：封面缩略图 + 歌名/歌手 + 进度条 + 控制按钮
- **拖动**：仅标题栏可拖

### LyricsWindow.vue
- **展开方式**：从播放条上沿向下展开（高度从 0 扩展到 ~300px）
- **歌词面板**：垂直滚动，自动高亮当前行
- **编辑按钮**：点编辑图标 → 弹出歌词编辑器

### LyricsEditor.vue（独立窗口）
- **窗口尺寸**：500 × 600 px
- **内容**：
  - 偏移量：[-0.3s] [-0.1s] [+0.1s] [+0.3s] 快捷按钮
  - 当前偏移显示（毫秒）
  - 歌词行列表（时间 + 文字，可点击编辑）
  - 播放/暂停按钮（辅助对齐）
  - 保存按钮

### ContextMenu.vue
- 自定义右键菜单（非系统原生）
- 支持菜单项：图标 + 文字 + 分隔线

---

## 四、模块独立性原则

每个 Rust 模块对外只暴露 Tauri Command，**不直接依赖前端**。

```
lyrics_editor.rs   → 纯 LRC 解析/保存逻辑，零前端依赖
library.rs         → 数据库 CRUD，零前端依赖
kuwo.rs            → 纯 API 调用，零前端依赖
```

前端通过 `invoke()` 调用，不感知实现细节。

---

## 五、MVP 阶段划分（2026-07-25 更新）

> ✅ = 已完成   🎯 = 进行中   📋 = 待做

| 阶段 | 内容 | 状态 |
|------|------|------|
| **Phase MVP** | 酷我搜索+播放 ✅ | ✅ 完成 |
| | 播放条独立窗口 ✅ | ✅ 完成 |
| | 歌词显示（LRCLIB）| 🎯 进行中 |
| | 收藏+分类（SQLite）| 📋 待做 |
| | 侧边栏完整结构 | 🎯 进行中 |
| **Phase 2** | B站音频接入 ✅ | ✅ 超前完成 |
| | 酷狗扫码登录 ✅ | ✅ 超前完成 |
| | 酷狗播放 | 🎯 进行中（链路待验证）|
| | 下载+自动入库 | 📋 待做 |
| | 本地+ID3扫描 | 📋 待做 |
| **Phase 3** | 歌词独立窗口 | 📋 待做 |
| | 歌词编辑器 | 📋 待做 |
| | 主题切换 | 📋 待做 |
| | 多源收藏聚合 | 📋 待做 |

---

## 六、调研结论汇总

| 问题 | 结论 |
|------|------|
| LRC 解析 | ✅ 纯正则 `[mm:ss.xx]`，零依赖 |
| LRC 保存 | ✅ `std::fs::write`，不依赖网络 |
| 偏移量 | ✅ 内存计算 + DB `lrc_offset` 字段 |
| LRCLIB 回传 | ❌ 服务端无 POST 接口 → 改为本地 `lrc-backup/` |
| 数据库 | ✅ SQLite |
| 扫描音频 | ✅ `id3` crate 解析 ID3 |
| 窗口通信 | ✅ Tauri IPC（emit + listen） |
| 右键菜单 | ✅ Vue 自定义组件，非系统菜单 |
