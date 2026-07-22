<template>
  <div class="content">
    <!-- Search results -->
    <div v-if="store.currentView === 'search'" class="results-area">
      <div v-if="store.isSearching" class="state-msg">
        搜索中...
      </div>
      <div v-else-if="store.searchResults.length === 0 && store.searchKeyword" class="state-msg">
        无结果
      </div>
      <div v-else-if="store.searchResults.length === 0" class="state-msg">
        输入关键词开始搜索
      </div>

      <div v-else class="results">
        <div
          v-for="song in store.searchResults"
          :key="song.song_id"
          class="song-item"
          @click="store.playSong(song)"
        >
          <div class="song-info">
            <div class="song-title">
              <!-- 来源徽章：B站📺, QQ-VIP🔒, QQ免费, 酷狗, 酷我 -->
              <span v-if="song.source === 'bilibili'" class="source-badge bili">📺</span>
              <span v-if="song.source === 'qq' && song.score === 1" class="source-badge qq-vip" title="QQ音乐VIP">🔒 VIP</span>
              <span v-if="song.source === 'qq' && song.score !== 1" class="source-badge qq">QQ</span>
              <span v-if="song.source === 'kugou'" class="source-badge kg">酷狗</span>
              <span v-if="song.source === 'kuwo'" class="source-badge kw">酷我</span>
              {{ song.name }}
            </div>
            <div class="song-meta">
              <span v-if="song.singer" class="meta-singer">{{ song.singer }}</span>
              <span v-if="song.album && song.album !== song.name" class="meta-album"> · {{ song.album }}</span>
              <span v-if="song.duration" class="meta-dur"> · {{ fmtDur(song.duration) }}</span>
            </div>
          </div>
          <div class="song-actions" @click.stop>
            <button @click="store.playSong(song)">播放</button>
            <button @click="addToQueue(song)">+队列</button>
            <!-- U1: 收藏按钮 -->
            <button
              v-if="!isResultCollected(song)"
              class="collect-btn"
              @click="collectSong(song)"
              :disabled="collectingId === song.song_id"
            >
              {{ collectingId === song.song_id ? '收藏中...' : '收藏' }}
            </button>
            <span v-else class="collected-badge">已收藏</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 收藏视图 — 文件夹树 + 回收站 -->
    <div v-else-if="store.currentView === 'fav'" class="fav-area">
      <div v-if="!libStore.hasActiveLibrary" class="state-msg">
        <div class="placeholder-icon">♡</div>
        暂无音乐库<br>
        <small>请在设置中创建或切换音乐库</small>
        <div style="margin-top: 16px">
          <button class="import-btn" @click="store.setView('settings')">⚙️ 前往设置</button>
        </div>
      </div>
      <template v-else>
        <!-- Folder tree -->
        <div class="fav-header">
          <h3 class="section-title">📂 {{ libStore.activeLibrary?.name || '收藏' }}</h3>
          <div class="fav-actions">
            <button class="small-btn" @click="showCreateFolder = true">+ 新文件夹</button>
            <button class="small-btn" :disabled="libStore.importing" @click="libStore.importLocalMusic()">
              {{ libStore.importing ? libStore.importProgress || '导入中...' : '📁 导入' }}
            </button>
          </div>
        </div>

        <!-- 创建文件夹输入框 -->
        <div v-if="showCreateFolder" class="create-folder-row">
          <input
            ref="folderInput"
            v-model="newFolderName"
            placeholder="文件夹名称"
            @keyup.enter="doCreateFolder"
            class="folder-input"
          />
          <button class="small-btn" @click="doCreateFolder">确认</button>
          <button class="small-btn cancel" @click="showCreateFolder = false; newFolderName = ''">取消</button>
        </div>

        <p v-if="libStore.importError" style="color: #e94560; font-size: 12px; margin: 4px 0">
          {{ libStore.importError }}
        </p>

        <!-- Folder tree -->
        <FolderNode
          v-for="folder in rootFolders"
          :key="folder.id"
          :folder="folder"
          :depth="0"
          @play="onPlayFolderSong"
          @move-to-trash="onMoveToTrash"
          @move-to-folder="onShowMoveMenu"
        />

        <!-- 右键菜单：三档操作 -->
        <div
          v-if="moveMenu.visible"
          class="context-menu"
          :style="{ top: moveMenu.y + 'px', left: moveMenu.x + 'px' }"
        >
          <div class="menu-title">歌曲操作</div>
          <!-- 档1: 移出本文件夹 → 默认文件夹 -->
          <div class="menu-item" @click="doMoveToDefault(moveMenu.musicId)">
            📂 移出本文件夹（归入默认）
          </div>
          <!-- 档2: 移到回收站 -->
          <div class="menu-item danger" @click="doMoveToTrashMenu(moveMenu.musicId)">
            🗑️ 移到回收站
          </div>
          <div class="menu-subtitle">移动到文件夹</div>
          <div
            v-for="f in allFolders"
            :key="f.id"
            class="menu-item"
            @click="doMoveToFolder(moveMenu.musicId, f.id)"
          >
            {{ f.name }}
          </div>
          <div class="menu-item cancel" @click="moveMenu.visible = false">取消</div>
        </div>

        <!-- 回收站 -->
        <div v-if="libStore.trashItems.length > 0" class="trash-section">
          <div class="trash-header">
            <h3 class="section-title">🗑️ 回收站 ({{ libStore.trashItems.length }})</h3>
            <button class="small-btn danger" @click="onEmptyTrash">清空回收站</button>
          </div>
          <div
            v-for="item in libStore.trashItems"
            :key="item.music_id"
            class="trash-item"
          >
            <div class="song-info">
              <div class="song-title">{{ item.title || item.music_id }}</div>
              <div class="song-meta">
                {{ item.artist || '未知' }} · 删除于 {{ formatDate(item.deleted_at) }}
              </div>
            </div>
            <div class="song-actions">
              <button class="restore-btn" @click="onRestore(item.music_id)">恢复</button>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- 本地视图 — 文件夹树 + 导入入口 -->
    <div v-else-if="store.currentView === 'local'" class="local-area">
      <div v-if="!libStore.hasActiveLibrary" class="state-msg">
        <div class="placeholder-icon">💾</div>
        暂无音乐库<br>
        <small>请在设置中创建或切换音乐库</small>
      </div>
      <template v-else>
        <div class="fav-header">
          <h3 class="section-title">💾 本地文件</h3>
          <div class="fav-actions">
            <button class="small-btn" :disabled="libStore.importing" @click="libStore.importLocalMusic()">
              {{ libStore.importing ? libStore.importProgress || '导入中...' : '📁 导入本地音乐' }}
            </button>
          </div>
        </div>
        <p v-if="libStore.importError" style="color: #e94560; font-size: 12px; margin: 4px 0">
          {{ libStore.importError }}
        </p>
        <FolderNode
          v-for="folder in rootFolders"
          :key="folder.id"
          :folder="folder"
          :depth="0"
          @play="onPlayFolderSong"
          @move-to-trash="onMoveToTrash"
          @move-to-folder="onShowMoveMenu"
        />
      </template>
    </div>

    <!-- Placeholder views -->
    <div v-else class="state-msg">
      <template v-if="store.currentView === 'queue'">
        <div class="placeholder-icon">📋</div>
        队列为空
      </template>
      <template v-else-if="store.currentView === 'history'">
        <div class="placeholder-icon">📜</div>
        暂无播放历史
      </template>
      <template v-else-if="store.currentView === 'settings'">
        <SettingsPanel />
      </template>
    </div>

    <!-- Overlay to close context menu -->
    <div v-if="moveMenu.visible" class="menu-overlay" @click="moveMenu.visible = false"></div>
  </div>
