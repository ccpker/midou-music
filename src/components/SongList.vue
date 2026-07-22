<script setup lang="ts">
/**
 * 组件: SongList.vue
 * 功能: 歌曲搜索结果列表
 * props: { songs: Song[] }
 * emits: ["play"] — 携带 song: Song
 * 依赖: 无
 */
import { computed } from 'vue';

interface Song {
  song_id: string;
  name: string;
  singer: string;
  album: string;
  source: string;
  duration?: number;
}

const props = defineProps<{
  songs: Song[];
}>();

const emit = defineEmits<{
  (e: 'play', song: Song): void;
}>();

function fmtDur(sec?: number): string {
  if (!sec || sec <= 0) return '';
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

const sourceBadge = (s: string) => {
  const map: Record<string, string> = { kuwo: '♬', kugou: '🎵', bilibili: '📺', qq: '🐧' };
  return map[s] || s;
};
</script>

<template>
  <div class="song-list">
    <div
      v-for="song in songs"
      :key="song.song_id"
      class="song-item"
      @click="emit('play', song)"
    >
      <div class="song-info">
        <span class="song-name">
          <span class="source-badge">{{ sourceBadge(song.source) }}</span>
          {{ song.name }}
        </span>
        <span class="song-meta">
          {{ song.singer }} · {{ song.album }}
          <span v-if="song.duration" class="song-dur">{{ fmtDur(song.duration) }}</span>
        </span>
      </div>
    </div>
    <p v-if="songs.length === 0" class="empty-hint">暂无结果</p>
  </div>
</template>

<style scoped>
.song-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.song-item {
  padding: 12px 16px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}

.song-item:hover {
  background: var(--color-surface);
}

.song-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.song-name {
  font-size: 16px;
  font-weight: 500;
}

.source-badge {
  margin-right: 6px;
}

.song-meta {
  font-size: 13px;
  color: var(--color-muted);
}

.song-dur {
  margin-left: 8px;
}

.empty-hint {
  text-align: center;
  color: var(--color-muted);
  padding: 40px 0;
}
</style>
