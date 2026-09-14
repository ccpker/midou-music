// ════════════════════════════════════════════════
// 模块: composables::usePlayer
// 路径: src/composables/usePlayer.ts
// ────────────────────────────────────────────
// 功能: 播放状态管理 + Tauri IPC 联动
// 标注: 单例（模块级状态），所有播放逻辑集中于此
//
// 调用路径:
//   handlePlay(song)
//     → invoke('play_url', { songId })     获取音频 URL
//     → invoke('open_player')              打开播放条窗口（仅首次）
//     → invoke('emit_play_state', { state }) 推送播放状态
//
// 窗口通信:
//   emit('player_cmd')     → 播放条通知主窗口（上一首/下一首）
//   listen('player_position') → 播放条推进度（当前未用，保留）
// ════════════════════════════════════════════════

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ── 类型 ─────────────────────────────────────────

export interface Song {
  song_id: string;
  name: string;
  singer: string;
  album: string;
  duration?: number;
  source: string;
  cover_url?: string;
}

interface PlayState {
  song: Song | null;
  url: string | null;
  is_playing: boolean;
  position: number;
  duration: number;
}

// ── 单例状态 ─────────────────────────────────────

const currentSong = ref<Song | null>(null);
const playerOpen = ref(false);
let unlistenCmd: UnlistenFn | null = null;
let unlistenPos: UnlistenFn | null = null;

// ── 公开 API ────────────────────────────────────

/**
 * 播放一首歌曲
 * @param song - 要播放的歌曲对象
 */
export async function playSong(song: Song) {
  console.warn('[playSong] 开始播放:', song.name, 'id=', song.song_id);
  try {
    // 1. 获取音频 URL
    console.warn('[playSong] 调用 play_url...');
    const result = await invoke<{ url: string; source: string }>('play_url', {
      songId: song.song_id,
    });
    console.warn('[playSong] play_url 返回:', result);

    // 本地文件：play_url 返回的是绝对路径，需转成 asset 协议才能给 <audio> 用
    let playUrl = result.url;
    if (result.source === 'local' || song.song_id.startsWith('local:')) {
      playUrl = convertFileSrc(result.url);
    }

    currentSong.value = song;

    // 2. 打开播放条（仅首次）
    if (!playerOpen.value) {
      await invoke('open_player');
      playerOpen.value = true;
      // 等待窗口 WebView 加载完成，确保 listen('play_state') 已注册
      await new Promise((r) => setTimeout(r, 800));
      // 监听播放条命令（仅注册一次）
      if (!unlistenCmd) {
        unlistenCmd = await listen<string>('player_cmd', (ev) => {
          console.log('[usePlayer] 播放条命令:', ev.payload);
          // TODO: 实现上一首/下一首
        });
      }
      if (!unlistenPos) {
        unlistenPos = await listen<{ position: number; duration: number }>(
          'player_position',
          (ev) => {
            // 可扩展：同步队列进度
          }
        );
      }
    }

    // 3. 推送播放状态
    console.warn('[playSong] 推送状态到播放条, url长度=', result.url?.length);
    try {
      await invoke('emit_play_state', {
        state: {
          song,
          url: playUrl,
          is_playing: true,
          position: 0,
          duration: song.duration || 0,
        } as PlayState,
      });
      console.warn('[playSong] 播放状态推送完成');
    } catch(e2) {
      console.error('[playSong] emit_play_state 失败:', e2);
      alert('[emit_play_state 失败] ' + String(e2));
    }
  } catch (e) {
    console.error('[playSong] 播放失败:', e);
    alert('[播放失败] ' + song.name + '\n错误: ' + String(e));
  }
}

/**
 * 当前播放的歌曲
 */
export function useCurrentSong() {
  return { currentSong };
}
