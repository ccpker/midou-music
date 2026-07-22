<template>
  <div class="player-bar">
    <!-- Track info -->
    <div class="player-info">
      <span class="player-title">{{ store.nowPlaying?.name || '未播放' }}</span>
      <span class="player-artist">{{ store.nowPlaying?.artist || '' }}</span>
    </div>

    <!-- Playback controls -->
    <div class="player-controls">
      <button title="上一首" @click="prev">⏮</button>
      <button class="play-btn" title="播放/暂停" @click="store.togglePlay()">
        {{ store.isPlaying ? '⏸' : '▶' }}
      </button>
      <button title="下一首" @click="next">⏭</button>
      <button title="歌词" @click="openLyrics">词</button>
    </div>

    <!-- Extra actions -->
    <div class="player-extra">
      <!-- B站视频/音频切换 -->
      <button
        v-if="store.nowPlaying?.source === 'bilibili'"
        class="bili-video-btn"
        :class="{ active: store.biliVideoMode }"
        :title="store.biliVideoMode ? '切换音频模式' : '切换视频模式'"
        @click="store.toggleBiliVideoMode()"
      >
        {{ store.biliVideoMode ? '🎵' : '📺' }}
      </button>
      <button title="下载" :disabled="!store.nowPlaying" @click="downloadAndCollect">⬇</button>
      <button
        title="收藏"
        :disabled="!store.nowPlaying || isCurrentFaved"
        :class="{ 'faved': isCurrentFaved }"
        @click="downloadAndCollect"
      >
        {{ isCurrentFaved ? '♥' : '♡' }}
      </button>
    </div>
  </div>
</template>

<script setup>
import { computed, watch } from 'vue'
import { usePlayerStore } from '@/stores/player'
import { useLibraryStore } from '@/stores/library'

const store = usePlayerStore()
const libStore = useLibraryStore()

// ── U1: 收藏状态（跟随 nowPlaying 变化） ──
const currentSource = computed(() => store.nowPlaying?.source || store.currentSource)
const currentSourceId = computed(() => {
  if (!store.nowPlaying) return ''
  return store.nowPlaying.song_id || store.nowPlaying.id || ''
})
const isCurrentFaved = computed(() => {
  if (!store.nowPlaying) return false
  return libStore.isSongCollected(currentSource.value, currentSourceId.value)
})

// 切换到播放中的歌曲时刷新收藏状态
watch(() => store.nowPlaying?.song_id, () => {
  if (store.nowPlaying) {
    libStore.refreshActiveLibrary()
  }
})

async function openLyrics() {
  if (!store.nowPlaying) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('open_lyrics_window_cmd')
  } catch (e) {
    window.open('http://127.0.0.1:8899/lyrics-view', 'lyrics', 'width=420,height=700')
  }
}

function prev() {
  // placeholder
}

function next() {
  if (store.queue.length > 0) {
    const next = store.queue.shift()
    store.playSong(next)
  }
}

// ── U1: 收藏下载 — download → addSongToLibrary ──
async function downloadAndCollect() {
  if (!store.nowPlaying) return
  if (isCurrentFaved.value) return // 已收藏

  const song = store.nowPlaying
  const rid = song.song_id || song.id
  if (!rid) {
    alert('无法获取歌曲 ID')
    return
  }

  const singer = song.artist || song.singer || ''
  const name = song.name || song.title || ''
  const src = song.source || 'kuwo'

  // P10: 按音源路由到不同下载API
  const downloadUrl = src === 'kugou'
    ? `/api/download/kugou/${encodeURIComponent(rid)}?name=${encodeURIComponent(name)}&singer=${encodeURIComponent(singer)}&category=${encodeURIComponent('默认')}`
    : `/api/download/kuwo/${encodeURIComponent(rid)}?name=${encodeURIComponent(name)}&singer=${encodeURIComponent(singer)}&category=${encodeURIComponent('默认')}`

  try {
    // 1. 调用下载 API
    const resp = await fetch(downloadUrl)
    const data = await resp.json()

    if (data.error) {
      alert('下载失败: ' + data.error)
      return
    }

    console.log('[download] 已保存:', data.filename, 'source:', src)

    // 2. 入库
    if (data.path && data.filename) {
      const fileSize = data.file_size || 0
      const sourceId = rid
      const title = name
      const artist = singer

      // file_path 用文件名（库根目录相对路径）
      await libStore.addSongToLibrary(title, artist, src, sourceId, data.filename, fileSize)
      console.log(`[PlayerBar] 收藏入库: ${title} — ${artist}`)
    }
  } catch (err) {
    alert('操作失败: ' + err.message)
  }
}
</script>

<style scoped>
.player-bar {
  position: fixed;
  bottom: 0;
  left: 80px;
  right: 0;
  height: 60px;
  background: var(--card);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 20px;
  gap: 20px;
  z-index: 100;
}

.player-info {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.player-title {
  display: block;
  font-weight: 500;
  margin-bottom: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.player-artist {
  font-size: 11px;
  color: var(--muted);
}

.player-controls {
  display: flex;
  gap: 10px;
  align-items: center;
}

.player-controls button {
  width: 34px;
  height: 34px;
  border: none;
  border-radius: 50%;
  background: var(--bg);
  color: var(--text);
  cursor: pointer;
  font-size: 14px;
  transition: background 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.player-controls button:hover {
  background: var(--accent);
}

.play-btn {
  width: 38px !important;
  height: 38px !important;
  font-size: 16px !important;
}

.player-extra {
  display: flex;
  gap: 8px;
}

.player-extra button {
  padding: 6px 12px;
  border: none;
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text);
  cursor: pointer;
  font-size: 14px;
  transition: background 0.2s;
}

.player-extra button:hover {
  background: var(--accent);
}

.faved {
  color: var(--accent) !important;
}

.bili-video-btn {
  font-size: 16px !important;
  padding: 4px 10px !important;
  border-radius: 6px !important;
  background: var(--bg) !important;
  transition: all 0.2s !important;
}

.bili-video-btn.active {
  background: #FF6B9D !important;
  color: #fff !important;
}

.bili-video-btn.active:hover {
  background: #e05587 !important;
}
</style>
