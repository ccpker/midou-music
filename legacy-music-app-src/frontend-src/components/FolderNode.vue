<template>
  <div class="folder-node" :style="{ paddingLeft: depth * 16 + 'px' }">
    <!-- folder header -->
    <div class="folder-header" @click="expanded = !expanded">
      <span class="folder-arrow">{{ expanded ? '▼' : '▶' }}</span>
      <span class="folder-icon">📁</span>
      <span class="folder-name">{{ folder.name }}</span>
      <span class="folder-count">({{ folder.songs?.length || folder.song_ids?.length || 0 }})</span>
    </div>

    <!-- folder songs -->
    <div v-if="expanded" class="folder-children">
      <div
        v-for="song in displaySongs"
        :key="song.music_id"
        class="folder-song-item"
        @contextmenu.prevent="onContextMenu(song.music_id, $event)"
      >
        <div class="song-info" @click="$emit('play', song)">
          <div class="song-title">{{ song.title }}</div>
          <div class="song-meta">{{ song.artist || '未知' }} · {{ song.source }}</div>
        </div>
        <div class="song-actions" @click.stop>
          <button
            class="trash-btn"
            title="删除"
            @click="$emit('move-to-trash', song.music_id)"
          >🗑</button>
        </div>
      </div>

      <!-- sub-folders (recursive) -->
      <FolderNode
        v-for="sub in subFolders"
        :key="sub.id"
        :folder="sub"
        :depth="depth + 1"
        @play="$emit('play', $event)"
        @move-to-trash="$emit('move-to-trash', $event)"
        @move-to-folder="$emit('move-to-folder', $event[0], $event[1])"
      />

      <div v-if="displaySongs.length === 0 && subFolders.length === 0" class="empty-folder">
        空文件夹
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useLibraryStore } from '@/stores/library'

const props = defineProps({
  folder: { type: Object, required: true },
  depth: { type: Number, default: 0 },
})

const emit = defineEmits(['play', 'move-to-trash', 'move-to-folder'])

const libStore = useLibraryStore()
const expanded = ref(props.depth === 0)

// Display songs — either inline `songs` array (from get_active_library) or lookup by song_ids
const displaySongs = computed(() => {
  if (props.folder.songs && props.folder.songs.length > 0) {
    return props.folder.songs
  }
  // fallback: lookup from all library folders
  return (props.folder.song_ids || [])
    .map(id => {
      for (const f of libStore.folders) {
        const found = (f.songs || []).find(s => s.music_id === id)
        if (found) return found
      }
      return null
    })
    .filter(Boolean)
})

const subFolders = computed(() => {
  return libStore.folders.filter(f => f.parent_id === props.folder.id)
})

function onContextMenu(musicId, event) {
  // Pass up to parent with musicId + event for positioning
  // Since Vue emit doesn't support multiple args well on nested,
  // we emit with the parent-id pattern
  emit('move-to-folder', [musicId, event])
}
</script>

<script>
export default {
  name: 'FolderNode',
}
</script>

<style scoped>
.folder-node {
  user-select: none;
}

.folder-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background 0.15s;
}

.folder-header:hover {
  background: var(--hover);
}

.folder-arrow {
  font-size: 10px;
  color: var(--muted);
  width: 12px;
  text-align: center;
}

.folder-icon {
  font-size: 14px;
}

.folder-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.folder-count {
  font-size: 11px;
  color: var(--muted);
}

.folder-children {
  padding-left: 20px;
}

.folder-song-item {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  margin: 2px 0;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background 0.15s;
  gap: 8px;
}

.folder-song-item:hover {
  background: var(--hover);
}

.song-info {
  flex: 1;
  min-width: 0;
}

.song-title {
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text);
}

.song-meta {
  font-size: 10px;
  color: var(--muted);
}

.song-actions {
  flex-shrink: 0;
}

.trash-btn {
  padding: 2px 4px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
  opacity: 0;
  transition: opacity 0.15s;
}

.folder-song-item:hover .trash-btn {
  opacity: 1;
}

.trash-btn:hover {
  color: #e94560;
}

.empty-folder {
  font-size: 11px;
  color: var(--muted);
  padding: 8px;
  font-style: italic;
}
</style>