</template>

<script setup>
import { ref, computed, nextTick, onMounted, watch } from 'vue'
import { usePlayerStore } from '@/stores/player'
import { useLibraryStore } from '@/stores/library'
import FolderNode from './FolderNode.vue'
import SettingsPanel from './SettingsPanel.vue'

const store = usePlayerStore()
const libStore = useLibraryStore()

// 时长格式化: 秒 → "3:45"
function fmtDur(sec) {
  if (!sec || sec <= 0) return ''
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

// ── U1: 收藏状态 ──
const collectingId = ref(null)

function isResultCollected(song) {
  const source = 'kuwo'
  const sourceId = song.song_id || song.id || ''
  return libStore.isSongCollected(source, sourceId)
}

async function collectSong(song) {
  if (!libStore.hasActiveLibrary) {
    alert('请先在设置中创建或切换音乐库')
    return
  }

  const rid = song.song_id || song.id
  if (!rid) return

  const singer = song.artist || song.singer || ''
  const name = song.name || song.title || ''

  collectingId.value = rid
  try {
    const resp = await fetch(
      `/api/download/kuwo/${encodeURIComponent(rid)}?name=${encodeURIComponent(name)}&singer=${encodeURIComponent(singer)}&category=${encodeURIComponent('默认')}`
    )
    const data = await resp.json()

    if (data.error) {
      alert('收藏失败: ' + data.error)
      return
    }

    if (data.path && data.filename) {
      await libStore.addSongToLibrary(
        name,
        singer,
        'kuwo',
        rid,
        data.filename,
        data.file_size || 0,
        libStore.defaultFolderId || null
      )
      console.log(`[SongList] 收藏入库: ${name} — ${singer}`)
    }
  } catch (err) {
    alert('操作失败: ' + err.message)
  } finally {
    collectingId.value = null
  }
}

// ── 创建文件夹 ──
const showCreateFolder = ref(false)
const newFolderName = ref('')
const folderInput = ref(null)

async function doCreateFolder() {
  const name = newFolderName.value.trim()
  if (!name) return
  await libStore.createFolder(name)
  newFolderName.value = ''
  showCreateFolder.value = false
}

onMounted(() => {
  libStore.loadLibraries()
})

watch(showCreateFolder, async (val) => {
  if (val) await nextTick(() => folderInput.value?.focus())
})

// ── 文件夹树 ──
const rootFolders = computed(() => {
  return libStore.folders.filter(f => f.parent_id === null || !f.parent_id)
})

const allFolders = computed(() => libStore.folders)

// ── 右键菜单 ──
const moveMenu = ref({ visible: false, x: 0, y: 0, musicId: '' })

function onShowMoveMenu(musicId, event) {
  moveMenu.value = {
    visible: true,
    x: event.clientX,
    y: event.clientY,
    musicId,
  }
}

async function doMoveToFolder(musicId, folderId) {
  moveMenu.value.visible = false
  await libStore.addSongToFolder(musicId, folderId)
}

// ── 档1: 移出本文件夹 → 默认文件夹 ──
async function doMoveToDefault(musicId) {
  moveMenu.value.visible = false
  if (!libStore.defaultFolderId) return
  await libStore.addSongToFolder(musicId, libStore.defaultFolderId)
}

// ── 档2: 移到回收站 ──
async function doMoveToTrashMenu(musicId) {
  moveMenu.value.visible = false
  if (confirm('确定将此歌曲移到回收站？')) {
    await libStore.moveToTrash(musicId)
  }
}

// ── 歌曲操作 ──
function onPlayFolderSong(song) {
  store.nowPlaying = song
  store.isPlaying = true
  // 触发播放
  window.dispatchEvent(new CustomEvent('player:set-src', { detail: song.file_path }))
}

async function onMoveToTrash(musicId) {
  if (confirm('确定删除此歌曲到回收站？')) {
    await libStore.moveToTrash(musicId)
  }
}

async function onRestore(musicId) {
  await libStore.restoreFromTrash(musicId)
}

async function onEmptyTrash() {
  if (confirm(`确定清空回收站？将永久删除 ${libStore.trashItems.length} 首歌。`)) {
    await libStore.emptyTrash()
  }
}

function addToQueue(song) {
  store.queue.push(song)
}

function formatDate(iso) {
  if (!iso) return ''
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}
</script>

<style scoped>
.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  padding-bottom: 80px;
  position: relative;
}

