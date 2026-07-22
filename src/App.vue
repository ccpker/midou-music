<script setup lang="ts">
/**
 * 组件: App.vue
 * 功能: 应用根组件 — 搜索框 + 歌曲列表 + 播放器
 * props: 无
 * emits: 无
 * 依赖: @tauri-apps/api invoke, stores/player.ts
 */
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import SearchBar from './components/SearchBar.vue';
import SongList from './components/SongList.vue';

interface Song {
  song_id: string;
  name: string;
  singer: string;
  album: string;
  source: string;
  duration?: number;
}

const songs = ref<Song[]>([]);
const loading = ref(false);
const currentUrl = ref('');

async function handleSearch(keyword: string) {
  loading.value = true;
  try {
    songs.value = await invoke<Song[]>('search', { keyword });
  } catch (e) {
    console.error('搜索失败:', e);
    songs.value = [];
  } finally {
    loading.value = false;
  }
}

async function handlePlay(song: Song) {
  try {
    const result = await invoke<{ url: string; source: string }>('play_url', {
      songId: song.song_id,
    });
    currentUrl.value = result.url;
  } catch (e) {
    console.error('获取播放地址失败:', e);
  }
}
</script>

<template>
  <div class="app-container">
    <h1 class="app-title">🎵 米豆音乐</h1>
    <SearchBar @search="handleSearch" />
    <p v-if="loading" class="status">搜索中...</p>
    <SongList :songs="songs" @play="handlePlay" />
    <audio
      v-if="currentUrl"
      :src="currentUrl"
      controls
      autoplay
      class="audio-player"
    />
  </div>
</template>

<style scoped>
.app-container {
  max-width: 800px;
  margin: 0 auto;
  padding: 24px;
}

.app-title {
  text-align: center;
  font-size: 28px;
  margin-bottom: 24px;
  color: var(--color-accent);
}

.status {
  text-align: center;
  color: var(--color-muted);
  margin: 12px 0;
}

.audio-player {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  width: 100%;
  background: var(--color-surface);
  border-top: 1px solid var(--color-border);
}
</style>
