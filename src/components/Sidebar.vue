<script setup lang="ts">
/**
 * 组件: Sidebar.vue
 * 功能: 左侧导航侧边栏（v0.4 网络收藏取消一级，收藏并入各平台）
 * 标注: 纯展示，emit 事件驱动 App.vue 更新 activeSection
 */
import { ref } from 'vue'
import { isLoggedIn as kugouLoggedIn, startQrLogin } from '../composables/useKugouLogin'
import { playlists, fetchPlaylists, currentPlaylist, type KugouPlaylist } from '../composables/useKugouPlaylist'
import type { PropType } from 'vue'

defineProps({
  activeSection: { type: String as PropType<string>, required: true },
});

const emit = defineEmits<{
  (e: 'navigate', section: string): void;
  (e: 'openKugouPlaylist', playlist: KugouPlaylist): void;
}>();

// 展开状态
const expandedPlatform = ref<string | null>(null)

function toggleExpand(id: string) {
  expandedPlatform.value = expandedPlatform.value === id ? null : id
  // 点击音源一级时同时切换到对应搜索面板
  if (['kuwo_search','bili_search','kugou_search'].includes(id)) {
    emit('navigate', id)
  }
  // 展开酷狗且已登录时拉歌单列表
  if (id === 'kugou_search' && kugouLoggedIn.value && playlists.value.length === 0) {
    fetchPlaylists()
  }
}

function handleKugouLogin() {
  console.warn('[Sidebar] 点击扫码登录')
  startQrLogin()
}

// ── 菜单结构 ─────────────────────────────────────

const sources = [
  { id: 'kuwo_search',  label: '酷我音乐', icon: '♬' },
  { id: 'bili_search',  label: 'B站音频',  icon: '📺' },
  { id: 'kugou_search', label: '酷狗',     icon: '🎵' },
  { id: 'qq_search',    label: 'QQ音乐',   icon: '🎤', disabled: true },
];

const local = [
  { id: 'local_all', label: '全部歌曲' },
];
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-title">🎵 米豆音乐</div>

    <!-- 音源区 -->
    <div class="sidebar-section">
      <div class="section-label">音源</div>

      <div v-for="s in sources" :key="s.id">
        <!-- 一级：平台名（可点击切换搜索 + 展开子菜单） -->
        <div
          class="sidebar-item primary"
          :class="{ active: activeSection === s.id || activeSection.startsWith(s.id), disabled: s.disabled }"
          @click="!s.disabled && toggleExpand(s.id)"
        >
          <span class="icon">{{ s.icon }}</span>{{ s.label }}
          <span v-if="!s.disabled" class="expand-arrow">{{ expandedPlatform === s.id ? '▼' : '▶' }}</span>
        </div>

        <!-- 二级：收藏 / 登录 -->
        <div v-if="expandedPlatform === s.id && !s.disabled" class="sidebar-children">
          <!-- 酷狗：未登录显示登录入口 -->
          <div
            v-if="s.id === 'kugou_search' && !kugouLoggedIn"
            class="sidebar-item child kugou-login"
            @click.stop="handleKugouLogin"
          >
            <span class="sep">🔑</span>扫码登录
          </div>
          <!-- 酷狗：已登录显示收藏 + 歌单三级列表 -->
          <template v-if="s.id === 'kugou_search' && kugouLoggedIn">
            <!-- 我的收藏（入口） -->
            <div
              class="sidebar-item child"
              :class="{ active: activeSection === 'fav_kugou' && !currentPlaylist }"
              @click="emit('navigate', 'fav_kugou')"
            >
              <span class="sep">♡</span>我的收藏
            </div>
            <!-- 歌单子分类 -->
            <div
              v-for="pl in playlists"
              :key="pl.global_collection_id"
              class="sidebar-item child sub-playlist"
              :class="{ active: currentPlaylist?.global_collection_id === pl.global_collection_id }"
              @click="emit('openKugouPlaylist', pl)"
            >
              <span class="sep">♪</span>{{ pl.name }}
            </div>
          </template>
          <!-- 其他平台：收藏 -->
          <div
            v-if="s.id !== 'kugou_search'"
            class="sidebar-item child"
            :class="{ active: activeSection === `fav_${s.id.replace('_search','')}` }"
            @click="emit('navigate', `fav_${s.id.replace('_search','')}`)"
          >
            <span class="sep">♡</span>我的收藏
          </div>
        </div>
      </div>
    </div>

    <div class="sidebar-divider" />

    <!-- 本地区 -->
    <div class="sidebar-section">
      <div class="section-label">本地</div>
      <div
        v-for="s in local"
        :key="s.id"
        class="sidebar-item sub"
        :class="{ active: activeSection === s.id }"
        @click="emit('navigate', s.id)"
      >
        <span class="sep">📂</span>{{ s.label }}
      </div>
    </div>

    <div class="sidebar-spacer" />

    <!-- 设置 -->
    <div class="sidebar-section">
      <div
        class="sidebar-item"
        :class="{ active: activeSection === 'settings' }"
        @click="emit('navigate', 'settings')"
      >
        ⚙ 设置
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 200px;
  flex-shrink: 0;
  background: #1a1a2e;
  border-right: 1px solid rgba(255,255,255,0.06);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.sidebar-title {
  padding: 16px 16px 12px;
  font-size: 15px;
  font-weight: 700;
  color: white;
}

.sidebar-section {
  padding: 4px 0 6px;
}

.section-label {
  font-size: 10px;
  font-weight: 600;
  color: rgba(255,255,255,0.3);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 6px 16px 2px;
}

.sidebar-item {
  padding: 7px 16px;
  font-size: 13px;
  cursor: pointer;
  color: rgba(255,255,255,0.7);
  transition: all 0.12s;
  display: flex;
  align-items: center;
  gap: 6px;
  user-select: none;
}
.sidebar-item:hover {
  background: rgba(255,255,255,0.06);
  color: white;
}
.sidebar-item.active {
  background: rgba(124, 106, 247, 0.18);
  color: #a89cff;
}
.sidebar-item.primary {
  font-weight: 600;
  color: rgba(255,255,255,0.85);
}
.sidebar-item.sub {
  font-size: 12px;
  padding-left: 28px;
  color: rgba(255,255,255,0.45);
}
.sidebar-item .icon { font-size: 12px; }
.sidebar-item .sep { color: rgba(255,255,255,0.3); font-size: 11px; }

.expand-arrow {
  margin-left: auto;
  font-size: 9px;
  opacity: 0.4;
}

.sidebar-children {
  overflow: hidden;
}

.sidebar-item.child {
  font-size: 12px;
  padding-left: 36px;
  color: rgba(255,255,255,0.5);
}
.sidebar-item.child:hover {
  color: rgba(255,255,255,0.85);
}
.sidebar-item.child.active {
  color: #a89cff;
}

.sidebar-item.kugou-login {
  color: rgba(255, 200, 80, 0.7);
}
.sidebar-item.kugou-login:hover {
  color: rgba(255, 200, 80, 1);
}
.sidebar-item.kugou-login.active {
  color: rgba(255, 200, 80, 1);
}

/* 歌单三级子项 */
.sidebar-item.sub-playlist {
  padding-left: 52px;
  font-size: 12px;
  color: rgba(255,255,255,0.4);
}
.sidebar-item.sub-playlist:hover {
  color: rgba(255,255,255,0.8);
}
.sidebar-item.sub-playlist.active {
  color: #a89cff;
}

.sidebar-item.disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.sidebar-divider {
  height: 1px;
  background: rgba(255,255,255,0.06);
  margin: 4px 12px;
}
.sidebar-spacer { flex: 1; }
</style>
