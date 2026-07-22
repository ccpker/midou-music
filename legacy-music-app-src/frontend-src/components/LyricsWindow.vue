<template>
  <div
    class="lyrics-window"
    :class="{ locked: isLocked }"
  >
    <!-- 顶部栏 -->
    <div class="title-bar" id="lyrics-titlebar">
      <span class="song-info" id="lyrics-song-info">{{ songTitle || '未播放' }}</span>
      <div class="window-actions">
        <button class="win-btn" id="btn-minimize" title="最小化">−</button>
        <button class="win-btn" id="btn-maximize" title="最大化">□</button>
        <button class="win-btn" id="btn-close" title="关闭">✕</button>
        <button
          class="win-btn lock-btn"
          id="btn-lock"
          :title="isLocked ? '点击解锁' : '点击锁定'"
        >
          {{ isLocked ? '🔓' : '🔒' }}
        </button>
      </div>
    </div>

    <!-- 歌词区域 -->
    <div class="lyrics-area" ref="lyricsArea">
      <!-- 空状态 -->
      <div v-if="!parsedLines.length" class="lyrics-empty">
        🎵 暂无歌词
      </div>

      <!-- LRC 行 -->
      <div
        v-for="line in parsedLines"
        :key="line.id"
        class="lrc-line"
        :class="{
          'lrc-active': line.isActive,
          'lrc-editing': editingId === line.id,
        }"
        :ref="(el) => setLineRef(line.id, el)"
        @dblclick="startEdit(line)"
      >
        <!-- 非编辑态 -->
        <template v-if="editingId !== line.id">
          {{ line.text }}
        </template>

        <!-- 编辑态 -->
        <input
          v-else
          class="lrc-edit-input"
          :value="line.text"
          @keydown.enter="saveEdit(line, $event)"
          @keydown.escape="cancelEdit"
          @blur="cancelEdit"
          ref="editInput"
        />
      </div>
    </div>

    <!-- 底部控制条（可隐藏） -->
    <div v-if="settings.showControls" class="control-bar">
      <button class="ctrl-btn" title="换源" @click="showSourcePanel = !showSourcePanel">🔄换源</button>
      <button class="ctrl-btn" title="上一首" @click="playerPrev">⏮</button>
      <button class="ctrl-btn play-btn" title="播放/暂停" @click="playerToggle">
        {{ playerStore.isPlaying ? '⏸' : '▶' }}
      </button>
      <button class="ctrl-btn" title="下一首" @click="playerNext">⏭</button>
      <!-- 进度条 -->
      <div class="progress-track" @click="seekProgress($event)">
        <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
      </div>
      <button class="ctrl-btn settings-btn" title="设置" @click="showSettings = !showSettings">
        <span v-if="showSettings">✕</span>
        <span v-else>⚙</span>
      </button>
    </div>

    <!-- 换源面板 -->
    <div v-if="showSourcePanel" class="source-panel">
      <div class="source-panel-header">
        <span>选择歌词源</span>
        <button class="close-btn" @click="showSourcePanel = false">✕</button>
      </div>
      <button class="source-option" @click="switchSource('lrclib')">🎵 LRCLIB</button>
      <button class="source-option" @click="switchSource('kuwo')">🎶 酷我歌词</button>
      <button class="source-option" @click="startManualEdit">✏️ 手动编辑</button>
    </div>

    <!-- 设置面板 -->
    <div v-if="showSettings && !isLocked" class="settings-panel">
      <div class="settings-item">
        <span>显示播放控件</span>
        <label class="toggle">
          <input type="checkbox" v-model="settings.showControls" />
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-item">
        <span>窗口置顶</span>
        <label class="toggle">
          <input type="checkbox" v-model="settings.alwaysOnTop" @change="toggleAlwaysOnTop" />
          <span class="toggle-slider"></span>
        </label>
      </div>
      <div class="settings-item">
        <span>字体大小</span>
        <select v-model="settings.fontSize" @change="applyFontSize">
          <option value="small">小</option>
          <option value="medium">中</option>
          <option value="large">大</option>
        </select>
      </div>

    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { usePlayerStore } from '@/stores/player'
import { useLibraryStore } from '@/stores/library'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'

const playerStore = usePlayerStore()
const libraryStore = useLibraryStore()
const appWindow = getCurrentWindow()

