<script setup lang="ts">
/**
 * 组件: FolderNode.vue（递归）
 * 功能: 本地音乐库的文件夹树节点
 * props: { node: LibraryNode, depth: number, current: LibraryNode|null }
 * emits: ["open"] — 携带 node
 */
import { ref } from 'vue';
import type { LibraryNode } from '../composables/useLibrary';

const props = defineProps<{
  node: LibraryNode;
  depth: number;
  current: LibraryNode | null;
}>();

const emit = defineEmits<{
  (e: 'open', node: LibraryNode): void;
}>();

const expanded = ref(false);

function toggle() {
  expanded.value = !expanded.value;
}

function onOpen() {
  emit('open', props.node);
}
</script>

<template>
  <div class="folder-node">
    <div
      class="folder-row"
      :class="{ active: current?.path === node.path }"
      :style="{ paddingLeft: (depth * 16 + 8) + 'px' }"
      @click="onOpen"
    >
      <span
        v-if="node.children?.length"
        class="folder-toggle"
        @click.stop="toggle"
      >{{ expanded ? '▾' : '▸' }}</span>
      <span v-else class="folder-toggle placeholder"></span>
      <span class="folder-icon">📁</span>
      <span class="folder-name">{{ node.name || '根目录' }}</span>
      <span class="folder-count" v-if="node.songs?.length">{{ node.songs.length }}</span>
    </div>

    <!-- 递归子目录 -->
    <template v-if="expanded && node.children?.length">
      <FolderNode
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :current="current"
        @open="emit('open', $event)"
      />
    </template>
  </div>
</template>

<style scoped>
.folder-node {
  user-select: none;
}
.folder-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  font-size: 13px;
  color: rgba(255,255,255,0.7);
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.12s;
  white-space: nowrap;
  overflow: hidden;
}
.folder-row:hover {
  background: rgba(255,255,255,0.06);
  color: white;
}
.folder-row.active {
  background: rgba(124,106,247,0.18);
  color: #a89cff;
}
.folder-toggle {
  width: 14px;
  font-size: 10px;
  opacity: 0.5;
  flex-shrink: 0;
  text-align: center;
}
.folder-toggle.placeholder {
  visibility: hidden;
}
.folder-icon {
  flex-shrink: 0;
  font-size: 13px;
}
.folder-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.folder-count {
  font-size: 11px;
  color: rgba(255,255,255,0.35);
  flex-shrink: 0;
}
</style>
