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

function onClick(song: Song, event: MouseEvent) {
  // 强制视觉反馈
  const target = event.currentTarget as HTMLElement;
  target.style.background = 'rgba(46, 204, 113, 0.3)';
  setTimeout(() => {
    target.style.background = '';
  }, 200);
  
  emit('play', song);
}

function fmtDur(sec?: number): string {
  if (!sec || sec <= 0) return '--:--';
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
    >
      <button 
        class="play-btn"
        @click="onClick(song, $event)"
        :title="'播放: ' + song.name"
      >▶</button>
      <div class="song-info" @click="onClick(song, $event)">
        <span class="song-name">
          <span class="source-badge">{{ sourceBadge(song.source) }}</span>
          {{ song.name }}
        </span>
        <span class="song-meta">
          {{ song.singer }} · {{ song.album }}
          <span class="song-dur">{{ fmtDur(song.duration) }}</span>
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
  background: rgba(255,255,255,0.02);
  border: 1px solid transparent;
  margin: 2px 0;
}

.song-item:hover {
  background: rgba(255,255,255,0.08);
  border-color: rgba(255,255,255,0.1);
}

.song-item:active {
  background: rgba(255,255,255,0.15);
  transform: scale(0.995);
}

.play-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: #2ecc71;
  color: white;
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-right: 12px;
}

.play-btn:hover {
  background: #27ae60;
  transform: scale(1.1);
}

.play-btn:active {
  transform: scale(0.95);
}

.song-item {
  display: flex;
  align-items: center;
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
