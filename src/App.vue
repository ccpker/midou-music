<script setup lang="ts">
/**
 * 组件: App.vue（主窗口根组件）
 * 功能: 侧边栏 + 主面板 + 搜索/播放入口编排
 * 标注: 本文件只做 UI 编排，具体逻辑委托给 composables
 */
import { ref, onMounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import Sidebar from './components/Sidebar.vue';
import SearchBar from './components/SearchBar.vue';
import SongList from './components/SongList.vue';
import KugouLogin from './components/KugouLogin.vue';
import FolderNode from './components/FolderNode.vue';
import { playSong, type Song } from './composables/usePlayer';
import { search, useSearch } from './composables/useSearch';
import { useKugouPlaylist, type KugouPlaylistSong, type KugouPlaylist } from './composables/useKugouPlaylist';
import { vipStatus, isSignedToday, signLoading, signError, signSuccess, fetchVipStatus, signVip, autoSignVip, bindAutoSign, watchAd, adLoading, adResult, adError } from './composables/useKugouVip';
import { useKugouFm, type FmSong } from './composables/useKugouFm';
import { downloadSong, libraryRoot } from './composables/useDownload';
import { libraryTree, libraryLoading, libraryError, currentFolder, scanLibrary, openFolder, type LibraryNode, type LocalSong } from './composables/useLibrary';

const { results, loading } = useSearch();

const { fmSongs, loading: fmLoading, error: fmError, fetchPersonalFm } = useKugouFm();

const {
  playlists,
  loadingLists,
  songs: kugouSongs,
  loadingSongs,
  error: kugouPlaylistError,
  currentPlaylist,
  fetchPlaylists,
  fetchPlaylistSongs,
} = useKugouPlaylist();

// ── 状态 ─────────────────────────────────────────

const activeSection = ref('kuwo_search');
const currentKeyword = ref('');

// section → 音源映射
const SOURCE_MAP: Record<string, string> = {
  kuwo_search:  'kuwo',
  bili_search:  'bili',
  kugou_search: 'kugou',
  qq_search:    'qq',
};

// ── 导航（切换平台时自动重搜）───────────────────

function navigate(section: string) {
  activeSection.value = section;
  // 进入酷狗收藏时自动拉取歌单列表 + VIP 状态
  if (section === 'fav_kugou') {
    fetchPlaylists();
    fetchVipStatus();
  }
  // 进入设置页时刷新 VIP 状态
  if (section === 'settings') {
    fetchVipStatus();
  }
  // 进入本地音乐库时扫描
  if (section === 'local_all') {
    handleEnterLocal();
  }
  // 切换到有结果的平台时自动重搜
  if (currentKeyword.value && SOURCE_MAP[section]) {
    search(currentKeyword.value, SOURCE_MAP[section]);
  }
}

// ── 搜索 ────────────────────────────────────────

async function handleSearch(keyword: string) {
  currentKeyword.value = keyword;
  // 搜索默认用当前激活的平台
  const src = SOURCE_MAP[activeSection.value] ?? 'kuwo';
  await search(keyword, src);
}

// ── 播放状态横幅 ───────────────────────────────

const playBanner = ref<{ song: string; ok: boolean; msg: string } | null>(null);

async function handlePlay(song: Song) {
  // 立即显示"播放中"，2秒后自动消失
  playBanner.value = { song: song.name, ok: true, msg: '播放中...' };
  try {
    await playSong(song);
  } catch(e) {
    playBanner.value = { song: song.name, ok: false, msg: '失败: ' + String(e) };
  }
  setTimeout(() => { playBanner.value = null; }, 3000);
}

// ── 酷狗收藏/歌单 ──────────────────────────────────

async function handleKugouFav() {
  await fetchPlaylists();
}

async function handleOpenPlaylist(pl: KugouPlaylist) {
  await fetchPlaylistSongs(pl);
}

// 侧边栏点歌单：切到酷狗收藏区 + 加载该歌单
async function handleOpenKugouPlaylist(pl: KugouPlaylist) {
  activeSection.value = 'fav_kugou';
  await handleOpenPlaylist(pl);
}

function kugouSongToSong(s: KugouPlaylistSong): Song {
  return {
    song_id: s.song_id,
    name: s.name,
    singer: s.singer,
    album: s.album,
    duration: s.duration,
    source: s.source,
    cover_url: s.cover_url,
  };
}

async function handlePlayKugouFav(s: KugouPlaylistSong) {
  await handlePlay(kugouSongToSong(s));
}

async function handleDownloadKugouFav(s: KugouPlaylistSong) {
  await handleDownload(kugouSongToSong(s));
}

// ── 私人 FM ─────────────────────────────────────

function fmSongToSong(s: FmSong): Song {
  return {
    song_id: s.song_id,
    name: s.name,
    singer: s.singer,
    album: s.album,
    duration: s.duration,
    source: s.source,
    cover_url: s.cover_url,
  };
}

async function handlePlayFm(s: FmSong) {
  await handlePlay(fmSongToSong(s));
}

async function handleDownloadFm(s: FmSong) {
  await handleDownload(fmSongToSong(s));
}

async function handleOpenFm() {
  activeSection.value = 'fm_kugou';
  await fetchPersonalFm();
}

// ── 下载 ────────────────────────────────────────

const dlBanner = ref<{ name: string; ok: boolean; msg: string } | null>(null);

async function handleDownload(song: Song) {
  dlBanner.value = { name: song.name, ok: true, msg: '开始下载...' };
  const path = await downloadSong(song);
  if (path) {
    dlBanner.value = { name: song.name, ok: true, msg: '已下载到 ' + path };
  } else {
    dlBanner.value = { name: song.name, ok: false, msg: '下载失败或已取消' };
  }
  setTimeout(() => { dlBanner.value = null; }, 4000);
}

// ── 本地音乐库 ────────────────────────────────────

async function handleEnterLocal() {
  if (!libraryRoot.value) return; // 未选目录，显示引导
  if (!libraryTree.value) {
    await scanLibrary();
  }
}

function handleOpenFolder(node: LibraryNode) {
  openFolder(node);
}

function localSongToSong(s: LocalSong): Song {
  return {
    song_id: s.song_id,
    name: s.name,
    singer: s.singer,
    album: s.album,
    duration: s.duration,
    source: s.source,
    cover_url: s.cover_url,
  };
}

async function handlePlayLocal(s: LocalSong) {
  await handlePlay(localSongToSong(s));
}

// ── 面板标题 ────────────────────────────────────

function panelTitle(id: string): string {
  const map: Record<string, string> = {
    kuwo_search:  '🎵 酷我音乐',
    bili_search:  '📺 B站音频',
    kugou_search: '🎧 酷狗音乐',
    qq_search:    '🎤 QQ音乐',
    fav_kuwo:    '♡ 酷我收藏',
    fav_bili:    '♡ B站收藏',
    fav_kugou:   '♡ 酷狗收藏',
    fm_kugou:    '🎧 酷狗私人FM',
    local_all:   '📂 本地歌曲',
    settings:    '⚙ 设置',
  };
  return map[id] || id;
}

// ── 启动时自动签到 ─────────────────────────────
// 登录态已由 KugouLogin.vue 的 initKugouLogin() 异步初始化，
// 这里延时等待登录态确定后再静默签到
onMounted(() => {
  bindAutoSign()
  setTimeout(async () => {
    await fetchVipStatus()
    await autoSignVip()
  }, 1500)

  // 监听双击文件打开（系统文件关联 / 单实例唤醒）
  listen<string>('file_open', (event) => {
    const path = event.payload
    console.warn('[App] 收到双击文件:', path)
    // 切成本地歌曲播放
    const song: Song = {
      song_id: 'local:' + path,
      name: fileNameFromPath(path),
      singer: '',
      album: '',
      duration: 0,
      source: 'local',
    }
    activeSection.value = 'local_all'
    handlePlay(song)
  })
})

// 从路径提取文件名（去掉扩展名）
function fileNameFromPath(p: string): string {
  const parts = p.split(/[\\/]/)
  const last = parts[parts.length - 1] || p
  return last.replace(/\.[^.]+$/, '')
}


</script>

<template>
  <div class="app">
    <!-- 侧边栏 -->
    <Sidebar
      :active-section="activeSection"
      @navigate="navigate"
      @open-kugou-playlist="handleOpenKugouPlaylist"
    />

    <!-- 扫码登录弹窗（全局） -->
    <KugouLogin />

    <!-- 播放状态横幅 -->
    <div v-if="playBanner" :style="{
      position:'fixed',top:'20px',right:'20px',zIndex:999999,
      background: playBanner.ok ? '#2ecc71' : '#e74c3c',
      color:'white',padding:'12px 20px',borderRadius:'10px',
      fontSize:'14px',fontWeight:600,
      boxShadow:'0 4px 20px rgba(0,0,0,0.3)',
      maxWidth:'300px'
    }">
      <div>🎵 {{ playBanner.song }}</div>
      <div style="font-size:12px;margin-top:4px;opacity:0.9;">{{ playBanner.msg }}</div>
    </div>

    <!-- 下载状态横幅 -->
    <div v-if="dlBanner" :style="{
      position:'fixed',bottom:'20px',right:'20px',zIndex:999999,
      background: dlBanner.ok ? '#7c6af7' : '#e74c3c',
      color:'white',padding:'12px 20px',borderRadius:'10px',
      fontSize:'14px',fontWeight:600,
      boxShadow:'0 4px 20px rgba(0,0,0,0.3)',
      maxWidth:'360px'
    }">
      <div>⬇ {{ dlBanner.name }}</div>
      <div style="font-size:12px;margin-top:4px;opacity:0.9;word-break:break-all;">{{ dlBanner.msg }}</div>
    </div>

    <!-- 主面板 -->
    <main class="main-panel">
      <!-- 搜索栏 -->
      <div class="search-row">
        <SearchBar @search="handleSearch" />
      </div>

      <!-- 主内容 -->
      <div class="content">

        <!-- 酷我搜索 -->
        <template v-if="activeSection === 'kuwo_search'">
          <div class="content-header">
            <h2>{{ panelTitle(activeSection) }}</h2>
            <span class="count" v-if="results.length">共 {{ results.length }} 首</span>
          </div>
          <p v-if="loading" class="status">搜索中...</p>
          <p v-else-if="results.length === 0 && !loading && !currentKeyword" class="status hint">
            输入关键词开始搜索
          </p>
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" @download="handleDownload" />
          <p v-else-if="currentKeyword && !loading" class="status hint">未找到结果</p>
        </template>

        <!-- B站音频 -->
        <template v-else-if="activeSection === 'bili_search'">
          <div class="content-header">
            <h2>{{ panelTitle(activeSection) }}</h2>
            <span class="count" v-if="results.length">共 {{ results.length }} 首</span>
          </div>
          <p v-if="loading" class="status">搜索中...</p>
          <p v-else-if="results.length === 0 && !loading && !currentKeyword" class="status hint">
            输入关键词开始搜索 B站音频
          </p>
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" @download="handleDownload" />
          <p v-else-if="currentKeyword && !loading" class="status hint">未找到结果</p>
        </template>

        <!-- 酷狗搜索 -->
        <template v-else-if="activeSection === 'kugou_search'">
          <div class="content-header">
            <h2>{{ panelTitle(activeSection) }}</h2>
            <span class="count" v-if="results.length">共 {{ results.length }} 首</span>
          </div>
          <p v-if="loading" class="status">搜索中...</p>
          <p v-else-if="results.length === 0 && !loading && !currentKeyword" class="status hint">
            输入关键词开始搜索酷狗音乐
          </p>
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" @download="handleDownload" />
          <p v-else-if="currentKeyword && !loading" class="status hint">未找到结果</p>
        </template>

        <!-- QQ音乐（占位）-->
        <template v-else-if="activeSection === 'qq_search'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>
          <p class="status hint">QQ音乐搜索 — 开发中</p>
        </template>

        <!-- 酷狗收藏 -->
        <template v-else-if="activeSection === 'fav_kugou'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>

          <!-- 私人 FM 入口 -->
          <div class="fm-entry" @click="handleOpenFm">
            <span class="fm-entry-icon">🎧</span>
            <div class="fm-entry-body">
              <div class="fm-entry-title">私人 FM</div>
              <div class="fm-entry-desc">根据你的听歌口味，推荐你可能喜欢的歌</div>
            </div>
            <span class="fm-entry-arrow">▶</span>
          </div>

          <!-- 未进入歌单：显示歌单列表 -->
          <template v-if="!currentPlaylist">
            <p v-if="loadingLists" class="status">加载歌单中...</p>
            <p v-else-if="kugouPlaylistError" class="status hint">{{ kugouPlaylistError }}</p>
            <div v-else-if="playlists.length === 0" class="status hint">暂无歌单</div>
            <div v-else class="playlist-grid">
              <div
                v-for="pl in playlists"
                :key="pl.global_collection_id"
                class="playlist-card"
                @click="handleOpenPlaylist(pl)"
              >
                <div class="playlist-name">{{ pl.name }}</div>
                <div class="playlist-count">{{ pl.song_count }} 首</div>
              </div>
            </div>
          </template>

          <!-- 已进入歌单：显示歌曲列表 -->
          <template v-else>
            <div class="playlist-bar">
              <button class="back-btn" @click="currentPlaylist = null">← 返回歌单</button>
              <span class="playlist-title">{{ currentPlaylist.name }}</span>
              <span class="count" v-if="kugouSongs.length">共 {{ kugouSongs.length }} 首</span>
            </div>
            <p v-if="loadingSongs" class="status">加载歌曲中...</p>
            <p v-else-if="kugouPlaylistError" class="status hint">{{ kugouPlaylistError }}</p>
            <SongList v-else-if="kugouSongs.length > 0" :songs="kugouSongs" @play="handlePlayKugouFav" @download="handleDownloadKugouFav" />
            <p v-else class="status hint">该歌单暂无歌曲</p>
          </template>
        </template>

        <!-- 酷狗私人FM -->
        <template v-else-if="activeSection === 'fm_kugou'">
          <div class="content-header">
            <h2>{{ panelTitle(activeSection) }}</h2>
            <button class="refresh-btn" :disabled="fmLoading" @click="fetchPersonalFm">
              {{ fmLoading ? '推荐中...' : '🔄 换一批' }}
            </button>
          </div>
          <p v-if="fmLoading" class="status">正在推荐...</p>
          <p v-else-if="fmError" class="status hint">{{ fmError }}</p>
          <SongList v-else-if="fmSongs.length > 0" :songs="fmSongs.map(fmSongToSong)" @play="handlePlayFm" @download="handleDownloadFm" />
          <p v-else class="status hint">暂无推荐，点击右上角换一批</p>
        </template>

        <!-- 酷我收藏 -->
        <template v-else-if="activeSection === 'fav_kuwo'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>
          <p class="status hint">收藏功能 — 开发中</p>
        </template>

        <!-- B站收藏 -->
        <template v-else-if="activeSection === 'fav_bili'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>
          <p class="status hint">收藏功能 — 开发中</p>
        </template>

        <!-- 本地歌曲 -->
        <template v-else-if="activeSection === 'local_all'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>

          <!-- 未设置音乐库目录 -->
          <template v-if="!libraryRoot">
            <div class="local-empty">
              <p class="status hint">还没有设置音乐库目录</p>
              <p class="status hint" style="margin-top:-12px">
                你可以先在任意音源搜索并点「⬇ 下载」，选择下载目录后，这里会自动成为音乐库根目录
              </p>
            </div>
          </template>

          <!-- 已设置目录，但还没扫描 -->
          <template v-else-if="!libraryTree">
            <div class="local-empty">
              <p class="status hint">音乐库：{{ libraryRoot }}</p>
              <button class="scan-btn" @click="scanLibrary()">📂 扫描音乐库</button>
            </div>
          </template>

          <!-- 已扫描：左侧文件夹树 + 右侧歌曲列表 -->
          <template v-else>
            <div class="local-layout">
              <!-- 文件夹树 -->
              <div class="folder-tree">
                <div class="folder-tree-header">📁 文件夹</div>
                <FolderNode
                  :node="libraryTree"
                  :depth="0"
                  :current="currentFolder"
                  @open="handleOpenFolder"
                />
              </div>

              <!-- 当前文件夹歌曲 -->
              <div class="folder-songs">
                <template v-if="!currentFolder">
                  <p class="status hint">点击左侧文件夹查看歌曲</p>
                </template>
                <template v-else>
                  <div class="folder-title">
                    📁 {{ currentFolder.name }}
                    <span class="count" v-if="currentFolder.songs?.length">共 {{ currentFolder.songs.length }} 首</span>
                  </div>
                  <SongList
                    v-if="currentFolder.songs?.length"
                    :songs="currentFolder.songs.map(localSongToSong)"
                    @play="handlePlay"
                  />
                  <p v-else class="status hint">该文件夹暂无歌曲</p>
                </template>
              </div>
            </div>
          </template>
        </template>

        <!-- 设置 -->
        <template v-else-if="activeSection === 'settings'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>
          <div class="settings-panel">
            <div class="setting-item">
              <span class="label">播放条窗口</span>
              <button>打开</button>
            </div>

            <!-- 酷狗 VIP 状态 -->
            <div class="setting-item vip-setting">
              <span class="label">酷狗 VIP</span>
              <div class="vip-setting-body">
                <span v-if="isSignedToday" class="vip-status signed">✅ 今日已签到</span>
                <span v-else class="vip-status unsigned">⏳ 今日未签到</span>
                <span v-if="vipStatus?.tvip?.vip_end_time" class="vip-status expire">
                  到期 {{ vipStatus.tvip.vip_end_time }}
                </span>
                <button class="vip-btn" :disabled="signLoading" @click="signVip">
                  {{ signLoading ? '签到中...' : '手动签到' }}
                </button>
                <button class="vip-btn ad-btn" :disabled="adLoading" @click="watchAd">
                  {{ adLoading ? '看广告中...' : '📺 看广告领时长' }}
                </button>
              </div>
            </div>
            <div class="vip-msg" v-if="signSuccess" style="color:#2ecc71">{{ signSuccess }}</div>
            <div class="vip-msg" v-if="signError" style="color:#ff6b6b">{{ signError }}</div>
            <div class="vip-msg" v-if="adResult" style="color:#2ecc71">{{ adResult }}</div>
            <div class="vip-msg" v-if="adError" style="color:#ff6b6b">{{ adError }}</div>
          </div>
        </template>

      </div>
    </main>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  width: 100vw;
  height: 100vh;
  background: #0f0f1a;
  color: #e0e0e0;
  font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
}