// ── 锁机制 ──
// 🔒/🔓 标准锁图标
const isLocked = ref(false)

const settings = ref({
  showControls: true,
  alwaysOnTop: false,
  fontSize: 'medium',
})

// ── 歌曲信息 ──
const songTitle = computed(() => {
  if (!playerStore.nowPlaying) return ''
  const name = playerStore.nowPlaying.name || playerStore.nowPlaying.title || ''
  const artist = playerStore.nowPlaying.artist || ''
  if (name && artist) return `${name} · ${artist}`
  return name || artist || ''
})

// ── Dev 模式 API base ──
const apiBase = (() => {
  try { return new URLSearchParams(location.search).get('port') ? `http://127.0.0.1:${new URLSearchParams(location.search).get('port')}` : '' }
  catch { return '' }
})()

// ── LRC 解析 ──
const parsedLines = ref([])
const currentLrc = ref('')
const editingId = ref(null)
const editInput = ref(null)
const lineRefs = {}
const lyricsArea = ref(null)
const currentTime = ref(0)
const showSourcePanel = ref(false)
const showSettings = ref(false)
const isManualEditing = ref(false)

function setLineRef(id, el) {
  if (el) lineRefs[id] = el
}

function parseLrc(lrcText) {
  if (!lrcText) return []

  // Split lines, support both \n and <br>
  const rawLines = lrcText.split(/\n|<br\s*\/?>/i)

  // Track if this is LRC format (has timestamps)
  const timestampRe = /\[(\d{1,2}):(\d{1,2})(?:\.(\d{1,3}))?\]/g
  let lineId = 0
  const results = []

  for (const raw of rawLines) {
    const trimmed = raw.trim()
    if (!trimmed) continue

    // Extract all timestamps in this line
    const timestamps = []
    let match
    while ((match = timestampRe.exec(raw)) !== null) {
      timestamps.push({
        full: match[0],
        mm: parseInt(match[1], 10),
        ss: parseInt(match[2], 10),
        ms: match[3] ? parseInt(match[3].padEnd(3, '0'), 10) : 0,
      })
    }

    if (timestamps.length === 0) {
      // Plain text line — no timestamp
      results.push({
        id: lineId++,
        text: trimmed,
        timeMs: -1,
        isActive: false,
      })
    } else {
      // For each timestamp, create a line with the text after last tag
      // Remove all timestamp tags to get the text
      const cleanText = trimmed.replace(timestampRe, '').trim()
      if (!cleanText) continue

      // Multi-tag: one line per timestamp
      for (const ts of timestamps) {
        const timeMs = ts.mm * 60000 + ts.ss * 1000 + ts.ms
        results.push({
          id: lineId++,
          text: cleanText,
          timeMs,
          isActive: false,
        })
      }
    }
  }

  // Sort by timeMs
  results.sort((a, b) => {
    if (a.timeMs < 0 && b.timeMs < 0) return 0
    if (a.timeMs < 0) return 1
    if (b.timeMs < 0) return -1
    return a.timeMs - b.timeMs
  })

  return results
}

function updateActiveLine() {
  const lines = parsedLines.value
  if (!lines.length) return

  // Find the line that should be active
  let activeIdx = -1
  for (let i = lines.length - 1; i >= 0; i--) {
    if (lines[i].timeMs >= 0 && lines[i].timeMs <= currentTime.value) {
      activeIdx = i
      break
    }
  }

  // Update active state
  let needsScroll = false
  lines.forEach((line, i) => {
    const newActive = i === activeIdx
    if (line.isActive !== newActive) {
      line.isActive = newActive
      if (newActive) needsScroll = true
    }
  })

  // Scroll active line into view
  if (needsScroll && activeIdx >= 0) {
    nextTick(() => {
      const el = lineRefs[lines[activeIdx].id]
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' })
      }
    })
  }
}

// ── 歌词编辑 ──
function startEdit(line) {
  if (isLocked.value) return
  editingId.value = line.id
  nextTick(() => {
    const input = document.querySelector('.lrc-edit-input')
    if (input) {
      input.focus()
      input.select()
    }
  })
}

function saveEdit(line, event) {
  const newText = event.target.value.trim()
  if (newText && newText !== line.text) {
    line.text = newText
    // Rebuild LRC text and save
    saveEditedLyrics()
  }
  editingId.value = null
}

