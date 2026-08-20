/**
 * Composable: useKugouPlaylist
 * 路径: src/composables/useKugouPlaylist.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 酷狗歌单（我的收藏/歌单）数据管理
 *
 * 工作流:
 *   1. kugou_playlists()       → 获取用户歌单列表
 *   2. kugou_playlist_songs()  → 获取歌单内歌曲
 *   3. 点歌 → 复用 usePlayer.playSong()
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// ── 类型 ────────────────────────────────────────────

export interface KugouPlaylist {
  listid: number | string
  global_collection_id: string
  name: string
  song_count: number
}

export interface KugouPlaylistSong {
  song_id: string
  name: string
  singer: string
  album: string
  duration?: number
  source: string
  cover_url?: string
}

// ── 状态 ────────────────────────────────────────────

const playlists = ref<KugouPlaylist[]>([])
const loadingLists = ref(false)
const songs = ref<KugouPlaylistSong[]>([])
const loadingSongs = ref(false)
const error = ref('')
const currentPlaylist = ref<KugouPlaylist | null>(null)

// 导出 ref（供 Sidebar 等组件直接引用）
export { playlists, currentPlaylist, songs }

// ── 拉歌单列表 ─────────────────────────────────────

export async function fetchPlaylists(): Promise<void> {
  loadingLists.value = true
  error.value = ''
  try {
    const res = await invoke<KugouPlaylist[]>('kugou_playlists')
    playlists.value = res || []
  } catch (e: unknown) {
    error.value = String(e)
    playlists.value = []
  } finally {
    loadingLists.value = false
  }
}

// ── 拉歌单歌曲 ─────────────────────────────────────

export async function fetchPlaylistSongs(playlist: KugouPlaylist): Promise<void> {
  loadingSongs.value = true
  error.value = ''
  currentPlaylist.value = playlist
  try {
    const res = await invoke<KugouPlaylistSong[]>('kugou_playlist_songs', {
      globalCollectionId: playlist.global_collection_id,
    })
    songs.value = res || []
  } catch (e: unknown) {
    error.value = String(e)
    songs.value = []
  } finally {
    loadingSongs.value = false
  }
}

export function useKugouPlaylist() {
  return {
    playlists,
    loadingLists,
    songs,
    loadingSongs,
    error,
    currentPlaylist,
    fetchPlaylists,
    fetchPlaylistSongs,
  }
}