.results-area, .fav-area, .local-area {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.state-msg {
  color: var(--muted);
  padding: 40px;
  text-align: center;
  font-size: 14px;
}

.placeholder-icon {
  font-size: 36px;
  margin-bottom: 12px;
}

small {
  font-size: 12px;
  color: var(--muted);
}

/* ── Section headers ── */
.fav-header, .trash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
}

.section-title {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
  color: var(--text);
}

.fav-actions {
  display: flex;
  gap: 6px;
}

.small-btn {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  cursor: pointer;
  font-size: 11px;
  transition: all 0.2s;
}

.small-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}

.small-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.small-btn.cancel {
  background: transparent;
  color: var(--muted);
}

.small-btn.danger {
  color: #e94560;
  border-color: #e94560;
}

.small-btn.danger:hover {
  background: #e94560;
  color: #fff;
}

/* ── Create folder row ── */
.create-folder-row {
  display: flex;
  gap: 6px;
  align-items: center;
  padding: 4px 0;
}

.folder-input {
  flex: 1;
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
}

/* ── Search results ── */
.results {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.song-item, .trash-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  background: var(--card);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.2s;
  gap: 12px;
}

.song-item:hover, .trash-item:hover {
  background: var(--hover);
}

.song-info {
  flex: 1;
  min-width: 0;
}

