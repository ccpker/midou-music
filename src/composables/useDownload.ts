// ════════════════════════════════════════════════
// Composable: useDownload
// 路径: src/composables/useDownload.ts
// ────────────────────────────────────────────
// 功能: 歌曲下载（全源）+ 音质选择
// 依赖: @tauri-apps/api (invoke / dialog / event)
// ════════════════════════════════════════════════

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export interface QualityOption {
  value: string;
  label: string;
}

// ── 状态 ─────────────────────────────────────────

// 音乐库根目录（用户点下载时选一次，之后记住）
export const libraryRoot = ref('');

// 下载中的歌曲（key: song_id → 状态）
export const downloading = ref<Record<string, boolean>>({});
export const downloadDone = ref<Record<string, string>>({}); // song_id → 保存路径
export const downloadError = ref<Record<string, string>>({});

// ── 音质映射 ─────────────────────────────────────

const QUALITY_LABELS: Record<string, string> = {
  '128kmp3': '128k 标准',
  '192kmp3': '192k 较高',
  '320kmp3': '320k 高品质',
  '128': '128k 标准',
  '320': '320k 高品质',
  'flac': 'FLAC 无损',
  'high': '平台默认',
};

// ── API ─────────────────────────────────────────

/**
 * 获取某音源的候选音质列表
 */
export async function getQualities(source: string): Promise<QualityOption[]> {
  try {
    const list = await invoke<string[]>('get_qualities', { source });
    return list.map((v) => ({ value: v, label: QUALITY_LABELS[v] || v }));
  } catch {
    return [{ value: 'high', label: '平台默认' }];
  }
}

/**
 * 弹目录选择框，让用户选下载目录（即音乐库根目录）
 */
export async function pickDownloadDir(): Promise<string | null> {
  try {
    const dir = await open({
      directory: true,
      multiple: false,
      title: '选择下载目录（作为音乐库根目录）',
    });
    if (typeof dir === 'string' && dir) {
      libraryRoot.value = dir;
      return dir;
    }
    return null;
  } catch (e) {
    console.warn('[useDownload] 选择目录失败:', e);
    return null;
  }
}

/**
 * 下载一首歌
 * 若未选目录，先弹目录选择框
 */
export async function downloadSong(song: {
  song_id: string;
  name: string;
  singer: string;
  album?: string;
  duration?: number;
  source: string;
  cover_url?: string;
}, quality?: string): Promise<string | null> {
  const sid = song.song_id;

  // 1. 确保有下载目录
  let dir = libraryRoot.value;
  if (!dir) {
    dir = (await pickDownloadDir()) || '';
    if (!dir) return null; // 用户取消
  }

  // 2. 确定音质
  const q = quality || (await defaultQuality(song.source));

  // 3. 标记下载中
  downloading.value[sid] = true;
  downloadError.value[sid] = '';
  delete downloadDone.value[sid];

  try {
    const savedPath = await invoke<string>('download_song', {
      songId: song.song_id,
      name: song.name,
      singer: song.singer,
      album: song.album || '',
      duration: song.duration || 0,
      source: song.source,
      dir,
      quality: q,
    });
    downloadDone.value[sid] = savedPath;
    return savedPath;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    downloadError.value[sid] = msg;
    return null;
  } finally {
    downloading.value[sid] = false;
  }
}

async function defaultQuality(source: string): Promise<string> {
  const list = await getQualities(source);
  // 默认取最后一个（通常最高音质）
  return list.length > 0 ? list[list.length - 1].value : 'high';
}
