<script setup lang="ts">
/**
 * 组件: PlayerBar.vue
 * 功能: 独立播放条窗口 — 置顶悬浮
 * 窗口: 无边框，480×120px，底部居中置顶
 * 通信: listen('play_state') 接收主窗口推送
 * 拖动: data-tauri-drag-region（标题栏区域）
 */
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { listen, emit as emitEvent, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { parseLrc, findCurrentLine, fetchLyric, type LyricLine } from '../composables/useLyric';

interface Song {
  song_id: string;
  name: string;
  singer: string;
  album: string;
  duration: number;
  source: string;
  cover_url?: string;
}

interface PlayState {
  song: Song | null;
  url: string | null;
  is_playing: boolean;
  position: number;
  duration: number;
}

const currentSong = ref<Song | null>(null);
const audioUrl = ref('');
const isPlaying = ref(false);
const position = ref(0);   // 秒
const duration = ref(0);   // 秒
const volume = ref(0.8);

const audio = ref<HTMLAudioElement | null>(null);
let unlisten: UnlistenFn | null = null;

// ── 歌词状态 ─────────────────────────────────────
const lyricLines = ref<LyricLine[]>([]);
const lyricRaw = ref('');
const currentLineIdx = ref(-1);
const showLyric = ref(false);
let lyricLoading = false;

// 歌词容器（滚动用）
const lyricScroller = ref<HTMLElement | null>(null);

// 格式化时间 mm:ss
function fmt(s: number) {
  const sec = Math.floor(s);
  return `${Math.floor(sec / 60)}:${String(sec % 60).padStart(2, '0')}`;
}

const posStr = computed(() => fmt(position.value));
const durStr = computed(() => fmt(duration.value));

// 进度百分比
const progress = computed(() =>
  duration.value > 0 ? (position.value / duration.value) * 100 : 0
);

// 点击进度条
function seek(e: MouseEvent) {
  const bar = e.currentTarget as HTMLElement;
  const rect = bar.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  if (audio.value && duration.value > 0) {
    audio.value.currentTime = ratio * duration.value;
  }
}

// 播放/暂停
function togglePlay() {
  if (!audio.value) return;
  if (isPlaying.value) {
    audio.value.pause();
  } else {
    audio.value.play();
  }
}

// 上一首/下一首（通知主窗口处理）
async function prev() {
  await emitEvent('player_cmd', 'prev');
}

async function next() {
  await emitEvent('player_cmd', 'next');
}

// 关闭播放条
async function closePlayer() {
  const win = getCurrentWindow();
  await win.close();
}

onMounted(async () => {
  // 创建 audio 元素
  audio.value = new Audio();
  audio.value.volume = volume.value;
  
  // 页面加载时先尝试解锁音频上下文
  audio.value.volume = 0;
  audio.value.play().then(() => {
    audio.value!.pause();
    audio.value!.volume = volume.value;
  }).catch(() => {
    // 第一次失败，监听 canplay 自动续播
    audio.value!.addEventListener('canplay', () => {
      audio.value!.volume = volume.value;
      if (isPlaying.value) {
        audio.value!.play().catch(() => {});
      }
    }, { once: true });
  });

  audio.value.addEventListener('timeupdate', () => {
    position.value = audio.value!.currentTime;
    // 同步歌词高亮
    syncLyric();
    // 同步回主窗口
    emitEvent('player_position', {
      position: audio.value!.currentTime,
      duration: audio.value!.duration || 0,
    });
  });

  // 时长加载事件
  // 音频可播放时自动续播
  audio.value.addEventListener('canplay', () => {
    const a = audio.value!;
    writeDebug(`canplay! isPlaying=${isPlaying.value}, paused=${a.paused}`);
    // 只要用户点了播放，就自动续播
    if (isPlaying.value && a.paused) {
      writeDebug('canplay触发续播!');
      a.play().then(() => {
        writeDebug('续播成功!');
      }).catch((e: any) => {
        writeError(`续播失败: ${e}`);
      });
    }
    // 更新时长
    const d = a.duration;
    if (d && !Number.isNaN(d) && d !== Infinity) {
      duration.value = d;
    }
  });

  audio.value.addEventListener('loadedmetadata', () => {
    const d = audio.value!.duration;
    duration.value = Number.isNaN(d) ? 0 : (d || 0);
    writeDebug(`loadedmetadata! duration=${d}`);
  });
  
  audio.value.addEventListener('durationchange', () => {
    const d = audio.value!.duration;
    if (d && !Number.isNaN(d) && d !== Infinity) {
      duration.value = d;
    }
  });
  
  audio.value.addEventListener('error', (e) => {
    const err = audio.value?.error;
    writeError(`音频错误! code=${err?.code}, msg=${err?.message}`);
    isPlaying.value = false;
  });

  audio.value.addEventListener('ended', () => {
    isPlaying.value = false;
    next();
  });

  audio.value.addEventListener('play', () => { isPlaying.value = true; });
  audio.value.addEventListener('pause', () => { isPlaying.value = false; });

    // 监听主窗口推送的播放状态
  unlisten = await listen<PlayState>('play_state', (ev) => {
    const s = ev.payload;
    console.warn('[PlayerBar] 收到 play_state:', s?.song?.name, 'url长度=', s?.url?.length);
    
    // 强制更新：只要有 song 就更新（不管 URL 是否变化，因为可能是同一首歌重新播放）
    if (s.song) {
      currentSong.value = s.song;
      // 换歌：加载歌词
      loadLyric(s.song);
    }
    
    if (s.url) {
      // 加时间戳防止缓存
      const urlWithTs = s.url + (s.url.includes('?') ? '&' : '?') + '_t=' + Date.now();
      writeDebug(`设置URL: ${urlWithTs}`);
      audioUrl.value = urlWithTs;
      
      // 先暂停当前播放
      audio.value!.pause();
      audio.value!.currentTime = 0;
      
      // 设置新URL并强制加载
      audio.value!.src = urlWithTs;
      audio.value!.load();
      
      // 重置状态
      position.value = 0;
      duration.value = 0;
      
      // 记录收到的状态
      writeDebug(`listen: is_playing=${s.is_playing}, url=${s.url?.substring(0,50)}, song=${s.song?.name}`);
      
      if (s.is_playing) {
        isPlaying.value = true;
        audio.value!.play().then(() => {
          writeDebug('播放成功!');
        }).catch(() => {
          // 被拦截没关系，等 canplay 自动续播
        });
      }
    } else if (s.is_playing !== undefined) {
      if (s.is_playing) audio.value!.play();
      else audio.value!.pause();
    }
  });
});

// ── 歌词加载 ─────────────────────────────────────

async function loadLyric(song: Song) {
  lyricLoading = true;
  lyricLines.value = [];
  lyricRaw.value = '';
  currentLineIdx.value = -1;
  try {
    const lrc = await fetchLyric({
      name: song.name,
      singer: song.singer,
      duration: song.duration,
      source: song.source,
      song_id: song.song_id,
    });
    if (lrc) {
      lyricRaw.value = lrc;
      lyricLines.value = parseLrc(lrc);
      showLyric.value = lyricLines.value.length > 0;
      writeDebug(`歌词加载: ${lyricLines.value.length} 行`);
    } else {
      showLyric.value = false;
      writeDebug('无歌词');
    }
  } catch (e) {
    showLyric.value = false;
    writeDebug('歌词加载失败: ' + String(e));
  } finally {
    lyricLoading = false;
  }
}

// 切换歌词显示
async function toggleLyric() {
  showLyric.value = !showLyric.value;
  try {
    await invoke('resize_player', { tall: showLyric.value });
  } catch (e) {
    console.warn('[PlayerBar] 调整窗口高度失败:', e);
  }
}

// 点击某行歌词跳转
function seekToLine(line: LyricLine) {
  if (audio.value) {
    audio.value.currentTime = line.time;
  }
}

// ── 歌词同步 ─────────────────────────────────────

function syncLyric() {
  if (!audio.value || lyricLines.value.length === 0) return;
  const idx = findCurrentLine(lyricLines.value, audio.value.currentTime);
  if (idx !== currentLineIdx.value) {
    currentLineIdx.value = idx;
    // 滚动到当前行
    scrollLyricToCurrent();
  }
}

function scrollLyricToCurrent() {
  const el = lyricScroller.value;
  if (!el) return;
  const active = el.querySelector('.lyric-line.active') as HTMLElement | null;
  if (active) {
    const containerH = el.clientHeight;
    const lineH = active.offsetHeight || 28;
    const target = active.offsetTop - containerH / 2 + lineH / 2;
    el.scrollTo({ top: target, behavior: 'smooth' });
  }
}

onUnmounted(() => {
  unlisten?.();
  audio.value?.pause();
  audio.value!.src = '';
});

// ── 拖拽区域递归绑定 ───────────────────────────
// data-tauri-drag-region 不会自动传播到子元素，需递归给所有非交互子元素添加
const NO_DRAG_SELECTOR = '.no-drag, button, .progress-bar, .lyric-panel';

function applyDragRegion(el: HTMLElement) {
  if (el.matches?.(NO_DRAG_SELECTOR)) return;
  el.setAttribute('data-tauri-drag-region', '');
  Array.from(el.children).forEach((child) => applyDragRegion(child as HTMLElement));
}

onMounted(() => {
  // 递归给顶部 drag-area 的所有子元素绑定拖拽（跳过按钮/进度条/歌词）
  const dragArea = document.querySelector('.drag-area') as HTMLElement | null;
  if (dragArea) applyDragRegion(dragArea);
});

// 复制调试信息
function writeDebug(msg: string) {
  invoke('debug_log_write', { level: 'info', tag: 'player_bar', msg }).catch(() => {});
}

function writeError(msg: string) {
  invoke('debug_log_write', { level: 'error', tag: 'player_bar', msg }).catch(() => {});
}
</script>

<template>
  <div class="player-bar">
    <!-- 顶部可拖动区（封面+歌名+进度条整行） -->
    <div class="drag-area" data-tauri-drag-region>
      <!-- 封面+歌名 -->
      <div class="song-row">
        <div class="cover" :style="currentSong?.cover_url ? `background-image:url(${currentSong.cover_url})` : ''">
          🎵
        </div>
        <div class="song-info">
          <div class="song-name">{{ currentSong?.name || '未播放' }}</div>
          <div class="song-singer">{{ currentSong?.singer || '' }}</div>
        </div>
      </div>

      <!-- 进度条 -->
      <div class="progress-area">
        <div class="time left">{{ posStr }}</div>
        <div class="progress-bar" @mousedown.stop @click.stop="seek">
          <div class="progress-fill" :style="{ width: progress + '%' }" />
        </div>
        <div class="time right">{{ durStr }}</div>
      </div>
    </div>

    <!-- 歌词区（在控制按钮上方展开） -->
    <div v-if="showLyric" class="lyric-panel no-drag" ref="lyricScroller">
      <p v-if="lyricLines.length === 0" class="lyric-empty">暂无歌词</p>
      <template v-else>
        <div
          v-for="(line, i) in lyricLines"
          :key="i"
          class="lyric-line"
          :class="{ active: i === currentLineIdx }"
          @click="seekToLine(line)"
        >
          {{ line.text }}
        </div>
      </template>
    </div>

    <!-- 右侧：控制按钮（固定在底部） -->
    <div class="controls no-drag">
      <button @click="prev" title="上一首">⏮</button>
      <button class="play-btn" @click="togglePlay" title="播放/暂停">
        {{ isPlaying ? '⏸' : '▶' }}
      </button>
      <button @click="next" title="下一首">⏭</button>
      <!-- 歌词开关 -->
      <button class="lyric-btn" :class="{ on: showLyric }" @click="toggleLyric" title="歌词">🎤</button>
      <!-- 关闭按钮 -->
      <button class="close-btn" @click="closePlayer" title="关闭播放条">✕</button>
    </div>
  </div>
</template>

<style scoped>
.player-bar {
  width: 100%;
  height: 100%;
  background: rgba(30, 30, 50, 0.95);
  backdrop-filter: blur(12px);
  border-radius: 12px;
  border: 1px solid rgba(255,255,255,0.1);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: white;
  font-family: 'Segoe UI', system-ui, sans-serif;
  user-select: none;
}

/* 拖动区（整个顶部：封面+歌名+进度条） */
.drag-area {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px 4px;
  cursor: grab;
  flex-shrink: 0;
}
.drag-area:active { cursor: grabbing; }

.song-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.cover {
  width: 36px;
  height: 36px;
  border-radius: 6px;
  background: rgba(255,255,255,0.1);
  background-size: cover;
  background-position: center;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.song-info {
  min-width: 0;
}
.song-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
}
.song-singer {
  font-size: 11px;
  color: rgba(255,255,255,0.6);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
}

/* 进度条 */
.progress-area {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  flex-shrink: 0;
}
.time {
  font-size: 10px;
  color: rgba(255,255,255,0.5);
  flex-shrink: 0;
  min-width: 28px;
}
.time.left { text-align: right; }

.progress-bar {
  flex: 1;
  height: 3px;
  background: rgba(255,255,255,0.15);
  border-radius: 2px;
  cursor: pointer;
  position: relative;
}
.progress-bar:hover { height: 5px; }
.progress-fill {
  height: 100%;
  background: #7c6af7;
  border-radius: 2px;
  transition: width 0.1s linear;
}

/* 控制按钮 */
.controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 4px 12px 8px;
  flex-shrink: 0;
}

