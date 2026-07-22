import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

// ── API Base URL ──
// Dev (Vite proxy): relative → /api/* forwarded to 127.0.0.1:8899
// Build (warp serve): relative → /api/* served by same origin
const API = ''

// ── Source Config ──
export const SOURCE_CONFIG = {
  kuwo:    { name: '酷我音乐', desc: 'VIP歌曲免费播', mode: 'normal' },
  qq:      { name: 'QQ音乐',   desc: '正版曲库',      mode: 'qq' },
  bilibili:{ name: 'B站音乐',   desc: '视频转音频',    mode: 'bilibili' },
  kugou:   { name: '酷狗音乐',  desc: '概念版源',      mode: 'kugou' },
}

export const usePlayerStore = defineStore('player', () => {
  // ── State ──
  const currentSource  = ref('kuwo')
  const currentView    = ref('search') // 'search' | 'queue' | 'fav' | 'history' | 'local' | 'settings'
  const nowPlaying    = ref(null)      // current song object
  const queue         = ref([])        // upcoming songs
  const searchResults = ref([])
  const isSearching   = ref(false)
  const isPlaying     = ref(false)
  const searchKeyword = ref('')
  const biliVideoMode = ref(false)  // B站视频模式开关

  // ── Computed ──
  const sourceDisplay = computed(() => SOURCE_CONFIG[currentSource.value])
  const hasSong       = computed(() => !!nowPlaying.value)

  // ── Actions ──
  function setSource(source) {
    currentSource.value = source
  }

  function setView(view) {
    currentView.value = view
  }

  function toggleBiliVideoMode() {
    biliVideoMode.value = !biliVideoMode.value
    // 切换模式后，如果正在播B站歌曲，重新加载
    if (nowPlaying.value && nowPlaying.value.source === 'bilibili') {
      playSong(nowPlaying.value)
    }
  }

  async function search(keyword) {
    if (!keyword.trim()) return
    searchKeyword.value = keyword
    isSearching.value = true
    searchResults.value = []

    try {
      const mode = SOURCE_CONFIG[currentSource.value].mode
      const res = await fetch(
        `${API}/api/search?keyword=${encodeURIComponent(keyword)}&mode=${mode}`
      )
      const data = await res.json()
      searchResults.value = data.songs || []
    } catch (err) {
      console.error('[store] search error:', err)
    } finally {
      isSearching.value = false
    }
  }

  async function playSong(song) {
    nowPlaying.value = song
    isPlaying.value = true

    // Phase 5: 上报 now-playing 供歌词窗口轮询
    const source = song.source || currentSource.value
    const songId = song.song_id || song.id || ''
    const title = song.name || song.title || ''
    const artist = song.artist || song.singer || ''
    const sourceId = song.song_id || song.source_id || songId

    fetch(`${API}/api/now-playing`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        music_id: `mid_${source}_${songId}`,
        song_id: songId,
        title,
        artist,
        source,
        source_id: sourceId,
        duration: song.duration || 0,
      }),
    }).catch(e => console.error('[player] now-playing POST failed:', e))

    try {
      const mode = SOURCE_CONFIG[currentSource.value].mode
      let url = ''

      if (currentSource.value === 'kuwo') {
        url = `${API}/api/play/kuwo/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`
      } else if (currentSource.value === 'kugou') {
        url = `${API}/api/play/kugou/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`
      } else if (currentSource.value === 'qq') {
        url = `${API}/api/play/qq/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`
      } else if (currentSource.value === 'bilibili') {
        url = `${API}/api/play/bilibili?song_id=${encodeURIComponent(song.song_id)}&name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`
        // 视频模式时追加 type=video
        if (biliVideoMode.value) {
          url += '&type=video'
        }
      }

      const res = await fetch(url)
      const data = await res.json()
      if (data.url) {
        // Emit to audio element via event
        window.dispatchEvent(new CustomEvent('player:set-src', { detail: data.url }))
        // Phase 4: Notify lyrics window via Tauri event
        emitLyricsUpdate()
      }
    } catch (err) {
      console.error('[store] play error:', err)
    }
  }

  const musicId = computed(() => {
    if (!nowPlaying.value) return ''
    const source = nowPlaying.value.source || currentSource.value
    const songId = nowPlaying.value.song_id || nowPlaying.value.id || ''
    return `mid_${source}_${songId}`
  })

  const songTitle = computed(() => nowPlaying.value?.name || nowPlaying.value?.title || '')
  const songArtist = computed(() => nowPlaying.value?.artist || '')

  function togglePlay() {
    isPlaying.value = !isPlaying.value
    window.dispatchEvent(new CustomEvent('player:toggle-play'))
  }

  // ── Phase 4: Tauri 事件发射 (歌词窗口通信) ──
  async function emitLyricsUpdate() {
    try {
      const { emit } = await import('@tauri-apps/api/event')
      await emit('lyrics-update', {
        song: nowPlaying.value,
        isPlaying: isPlaying.value,
        currentTime: 0,
        duration: 0,
      })
    } catch { /* dev mode: Tauri API unavailable */ }
  }

  async function emitLyricsTimeUpdate(currentTime, duration) {
    try {
      const { emit } = await import('@tauri-apps/api/event')
      await emit('lyrics-time-update', { currentTime, duration })
    } catch { /* dev mode */ }
  }

  return {
    // state
    currentSource,
    currentView,
    nowPlaying,
    queue,
    searchResults,
    isSearching,
    isPlaying,
    searchKeyword,
    // computed
    sourceDisplay,
    hasSong,
    musicId,
    songTitle,
    songArtist,
    biliVideoMode,
    // actions
    setSource,
    setView,
    search,
    playSong,
    togglePlay,
    toggleBiliVideoMode,
    emitLyricsUpdate,
    emitLyricsTimeUpdate,
  }
})
