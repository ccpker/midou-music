<script setup lang="ts">
/**
 * 组件: App.vue（主窗口根组件）
 * 功能: 侧边栏 + 主面板 + 搜索/播放入口编排
 * 标注: 本文件只做 UI 编排，具体逻辑委托给 composables
 */
import { ref } from 'vue';
import Sidebar from './components/Sidebar.vue';
import SearchBar from './components/SearchBar.vue';
import SongList from './components/SongList.vue';
import KugouLogin from './components/KugouLogin.vue';
import { playSong, type Song } from './composables/usePlayer';
import { search, useSearch } from './composables/useSearch';
import { useKugouPlaylist, type KugouPlaylistSong, type KugouPlaylist } from './composables/useKugouPlaylist';
import { vipStatus, signLoading, signError, signSuccess, fetchVipStatus, signVip } from './composables/useKugouVip';

const { results, loading } = useSearch();

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
  // 进入酷狗相关页面时自动拉取歌单列表 + VIP 状态
  if (section === 'fav_kugou') {
    fetchPlaylists();
    fetchVipStatus();
  }
  if (section === 'kugou_search') {
    fetchVipStatus();
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
    local_all:   '📂 本地歌曲',
    settings:    '⚙ 设置',
  };
  return map[id] || id;
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
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" />
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
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" />
          <p v-else-if="currentKeyword && !loading" class="status hint">未找到结果</p>
        </template>

        <!-- 酷狗搜索 -->
        <template v-else-if="activeSection === 'kugou_search'">
          <div class="content-header">
            <h2>{{ panelTitle(activeSection) }}</h2>
            <span class="count" v-if="results.length">共 {{ results.length }} 首</span>
          </div>

          <!-- VIP 签到卡片 -->
          <div class="vip-card">
            <div class="vip-card-row">
              <div class="vip-info">
                <span class="vip-badge" v-if="vipStatus?.tvip">👑 概念版畅听 VIP</span>
                <span class="vip-badge none" v-else>未领 VIP</span>
                <span class="vip-expire" v-if="vipStatus?.tvip?.vip_end_time">
                  到期 {{ vipStatus.tvip.vip_end_time }}
                </span>
              </div>
              <button
                class="vip-btn"
                :disabled="signLoading"
                @click="signVip"
              >
                {{ signLoading ? '签到中...' : '🎁 签到领 VIP' }}
              </button>
            </div>
            <div class="vip-msg" v-if="signSuccess" style="color:#2ecc71">{{ signSuccess }}</div>
            <div class="vip-msg" v-if="signError" style="color:#ff6b6b">{{ signError }}</div>
          </div>

          <p v-if="loading" class="status">搜索中...</p>
          <p v-else-if="results.length === 0 && !loading && !currentKeyword" class="status hint">
            输入关键词开始搜索酷狗音乐
          </p>
          <SongList v-else-if="results.length > 0" :songs="results" @play="handlePlay" />
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

          <!-- VIP 签到卡片 -->
          <div class="vip-card">
            <div class="vip-card-row">
              <div class="vip-info">
                <span class="vip-badge" v-if="vipStatus?.tvip">👑 概念版畅听 VIP</span>
                <span class="vip-badge none" v-else>未领 VIP</span>
                <span class="vip-expire" v-if="vipStatus?.tvip?.vip_end_time">
                  到期 {{ vipStatus.tvip.vip_end_time }}
                </span>
              </div>
              <button
                class="vip-btn"
                :disabled="signLoading"
                @click="signVip"
              >
                {{ signLoading ? '签到中...' : '🎁 签到领 VIP' }}
              </button>
            </div>
            <div class="vip-msg" v-if="signSuccess" style="color:#2ecc71">{{ signSuccess }}</div>
            <div class="vip-msg" v-if="signError" style="color:#ff6b6b">{{ signError }}</div>
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
            <SongList v-else-if="kugouSongs.length > 0" :songs="kugouSongs" @play="handlePlayKugouFav" />
            <p v-else class="status hint">该歌单暂无歌曲</p>
          </template>
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
          <p class="status hint">本地歌曲 — 开发中</p>
        </template>

        <!-- 设置 -->
        <template v-else-if="activeSection === 'settings'">
          <div class="content-header"><h2>{{ panelTitle(activeSection) }}</h2></div>
          <div class="settings-panel">
            <div class="setting-item">
              <span class="label">播放条窗口</span>
              <button>打开</button>
            </div>
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
.vip-card {
  background: linear-gradient(135deg, rgba(124,106,247,0.15), rgba(255,170,80,0.1));
  border: 1px solid rgba(124,106,247,0.35);
  border-radius: 12px;
  padding: 14px 18px;
  margin-bottom: 16px;
}
.vip-card-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.vip-info {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.vip-badge {
  font-size: 13px;
  font-weight: 700;
  color: #ffc14d;
}
.vip-badge.none {
  color: rgba(255,255,255,0.45);
  font-weight: 500;
}
.vip-expire {
  font-size: 12px;
  color: rgba(255,255,255,0.5);
}
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
