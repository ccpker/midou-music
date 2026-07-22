<template>
  <div class="app">
    <Sidebar />
    <div class="main">
      <SearchBar />
      <SongList />
    </div>
    <PlayerBar />

    <!-- Hidden audio element — controlled via store events -->
    <audio ref="audioEl" @ended="onEnded" @error="onError" />

    <!-- B站视频播放器（浮动在右下角，B站视频模式时显示） -->
    <div v-if="store.biliVideoMode && store.nowPlaying?.source === 'bilibili'" class="bili-video-container">
      <div class="bili-video-header">
        <span class="bili-video-title">📺 {{ store.nowPlaying?.name || '' }}</span>
        <button class="bili-video-close" @click="closeVideo">✕</button>
      </div>
      <video ref="videoEl" class="bili-video-player" controls autoplay />
    </div>
  </div>
</template>

<script setup>
import { ref, watch, onMounted, onUnmounted } from 'vue'
import Sidebar    from '@/components/Sidebar.vue'
import SearchBar  from '@/components/SearchBar.vue'
import SongList   from '@/components/SongList.vue'
import PlayerBar  from '@/components/PlayerBar.vue'
import { usePlayerStore } from '@/stores/player'

const store = usePlayerStore()
const audioEl = ref(null)
const videoEl = ref(null)

// ── Audio element wired to store events ──
function onEnded() {
  store.isPlaying = false
  // Phase 5: auto-play next from queue
}

function onError(e) {
  console.error('[audio] error:', e)
  store.isPlaying = false
}

// Listen for store playback commands
function handleSetSrc(e) {
  const isVideo = store.biliVideoMode && store.nowPlaying?.source === 'bilibili'
  if (isVideo && videoEl.value) {
    videoEl.value.src = e.detail
    videoEl.value.play().catch(() => {})
    store.isPlaying = true
  } else if (audioEl.value) {
    audioEl.value.src = e.detail
    audioEl.value.play().catch(() => {})
    store.isPlaying = true
  }
}

function closeVideo() {
  store.toggleBiliVideoMode()
}

function handleTogglePlay() {
  const isVideo = store.biliVideoMode && store.nowPlaying?.source === 'bilibili'
  const el = isVideo ? videoEl.value : audioEl.value
  if (!el?.src) return
  if (el.paused) {
    el.play()
    store.isPlaying = true
  } else {
    el.pause()
    store.isPlaying = false
  }
}

// Phase 4: Emit time updates to lyrics window
let timeTickInterval = null
function startTimeTicks() {
  if (timeTickInterval) return
  timeTickInterval = setInterval(() => {
    const el = (store.biliVideoMode && store.nowPlaying?.source === 'bilibili') ? videoEl.value : audioEl.value
    if (el && !el.paused) {
      const ct = el.currentTime
      const dur = el.duration || 0
      store.emitLyricsTimeUpdate(ct, dur)
    }
  }, 200)
}

function stopTimeTicks() {
  if (timeTickInterval) {
    clearInterval(timeTickInterval)
    timeTickInterval = null
  }
}

onMounted(() => {
  window.addEventListener('player:set-src',  handleSetSrc)
  window.addEventListener('player:toggle-play', handleTogglePlay)
  startTimeTicks()
})

// 监听切换：切回音频时隐藏视频，释放资源
watch(() => store.biliVideoMode, (newVal) => {
  if (!newVal && videoEl.value) {
    videoEl.value.pause()
    videoEl.value.src = ''
  }
})

onUnmounted(() => {
  window.removeEventListener('player:set-src',  handleSetSrc)
  window.removeEventListener('player:toggle-play', handleTogglePlay)
  stopTimeTicks()
})
</script>

<style scoped>
.app {
  display: flex;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

/* B站视频播放器浮动窗口 */
.bili-video-container {
  position: fixed;
  bottom: 70px;
  right: 16px;
  width: 360px;
  z-index: 1000;
  background: #1a1a2e;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0,0,0,0.5);
}

.bili-video-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: #16213e;
  font-size: 13px;
}

.bili-video-title {
  color: #fff;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.bili-video-close {
  background: none;
  border: none;
  color: #999;
  cursor: pointer;
  font-size: 14px;
  padding: 0 4px;
}

.bili-video-close:hover {
  color: #fff;
}

.bili-video-player {
  width: 100%;
  display: block;
  max-height: 240px;
}
</style>
