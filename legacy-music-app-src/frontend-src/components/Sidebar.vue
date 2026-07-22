<template>
  <aside class="sidebar">
    <!-- Logo -->
    <div class="logo">音楽自由</div>

    <!-- Source tabs -->
    <nav class="source-nav">
      <button
        v-for="(cfg, key) in SOURCE_CONFIG"
        :key="key"
        class="source-btn"
        :class="{ active: store.currentSource === key }"
        @click="store.setSource(key)"
      >
        {{ cfg.name.replace('音乐', '') }}
      </button>
    </nav>

    <!-- Bottom nav -->
    <div class="nav-bottom">
      <button
        v-for="nav in NAV_ITEMS"
        :key="nav.key"
        class="nav-btn"
        :class="{ active: store.currentView === nav.key }"
        @click="store.setView(nav.key)"
      >
        {{ nav.label }}
      </button>
    </div>
  </aside>
</template>

<script setup>
import { usePlayerStore, SOURCE_CONFIG } from '@/stores/player'

const store = usePlayerStore()

const NAV_ITEMS = [
  { key: 'search',   label: '搜索' },
  { key: 'queue',    label: '队列' },
  { key: 'fav',      label: '收藏' },
  { key: 'history',  label: '历史' },
  { key: 'local',    label: '本地' },
  { key: 'settings', label: '设置' },
]
</script>

<style scoped>
.sidebar {
  width: 80px;
  min-width: 80px;
  background: var(--card);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 12px 8px;
  height: 100vh;
  overflow: hidden;
}

.logo {
  font-size: 11px;
  color: var(--accent);
  text-align: center;
  margin-bottom: 16px;
  font-weight: bold;
}

.source-nav {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
  overflow-y: auto;
}

.source-btn {
  padding: 10px 4px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
  text-align: center;
  white-space: nowrap;
}

.source-btn:hover,
.source-btn.active {
  background: var(--accent);
  color: #fff;
}

.nav-bottom {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: auto;
}

.nav-btn {
  padding: 8px 4px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-size: 11px;
  transition: background 0.2s;
}

.nav-btn:hover,
.nav-btn.active {
  background: var(--hover);
  color: var(--text);
}
</style>
