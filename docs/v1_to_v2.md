# 米豆音乐 v2.0 — 从 v1 到 v2 转型说明

> 2026-07-27 13:19

## 为什么重开 v2.0

v1.0 的核心架构矛盾：

> **音频在浏览器的 WebView 里播放，但浏览器禁止自动播放**

这导致了一连串永远修不完的 bug：
- 首次点击歌曲必须点两次（浏览器音频上下文未解锁）
- 独立播放条窗口没有用户手势，textdownload 策略更严
- 切歌时 `audio.src` 换了但 `load()` 没生效（浏览器缓存/timing 问题）
- 调试面板、事件日志、多事件兜底全部加上还是不稳定

**根本原因**：把音频引擎放在浏览器沙箱里，等于让客人管着主厨的灶台。

v2.0 把音频搬到 Rust 里（rodio 原生解码+输出），浏览器只负责界面渲染。

## 技术栈对比

| 维度 | v1.0 | v2.0 |
|------|------|------|
| 前端框架 | Vue 3 | **React 19** |
| CSS 方案 | Tailwind/Naive 写了没装 | **Tailwind CSS 4 + Lucide** |
| 音频引擎 | WebView `<audio>` | **rodio（Rust 原生）** |
| 自动播放 | 浏览器拦截 | **无限制** |
| 独立播放窗口 | 依赖 Tauri WebviewWindow 通信 | **同一套机制，但音频不经过浏览器** |
| 音频解码 | 浏览器内置 codec | **rodio 后端（Windows = WASAPI）** |
| 播放状态获取 | Web Audio API（timing 不可靠） | **IPC 轮询（500ms，数据来自 Rust）** |
| 技术栈一致性 | 和书签程序不同（Vue vs React） | **和书签程序统一（都是 Tauri + React）** |

## Rust 后端复用

从 v1 直接搬了以下模块（一字不改）：

```
✅ commands/search.rs      — 三平台搜索
✅ commands/play.rs        — play_url 按前缀路由
✅ commands/window.rs      — 独立播放窗口
✅ commands/kugou_login.rs — 酷狗扫码登录
✅ platform/kuwo.rs        — 酷我适配器（搜索+播放URL）
✅ platform/bili.rs        — B站适配器
✅ platform/kugou.rs       — 酷狗适配器
✅ db.rs                   — SQLite 数据库
✅ debug_log.rs            — 文件日志
✅ types.rs                — 数据结构（Song/AppState 等）
```

新增唯一的新模块：

```
⭐ audio.rs               — rodio 音频引擎（通道模式，专用线程）
```

## rodio 音频架构

```
前端 React                    Rust 后端
──────────                    ─────────
invoke('audio_play', url)
        ↓
  lib.rs: audio_play()
    ├─ reqwest 下载音频数据（async）
    └─ AudioHandle.play(data)  ← 通道发送
           ↓
     [midou-audio 线程]         ← 专用线程（不被 Send 限制）
       ├─ rodio::Decoder  解码
       ├─ rodio::Sink     控制播放
       └─ rodio::OutputStream → WASAPI → 扬声器
```

- **AudioHandle**：Send + Sync，可以在 Tauri Command 和 AppState 里自由传递
- **专用线程**：持有 `OutputStream`（非 Send），通过 `mpsc` 通道接收命令
- **命令**：Play/Pause/Resume/Stop/SetVolume/GetState

## 前端文件清单

| 文件 | 说明 |
|------|------|
| `App.tsx` | 三栏布局 + 搜索 + 播放 + 状态栏 |
| `components/Sidebar.tsx` | 5 音源切换（酷我/B站/酷狗/QQ/咪咕/本地） |
| `components/SearchBar.tsx` | 关键词搜索 + 按键 Enter 触发 |
| `components/SongList.tsx` | 歌曲列表 + 双击/单击播放 |
| `components/PlayerBar.tsx` | 独立播放窗口（进度条 + 播放/暂停 + 时长） |
| `components/ui/button.tsx` | shadcn 风格按钮 |
| `components/ui/input.tsx` | 输入框 + 下拉框 |

## v1.0 处理建议

v1.0 (`D:\workspaces\search\projects\midou-music`) 保留不动：
- 可以作为"音源适配器"的参考实现（Rust 端全搬过来了）
- `legacy` 分支上有更早的 `music-app` 存档
- v1 的前端（Vue）不再维护

## 下一步

- [ ] 实际试听（酷我/B站/酷狗三平台逐首播放验证）
- [ ] 酷狗扫码登录在 v2 前端对接
- [ ] 播放列表管理（下一首/上一首）
- [ ] 频道搜索优化（分页/防抖）
- [ ] 清理 6 个 dead_code warnings（v1 遗留，不影响功能）
- [ ] 复制 v1 的 icons 到 v2