.main-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.search-row {
  padding: 12px 20px;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  flex-shrink: 0;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

.content-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.content-header h2 {
  font-size: 16px;
  font-weight: 600;
  color: white;
}
.count {
  font-size: 12px;
  color: rgba(255,255,255,0.4);
}

.status {
  text-align: center;
  color: rgba(255,255,255,0.4);
  margin: 40px 0;
}
.status.hint { margin: 20px 0; font-size: 13px; }

.settings-panel {
  font-size: 13px;
  color: rgba(255,255,255,0.7);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* ── 歌单列表 ────────────────────────────────── */
.vip-btn {
  background: rgba(255,193,77,0.9);
  border: none;
  border-radius: 8px;
  padding: 8px 16px;
  color: #3a2a00;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}
.vip-btn:hover:not(:disabled) {
  background: #ffc14d;
  transform: translateY(-1px);
}
.vip-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.vip-msg {
  font-size: 12px;
  margin-top: 8px;
}

/* 看广告领时长按钮 */
.vip-btn.ad-btn {
  background: rgba(124, 106, 247, 0.25);
  color: #a89cff;
}
.vip-btn.ad-btn:hover:not(:disabled) {
  background: rgba(124, 106, 247, 0.4);
}

/* 私人 FM 入口卡片 */
.fm-entry {
  display: flex;
  align-items: center;
  gap: 14px;
  background: linear-gradient(135deg, rgba(124,106,247,0.15), rgba(124,106,247,0.05));
  border: 1px solid rgba(124,106,247,0.35);
  border-radius: 12px;
  padding: 16px 18px;
  margin-bottom: 18px;
  cursor: pointer;
  transition: all 0.15s;
}
.fm-entry:hover {
  background: linear-gradient(135deg, rgba(124,106,247,0.25), rgba(124,106,247,0.1));
  border-color: rgba(124,106,247,0.6);
  transform: translateY(-1px);
}
.fm-entry-icon {
  font-size: 26px;
}
.fm-entry-body {
  flex: 1;
}
.fm-entry-title {
  font-size: 15px;
  font-weight: 700;
  color: white;
}
.fm-entry-desc {
  font-size: 12px;
  color: rgba(255,255,255,0.45);
  margin-top: 2px;
}
.fm-entry-arrow {
  color: rgba(124,106,247,0.7);
  font-size: 14px;
}

/* 刷新按钮 */
.refresh-btn {
  background: rgba(124,106,247,0.2);
  border: 1px solid rgba(124,106,247,0.4);
  color: #a89cff;
  padding: 4px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s;
  margin-left: auto;
}
.refresh-btn:hover:not(:disabled) {
  background: rgba(124,106,247,0.35);
}
.refresh-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.playlist-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}
.playlist-card {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 10px;
  padding: 18px 16px;
  cursor: pointer;
  transition: all 0.15s;
}
.playlist-card:hover {
  background: rgba(124,106,247,0.15);
  border-color: rgba(124,106,247,0.4);
}
.playlist-name {
  font-size: 14px;
  font-weight: 600;
  color: white;
  margin-bottom: 6px;
}
.playlist-count {
  font-size: 12px;
  color: rgba(255,255,255,0.4);
}
.playlist-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.back-btn {
  background: rgba(124,106,247,0.2);
  border: 1px solid rgba(124,106,247,0.4);
  color: #a89cff;
  padding: 4px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.back-btn:hover {
  background: rgba(124,106,247,0.35);
}
.playlist-title {
  font-size: 14px;
  font-weight: 600;
  color: white;
}
.setting-item {
  display: flex;
  align-items: center;
  gap: 12px;
}
.setting-item .label { color: rgba(255,255,255,0.6); }
.setting-item button {
  background: rgba(124,106,247,0.2);
  border: 1px solid rgba(124,106,247,0.4);
  color: #a89cff;
  padding: 4px 14px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s;
}
.setting-item button:hover { background: rgba(124,106,247,0.35); }

/* 设置页 VIP 状态 */
.vip-setting {
  border-top: 1px solid rgba(255,255,255,0.06);
  padding-top: 14px;
}
.vip-setting-body {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.vip-status {
  font-size: 13px;
  font-weight: 600;
}
.vip-status.signed { color: #2ecc71; }
.vip-status.unsigned { color: #ffc14d; }
.vip-status.expire {
  font-size: 12px;
  font-weight: 400;
  color: rgba(255,255,255,0.5);
}

/* 本地音乐库 */
.local-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px 20px;
}
.scan-btn {
  background: rgba(124,106,247,0.2);
  border: 1px solid rgba(124,106,247,0.4);
  color: #a89cff;
  padding: 8px 20px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.15s;
}
.scan-btn:hover {
  background: rgba(124,106,247,0.35);
}
.local-layout {
  display: flex;
  gap: 16px;
  min-height: 400px;
}
.folder-tree {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid rgba(255,255,255,0.06);
  padding-right: 12px;
  overflow-y: auto;
  max-height: 70vh;
}
.folder-tree-header {
  font-size: 12px;
  font-weight: 600;
  color: rgba(255,255,255,0.4);
  padding: 4px 8px 8px;
}
.folder-songs {
  flex: 1;
  min-width: 0;
}
.folder-title {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  font-weight: 600;
  color: white;
  margin-bottom: 12px;
}

/* ── 登录卡片 ────────────────────────────────── */
.login-card {
  max-width: 380px;
  margin: 24px auto;
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 12px;
  padding: 32px 28px;
}
.login-desc {
  font-size: 13px;
  color: rgba(255,255,255,0.4);
  text-align: center;
  margin: 0 0 24px;
}
.login-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.login-input {
  background: rgba(255,255,255,0.06);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 8px;
  padding: 10px 14px;
  color: white;
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s;
}
.login-input:focus {
  border-color: rgba(124,106,247,0.6);
}
.login-input::placeholder {
  color: rgba(255,255,255,0.25);
}
.login-error {
  font-size: 12px;
  color: #ff6b6b;
  margin: 0;
  text-align: center;
}
.login-btn {
  background: rgba(124,106,247,0.8);
  border: none;
  border-radius: 8px;
  padding: 11px;
  color: white;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s;
  margin-top: 4px;
}
.login-btn:hover:not(:disabled) {
  background: rgba(124,106,247,1);
}
.login-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
