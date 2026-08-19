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

interface Song {
  song_id: string;
  name: string;
  singer: string;
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

// 窗口拖动
async function startDrag(e: MouseEvent) {
  // 如果点击的是按钮等交互元素，不触发拖动
  const target = e.target as HTMLElement;
  if (target.closest('.no-drag') || target.closest('button') || target.closest('.progress-bar')) {
    return;
  }
  const win = getCurrentWindow();
  await win.startDragging();
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

onUnmounted(() => {
  unlisten?.();
  audio.value?.pause();
  audio.value!.src = '';
});

// 复制调试信息
function copyDebug() {
  navigator.clipboard.writeText(debugInfo.value).catch(() => {});
}

function writeDebug(msg: string) {
  // 同时显示在界面上和写到日志文件
  debugInfo.value = msg;
  invoke('debug_log_write', { level: 'info', tag: 'player_bar', msg }).catch(() => {});
}

function writeError(msg: string) {
  debugInfo.value = msg;
  invoke('debug_log_write', { level: 'error', tag: 'player_bar', msg }).catch(() => {});
}

const debugInfo = ref('等待音频...');
const showDebug = ref(true);

// 每秒更新一次调试信息
let debugInterval: number | null = null;
onMounted(() => {
  debugInterval = window.setInterval(() => {
    if (!audio.value) {
      writeDebug('audio=null');
      return;
    }
    const a = audio.value;
    const states = ['HAVE_NOTHING', 'HAVE_METADATA', 'HAVE_CURRENT_DATA', 'HAVE_FUTURE_DATA', 'HAVE_ENOUGH_DATA'];
    writeDebug(`readyState=${states[a.readyState]||a.readyState}, paused=${a.paused}, duration=${a.duration}, currentTime=${a.currentTime.toFixed(1)}`);
  }, 2000);
});
onUnmounted(() => {
  if (debugInterval) clearInterval(debugInterval);
});
</script>

<template>
  <div class="player-bar" @mousedown="startDrag">
    <!-- 拖动区：左侧封面+歌名 -->
    <div class="drag-area" data-tauri-drag-region>
      <div class="cover" :style="currentSong?.cover_url ? `background-image:url(${currentSong.cover_url})` : ''">
        🎵
      </div>
      <div class="song-info">
        <div class="song-name">{{ currentSong?.name || '未播放' }}</div>
        <div class="song-singer">{{ currentSong?.singer || '' }}</div>
      </div>
    </div>

    <!-- 中间：进度条 -->
    <div class="progress-area">
      <div class="time left">{{ posStr }}</div>
      <div class="progress-bar" @click="seek">
        <div class="progress-fill" :style="{ width: progress + '%' }" />
      </div>
      <div class="time right">{{ durStr }}</div>
    </div>

    <!-- 右侧：控制按钮 -->
    <div class="controls no-drag">
      <button @click="prev" title="上一首">⏮</button>
      <button class="play-btn" @click="togglePlay" title="播放/暂停">
        {{ isPlaying ? '⏸' : '▶' }}
      </button>
      <button @click="next" title="下一首">⏭</button>
      <!-- 关闭按钮 -->
      <button class="close-btn" @click="closePlayer" title="关闭播放条">✕</button>
    </div>
    
    <!-- 调试信息面板 -->
    <div v-if="showDebug" class="debug-panel no-drag">
      <div class="debug-text">{{ debugInfo }}</div>
      <div class="debug-hint">
        <button class="copy-btn" @click="copyDebug">复制</button>
        音频状态监控
      </div>
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

/* 拖动区 */
.drag-area {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px 4px;
  cursor: grab;
  flex-shrink: 0;
}
.drag-area:active { cursor: grabbing; }

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

/* 调试面板 - 放顶部更明显 */
.debug-panel {
  position: absolute;
  top: 2px;
  left: 2px;
  right: 2px;
  background: rgba(0,0,0,0.9);
  border: 2px solid #ff6b6b;
  border-radius: 4px;
  padding: 6px 8px;
  font-size: 10px;
  font-family: monospace;
  color: #ff6b6b;
  z-index: 9999;
  cursor: pointer;
}
.debug-text {
  line-height: 1.4;
}
.debug-hint {
  font-size: 8px;
  color: rgba(255,255,255,0.4);
  text-align: center;
  margin-top: 2px;
}
</style>
