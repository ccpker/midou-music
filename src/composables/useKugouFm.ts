/**
 * Composable: useKugouFm
 * 路径: src/composables/useKugouFm.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 酷狗私人 FM（个性化推荐流）
 *
 * 机制: 后端调 /v2/personal_recommend（persnfm.service.kugou.com）
 *   返回一批个性化推荐歌曲，直接用 play_url(/v5/url) 播放。
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface FmSong {
  song_id: string
  name: string
  singer: string
  album: string
  duration?: number
  source: string
  cover_url?: string
}

// ── 状态 ────────────────────────────────────────────

const fmSongs = ref<FmSong[]>([])
const loading = ref(false)
const error = ref('')

export { fmSongs, loading, error }

// ── 拉一批私人 FM 推荐 ─────────────────────────────

export async function fetchPersonalFm(): Promise<void> {
  loading.value = true
  error.value = ''
  try {
    const res = await invoke<FmSong[]>('kugou_personal_fm')
    // 追加到现有列表（连续推荐）
    fmSongs.value = res || []
  } catch (e: unknown) {
    error.value = String(e)
    fmSongs.value = []
  } finally {
    loading.value = false
  }
}

export function useKugouFm() {
  return {
    fmSongs,
    loading,
    error,
    fetchPersonalFm,
  }
}
