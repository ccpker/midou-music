// ════════════════════════════════════════════════
// 模块: composables::useSearch
// 路径: src/composables/useSearch.ts
// ────────────────────────────────────────────
// 功能: 搜索逻辑封装（多平台路由）
// 平台路由:
//   "kuwo"  → 酷我音乐
//   "bili"  → B站音频
//   "kugou" → 酷狗（待接入）
//   "qq"    → QQ音乐（待接入）
// ════════════════════════════════════════════════

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Song } from './usePlayer';

// ── 搜索状态（单例）──────────────────────────────

const results = ref<Song[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const currentSource = ref('kuwo');

/**
 * 搜索歌曲
 * @param keyword - 搜索关键词
 * @param source  - 音源标识（默认当前选中的平台）
 */
export async function search(keyword: string, source?: string): Promise<Song[]> {
  if (!keyword.trim()) {
    results.value = [];
    return [];
  }

  const src = source ?? currentSource.value;
  currentSource.value = src;
  loading.value = true;
  error.value = null;

  try {
    const songs = await invoke<Song[]>('search', { keyword, source: src });
    results.value = songs;
    return songs;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    error.value = msg;
    results.value = [];
    return [];
  } finally {
    loading.value = false;
  }
}

export function clearResults() {
  results.value = [];
  error.value = null;
}

export function useSearch() {
  return { results, loading, error, currentSource };
}