function cancelEdit() {
  editingId.value = null
}

function sanitizeFileName(name) {
  return name.replace(/[\\/:*?"<>|]/g, '_').trim()
}

function makeMusicId() {
  // ⚠️ P4 fix: 对齐 library.rs make_music_id(source, source_id) = mid_{source}_{source_id}
  const source = playerStore.nowPlaying?.source || playerStore.currentSource || 'unknown'
  const sourceId = playerStore.nowPlaying?.song_id || playerStore.nowPlaying?.source_id || playerStore.nowPlaying?.id || 'unknown'
  return `mid_${source}_${sourceId}`
}

async function saveEditedLyrics() {
  // Rebuild LRC text from parsed lines
  const lines = parsedLines.value
    .filter(l => l.timeMs >= 0)
    .map(l => {
      const mm = Math.floor(l.timeMs / 60000)
      const ss = Math.floor((l.timeMs % 60000) / 1000)
      const ms = l.timeMs % 1000
      return `[${String(mm).padStart(2, '0')}:${String(ss).padStart(2, '0')}.${String(ms).padStart(2, '0')}]${l.text}`
    })

  const nonTimed = parsedLines.value
    .filter(l => l.timeMs < 0)
    .map(l => l.text)

  const newLrc = [...nonTimed, ...lines].join('\n')
  currentLrc.value = newLrc

  try {
    const musicId = makeMusicId()
    await libraryStore.saveLyrics(musicId, newLrc)
    console.log('[lyrics] 歌词已保存:', musicId)
  } catch (err) {
    console.error('[lyrics] 保存歌词失败:', err)
  }
}

// ── 换源 ──
async function switchSource(source) {
  showSourcePanel.value = false
  if (!playerStore.nowPlaying) return

  const name = playerStore.nowPlaying.name || playerStore.nowPlaying.title || ''
  const artist = playerStore.nowPlaying.artist || ''
  const musicId = makeMusicId()

  try {
    let lyric = null

    if (source === 'lrclib') {
      lyric = await fetchLrclibLyric(name, artist)
    } else if (source === 'kuwo') {
      lyric = await fetchKuwoLyric(name, artist)
    }

    if (lyric) {
      currentLrc.value = lyric
      parsedLines.value = parseLrc(lyric)
      // Save to library (store lrc content + lyrics_source in lrc metadata)
      await libraryStore.saveLyrics(musicId, lyric)
      console.log('[lyrics] 换源成功:', source)
    } else {
      alert('该源未找到歌词')
    }
  } catch (err) {
    console.error('[lyrics] 换源失败:', err)
    alert('换源失败: ' + err.message)
  }
}

async function fetchLrclibLyric(name, artist) {
  const encodedArtist = encodeURIComponent(artist)
  const encodedName = encodeURIComponent(name)
  const url = `https://lrclib.net/api/get?artist_name=${encodedArtist}&track_name=${encodedName}`

  const res = await fetch(url, {
    headers: { 'User-Agent': 'music-app/0.4.0' },
  })
  const data = await res.json()

  return data.syncedLyrics || data.plainLyrics || null
}

async function fetchKuwoLyric(name, artist) {
  const kw = encodeURIComponent(`${name} ${artist}`)
  const searchUrl = `http://search.kuwo.cn/r.s?all=${kw}&ft=music&itemset=web_2013&client=kt&pn=0&rn=1&rformat=json&encoding=utf8`

  const searchRes = await fetch(searchUrl, {
    headers: { 'User-Agent': 'Mozilla/5.0' },
  })
  let searchText = await searchRes.text()

  // Clean up kuwo's try{...} wrapper
  searchText = searchText.replace(/^try\{/, '').replace(/\}\)$/, '').replace(/'/g, '"')

  const searchData = JSON.parse(searchText)
  const first = searchData?.abslist?.[0]
  if (!first?.MUSICRID) return null

  const rid = first.MUSICRID.replace('MUSIC_', '')
  const lyricUrl = `https://mobi.kuwo.cn/mobi.s?f=so&rid=${rid}`
  const lyricRes = await fetch(lyricUrl, {
    headers: { 'User-Agent': 'Mozilla/5.0' },
  })
  const body = await lyricRes.text()

  if (!body || body.includes('err')) return null

  // Extract <lyric>...</lyric>
  const match = body.match(/<lyric>([\s\S]*?)<\/lyric>/)
  return match ? match[1].trim() : null
}

function startManualEdit() {
  showSourcePanel.value = false
  isManualEditing.value = true
}

// ── 进度条 ──
const progressPercent = ref(0)

function seekProgress(event) {
  const rect = event.currentTarget.getBoundingClientRect()
  const ratio = (event.clientX - rect.left) / rect.width
  // Emit seek event to main window
  try {
    const { emit } = window.__TAURI__?.event || {}
    if (emit) {
      emit('lyrics-seek', { ratio: Math.max(0, Math.min(1, ratio)) })
    }
  } catch {}
}

// ── 窗口控制（通过原生 addEventListener 绑定，见 onMounted） ──

async function toggleAlwaysOnTop() {
  try {
    await appWindow.setAlwaysOnTop(settings.value.alwaysOnTop)
  } catch {}
}

// ── 播放控制 ──
function playerToggle() {
  playerStore.togglePlay()
}

function playerPrev() {
  try {
    const { emit } = window.__TAURI__?.event || {}
    if (emit) emit('lyrics-player-prev')
  } catch {}
}

function playerNext() {
  try {
    const { emit } = window.__TAURI__?.event || {}
    if (emit) emit('lyrics-player-next')
  } catch {}
}

// ── 字体大小 ──
function applyFontSize() {
  const sizes = { small: '14px', medium: '18px', large: '24px' }
  document.documentElement.style.setProperty('--lrc-font-size', sizes[settings.value.fontSize])
}

// ── 歌词加载 ──
let lastMusicId = ''

async function loadLyrics() {
  if (!playerStore.nowPlaying) {
    parsedLines.value = []
    return
  }

  const musicId = makeMusicId()
  if (musicId === lastMusicId && parsedLines.value.length > 0) return
  lastMusicId = musicId

  try {
    const stored = await libraryStore.getLyrics(musicId)
    if (stored) {
      currentLrc.value = stored
      parsedLines.value = parseLrc(stored)
      return
    }
  } catch {}

  try {
    const name = playerStore.nowPlaying.name || playerStore.nowPlaying.title || ''
    const artist = playerStore.nowPlaying.artist || ''
    const source = playerStore.nowPlaying.source || ''
    const duration = playerStore.nowPlaying.duration || 0
    const songId = playerStore.nowPlaying.song_id || ''

    let params = `name=${encodeURIComponent(name)}&artist=${encodeURIComponent(artist)}`
    if (source) params += `&source=${encodeURIComponent(source)}`
    if (duration > 0) params += `&duration=${duration * 1000}`
    if (source === 'kugou' && songId) {
      const hash = songId.split('|')[0]
      if (hash) params += `&hash=${encodeURIComponent(hash)}`
    }

    const res = await fetch(`${apiBase}/api/lyric?${params}`)
    const data = await res.json()
    if (data.lyric) {
      currentLrc.value = data.lyric
      parsedLines.value = parseLrc(data.lyric)
    } else {
      parsedLines.value = []
    }
  } catch (err) {
    console.error('[lyrics] 歌词加载失败:', err)
    parsedLines.value = []
  }
}

// ── Phase 5: 轮询 now-playing（dev/fallback，Tauri events 优先） ──
let npPollInterval = null

async function pollNowPlaying() {
  try {
    const res = await fetch(`${apiBase}/api/now-playing`)
    if (!res.ok) return
    const data = await res.json()
    if (!data || !data.title) return

    // 切歌检测
    const newMusicId = `mid_${data.source}_${data.source_id}`
    if (newMusicId !== lastMusicId) {
      playerStore.nowPlaying = {
        name: data.title,
        title: data.title,
        artist: data.artist,
        source: data.source,
        song_id: data.source_id,
      }
      playerStore.isPlaying = true
      loadLyrics()
    }
  } catch { /* ignore poll errors */ }
}

function startNpPoll() {
  if (npPollInterval) return
  pollNowPlaying()
  npPollInterval = setInterval(pollNowPlaying, 800)
}

function stopNpPoll() {
  if (npPollInterval) { clearInterval(npPollInterval); npPollInterval = null }
}

// ── 进度监听 — 轮询 now-playing API ──
let timeInterval = null

function startTimeTracking() {
  if (timeInterval) return
  timeInterval = setInterval(async () => {
    try {
      const res = await fetch(`${apiBase}/api/now-playing`)
      const data = await res.json()
      if (!data) return

      const storeNowPlaying = usePlayerStore().nowPlaying
      const newMusicId = (data.music_id || '').replace(/^mid_/, '')
      const currentMusicId = (storeNowPlaying?.music_id || `mid_${storeNowPlaying?.source}_${storeNowPlaying?.song_id}`).replace(/^mid_/, '')

      if (data.music_id && newMusicId !== currentMusicId) {
        const newSong = {
          name: data.title,
          title: data.title,
          artist: data.artist,
          song_id: data.song_id,
          source: data.source,
          source_id: data.source_id,
          duration: data.duration,
        }
        playerStore.nowPlaying = newSong
        playerStore.isPlaying = true
        await loadLyrics()
      }
    } catch { /* dev mode: no warp running yet */ }
  }, 500)
}

function stopTimeTracking() {
  if (timeInterval) {
    clearInterval(timeInterval)
    timeInterval = null
  }
}

// ── Tauri 事件注册（由 onMounted 调用） ──
async function registerTauriEvents() {
  try {
    // 切歌更新
    await listen('lyrics-update', (event) => {
      console.log('[lyrics] 收到切歌事件:', event.payload)
      if (event.payload?.song) {
        playerStore.nowPlaying = event.payload.song
        playerStore.isPlaying = event.payload.isPlaying || false
        loadLyrics()
      }
    })

    // 时间更新
    await listen('lyrics-time-update', (event) => {
      if (event.payload?.currentTime !== undefined) {
        currentTime.value = event.payload.currentTime * 1000
        updateActiveLine()
      }
      if (event.payload?.duration !== undefined) {
        const dur = event.payload.duration
        const cur = event.payload.currentTime || 0
        progressPercent.value = dur > 0 ? (cur / dur) * 100 : 0
      }
    })

    // 主题同步（来自主应用）
    await listen('theme-changed', (event) => {
      if (event.payload?.theme) {
        document.documentElement.setAttribute('data-theme', event.payload.theme)
        console.log('[lyrics] 主题切换:', event.payload.theme)
      }
    })
  } catch (err) {
    console.warn('[lyrics] Tauri events unavailable (dev mode):', err)
  }
}

// ── Lifecycle ──
onMounted(async () => {
  // 从 localStorage 读初始主题
  try {
    const saved = localStorage.getItem('music-app-theme')
    if (saved) document.documentElement.setAttribute('data-theme', saved)
  } catch {}
  applyFontSize()

  // ── 官方文档方式：原生事件监听（JS 方案，适合锁定/解锁切换） ──

  // 1. 窗口控制按钮
  document.getElementById('btn-minimize')?.addEventListener('click', () => appWindow.minimize())
  document.getElementById('btn-maximize')?.addEventListener('click', () => appWindow.toggleMaximize())
  document.getElementById('btn-close')?.addEventListener('click', () => appWindow.close())

  // 2. 拖拽：mousedown + startDragging（官方推荐的可定制方案）
  const titlebar = document.getElementById('lyrics-titlebar')
  if (titlebar) {
    titlebar.addEventListener('mousedown', (e) => {
      // 只在解锁态且左键时拖拽
      if (!isLocked.value && e.buttons === 1) {
        e.detail === 2
          ? appWindow.toggleMaximize()  // 双击最大化
          : appWindow.startDragging()   // 拖拽
      }
    })
  }

  // 3. 锁按钮
  document.getElementById('btn-lock')?.addEventListener('click', (e) => {
    e.stopPropagation()
    isLocked.value = !isLocked.value
  })

  // 4. Tauri 事件监听
  await registerTauriEvents()

  // 其他初始化
  if (playerStore.nowPlaying) loadLyrics()
  startNpPoll()
})

onUnmounted(() => {
  stopTimeTracking()
  stopNpPoll()
})
</script>

<style scoped>
/* ── 窗口基础 ── */
.lyrics-window {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  color: var(--text);
  overflow: hidden;
  user-select: none;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

/* 锁定态：内容区禁止交互 */
.lyrics-window.locked .lyrics-area,
.lyrics-window.locked .control-bar {
  pointer-events: none;
}

/* 锁定态：title-bar 不再可拖动 */
.lyrics-window.locked .title-bar {
  cursor: default;
}

/* ── 顶部栏（Tauri v2 使用 data-tauri-drag-region 实现拖动） ── */
.title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 4px 0 8px;
  background: var(--card);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  position: relative;
  cursor: move;
  user-select: none;
}

.song-info {
  font-size: 12px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 55%;
  flex-shrink: 0;
  user-select: none;
}

.window-actions {
  display: flex;
  gap: 2px;
  align-items: center;
  flex-shrink: 0;
  /* 按钮永远可点击，不受 drag 区域影响 */
  pointer-events: auto;
}

.win-btn {
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: 4px;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s, color 0.2s;
  /* 按钮永远可点击 */
  pointer-events: auto;
}

.win-btn:hover {
  background: var(--hover);
  color: var(--text);
}

/* 锁按钮：锁定态红色⛔，解锁态橙色🔓 */
.lock-btn {
  font-size: 17px;
  transition: color 0.2s, transform 0.15s;
}

.lyrics-window.locked .lock-btn {
  color: #e53e3e;
}

.lyrics-window:not(.locked) .lock-btn {
  color: #dd6b20;
}

.lock-btn:hover {
  transform: scale(1.2);
}

/* ── 歌词区域 ── */
.lyrics-area {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 20px 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.lyrics-empty {
  color: var(--muted);
  font-size: var(--lrc-font-size, 18px);
  margin-top: 40%;
}

:root {
  --lrc-font-size: 18px;
}

.lrc-line {
  font-size: var(--lrc-font-size, 18px);
  line-height: 1.8;
  color: var(--muted);
  text-align: center;
  transition: color 0.3s, transform 0.3s;
  cursor: default;
  padding: 4px 12px;
  border-radius: 4px;
  max-width: 100%;
  word-break: break-all;
  flex-shrink: 0;
}

.lrc-line.lrc-active {
  color: var(--accent);
  font-weight: 600;
}

.lrc-line:hover {
  background: var(--hover);
}

.lrc-edit-input {
  font-size: var(--lrc-font-size, 18px);
  line-height: 1.8;
  background: var(--card);
  color: var(--text);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 4px 8px;
  text-align: center;
  width: 100%;
  outline: none;
}

/* ── 底部控制条 ── */
.control-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 44px;
  padding: 0 12px;
  background: var(--card);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.ctrl-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: 4px;
  font-size: 13px;
  padding: 4px 8px;
  transition: background 0.2s, color 0.2s;
  flex-shrink: 0;
}

.ctrl-btn:hover {
  background: var(--hover);
  color: var(--text);
}

.play-btn {
  font-size: 16px;
}

.settings-btn {
  margin-left: auto;
  font-size: 14px;
}

/* ── 进度条 ── */
.progress-track {
  flex: 1;
  height: 6px;
  background: var(--border);
  border-radius: 3px;
  cursor: pointer;
  min-width: 60px;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.2s linear;
}

/* ── 换源面板 ── */
.source-panel {
  position: absolute;
  bottom: 52px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  z-index: 200;
  min-width: 180px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.source-panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--muted);
  padding: 4px 8px;
}

.close-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
}

.source-option {
  border: none;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 4px;
  text-align: left;
  font-size: 13px;
  transition: background 0.2s;
}

.source-option:hover {
  background: var(--hover);
}

/* ── 设置面板 ── */
.settings-panel {
  position: absolute;
  bottom: 52px;
  right: 12px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  z-index: 200;
  min-width: 200px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.settings-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
}

.settings-item select {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 12px;
}

/* ── Toggle ── */
.toggle {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--border);
  transition: 0.3s;
  border-radius: 10px;
}

.toggle-slider::before {
  content: "";
  position: absolute;
  height: 14px;
  width: 14px;
  left: 3px;
  bottom: 3px;
  background-color: var(--text);
  transition: 0.3s;
  border-radius: 50%;
}

.toggle input:checked + .toggle-slider {
  background-color: var(--accent);
}

.toggle input:checked + .toggle-slider::before {
  transform: translateX(16px);
}
</style>