button {
  background: none;
  border: none;
  color: rgba(255,255,255,0.8);
  font-size: 14px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  transition: all 0.15s;
}
button:hover {
  background: rgba(255,255,255,0.1);
  color: white;
}

.play-btn {
  font-size: 18px;
  color: white;
}

.close-btn {
  color: rgba(255,255,255,0.4);
  font-size: 12px;
}
.close-btn:hover {
  color: #ff6b6b;
  background: rgba(255,100,100,0.1);
}

/* ── 歌词按钮 ────────────────────────────────── */
.lyric-btn {
  color: rgba(255,255,255,0.5);
  font-size: 14px;
  transition: color 0.15s;
}
.lyric-btn.on {
  color: #7c6af7;
}

/* ── 歌词面板 ────────────────────────────────── */
.lyric-panel {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 16px;
  background: rgba(0,0,0,0.25);
  border-top: 1px solid rgba(255,255,255,0.06);
  scrollbar-width: thin;
  scrollbar-color: rgba(255,255,255,0.15) transparent;
}
.lyric-panel::-webkit-scrollbar {
  width: 4px;
}
.lyric-panel::-webkit-scrollbar-thumb {
  background: rgba(255,255,255,0.15);
  border-radius: 2px;
}
.lyric-line {
  font-size: 13px;
  line-height: 2.1;
  color: rgba(255,255,255,0.45);
  cursor: pointer;
  text-align: center;
  padding: 1px 0;
  transition: color 0.2s, transform 0.2s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.lyric-line:hover {
  color: rgba(255,255,255,0.75);
}
.lyric-line.active {
  color: #7c6af7;
  font-weight: 600;
  font-size: 14px;
  transform: scale(1.02);
}
.lyric-empty {
  text-align: center;
  color: rgba(255,255,255,0.3);
  font-size: 12px;
  margin: 20px 0;
}
</style>
