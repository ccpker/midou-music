<script setup lang="ts">
/**
 * 组件: SearchBar.vue
 * 功能: 搜索输入框 + 搜索按钮
 * props: 无
 * emits: ["search"] — 携带 keyword: string
 * 依赖: 无
 */
import { ref } from 'vue';

const keyword = ref('');
const emit = defineEmits<{
  (e: 'search', keyword: string): void;
}>();

function submit() {
  const kw = keyword.value.trim();
  if (kw) emit('search', kw);
}
</script>

<template>
  <div class="search-bar">
    <input
      v-model="keyword"
      type="text"
      placeholder="搜索歌曲..."
      class="search-input"
      @keydown.enter="submit"
    />
    <button class="search-btn" @click="submit">搜索</button>
  </div>
</template>

<style scoped>
.search-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 10px 16px;
  font-size: 16px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
  color: var(--color-text);
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: var(--color-accent);
}

.search-btn {
  padding: 10px 24px;
  font-size: 16px;
  background: var(--color-accent);
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: opacity 0.2s;
}

.search-btn:hover {
  opacity: 0.85;
}
</style>