.song-title {
  font-weight: 500;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.source-badge {
  display: inline-block;
  font-size: 0.7em;
  padding: 0 5px;
  border-radius: 3px;
  margin-right: 5px;
  vertical-align: middle;
  background: #4a5568;
  color: #fff;
}
.source-badge.bili { background: #FF6B9D; font-size: 0.85em; }  /* B站粉红 */
.source-badge.qq-vip { background: #FFD700; color: #333; font-weight: 700; }  /* QQ金色VIP */
.source-badge.qq { background: #2d8cf0; }
.source-badge.kg { background: #ff6b35; }
.source-badge.kw { background: #5e72e4; }

.meta-singer { color: #a0aec0; }
.meta-album { color: #718096; font-size: 0.9em; }
.meta-dur { color: #4a5568; font-size: 0.85em; }

.song-meta {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.song-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.song-actions button {
  padding: 6px 12px;
  border: none;
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text);
  cursor: pointer;
  font-size: 11px;
  transition: background 0.2s;
}

.song-actions button:hover {
  background: var(--accent);
}

.collect-btn {
  background: var(--accent) !important;
  color: #fff !important;
}

.collect-btn:hover:not(:disabled) {
  background: #0ea5e9 !important;
}

.collect-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.collected-badge {
  font-size: 11px;
  color: var(--accent);
  padding: 4px 8px;
  font-weight: 500;
}

.restore-btn {
  padding: 4px 10px;
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  background: transparent;
  color: var(--accent);
  cursor: pointer;
  font-size: 11px;
}

.restore-btn:hover {
  background: var(--accent);
  color: #fff;
}

.import-btn {
  padding: 8px 16px;
  border: 1px dashed var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--accent);
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.import-btn:hover:not(:disabled) {
  background: var(--card);
  border-color: var(--accent);
}

/* ── Trash ── */
.trash-section {
  margin-top: 20px;
  padding-top: 12px;
  border-top: 2px dashed var(--border);
}

.trash-item {
  opacity: 0.7;
}

/* ── Context menu ── */
.context-menu {
  position: fixed;
  z-index: 1000;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  min-width: 160px;
  padding: 4px 0;
}

.menu-title {
  padding: 6px 12px;
  font-size: 11px;
  color: var(--muted);
  border-bottom: 1px solid var(--border);
}

.menu-item {
  padding: 6px 12px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text);
  transition: background 0.15s;
}

.menu-item:hover {
  background: var(--hover);
}

.menu-item.cancel {
  color: var(--muted);
  border-top: 1px solid var(--border);
}

.menu-item.danger {
  color: #e94560;
}

.menu-item.danger:hover {
  background: rgba(233, 69, 96, 0.15);
}

.menu-subtitle {
  padding: 6px 12px;
  font-size: 11px;
  color: var(--muted);
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
}

.menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
}
</style>
