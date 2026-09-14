// ════════════════════════════════════════════════
// Composable: useLibrary
// 路径: src/composables/useLibrary.ts
// ────────────────────────────────────────────
// 功能: 本地音乐库（多级文件夹树 + 本地歌曲）
// ════════════════════════════════════════════════

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { libraryRoot } from './useDownload';

// ── 类型 ─────────────────────────────────────────

export interface LocalSong {
  song_id: string;    // "local:{绝对路径}"
  name: string;
  singer: string;
  album: string;
  duration: number;
  source: string;
  cover_url?: string;
}

export interface LibraryNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: LibraryNode[];
  songs?: LocalSong[];
}

// ── 状态 ─────────────────────────────────────────

export const libraryTree = ref<LibraryNode | null>(null);
export const libraryLoading = ref(false);
export const libraryError = ref('');
// 当前选中的文件夹
export const currentFolder = ref<LibraryNode | null>(null);

// ── API ─────────────────────────────────────────

/**
 * 扫描音乐库根目录
 */
export async function scanLibrary(root?: string): Promise<LibraryNode | null> {
  const r = root || libraryRoot.value;
  if (!r) {
    libraryError.value = '未设置音乐库目录';
    return null;
  }
  libraryLoading.value = true;
  libraryError.value = '';
  try {
    const tree = await invoke<LibraryNode>('scan_library', { root: r });
    libraryTree.value = tree;
    return tree;
  } catch (e) {
    libraryError.value = e instanceof Error ? e.message : String(e);
    libraryTree.value = null;
    return null;
  } finally {
    libraryLoading.value = false;
  }
}

/**
 * 打开某个文件夹（显示其歌曲列表）
 */
export function openFolder(node: LibraryNode) {
  currentFolder.value = node;
}
