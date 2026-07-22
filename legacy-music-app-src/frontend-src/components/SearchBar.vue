<template>
  <header class="top-bar">
    <!-- Search input row -->
    <div class="search-box">
      <input
        v-model="keyword"
        type="text"
        placeholder="搜索歌曲、歌手..."
        @keypress.enter="doSearch"
      />
      <button @click="doSearch">搜索</button>
    </div>

    <!-- Current source indicator -->
    <div class="source-display">
      <span class="source-name">{{ store.sourceDisplay.name }}</span>
      <small class="source-desc">{{ store.sourceDisplay.desc }}</small>
    </div>
  </header>
</template>

<script setup>
import { ref } from 'vue'
import { usePlayerStore } from '@/stores/player'

const store = usePlayerStore()
const keyword = ref('')

function doSearch() {
  store.search(keyword.value)
}
</script>

<style scoped>
.top-bar {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--card);
  flex-shrink: 0;
}

.search-box {
  display: flex;
  gap: 10px;
  margin-bottom: 8px;
}

.search-box input {
  flex: 1;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--text);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s;
}

.search-box input:focus {
  border-color: var(--accent);
}

.search-box input::placeholder {
  color: var(--muted);
}

.search-box button {
  padding: 10px 20px;
  border: none;
  border-radius: var(--radius);
  background: var(--accent);
  color: #fff;
  cursor: pointer;
  font-size: 14px;
  transition: opacity 0.2s;
}

.search-box button:hover {
  opacity: 0.85;
}

.source-display {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 12px;
}

.source-name {
  color: var(--accent);
  font-weight: 500;
}

.source-desc {
  color: var(--muted);
}
</style>
