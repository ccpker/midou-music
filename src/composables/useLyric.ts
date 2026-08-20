// ════════════════════════════════════════════════
// 模块: composables::useLyric
// 路径: src/composables/useLyric.ts
// ────────────────────────────────────────────
// 功能: LRC 歌词解析 + 获取
// ════════════════════════════════════════════════

import { invoke } from '@tauri-apps/api/core';

// ── 类型 ─────────────────────────────────────────

export interface LyricLine {
  time: number;   // 秒（浮点）
  text: string;
}

export interface SongMeta {
  name: string;
  singer: string;
  duration: number;
  source: string;
  song_id: string;
}

// ── LRC 解析 ─────────────────────────────────────

/**
 * 解析 LRC 文本为带时间戳的行数组（按时间升序）
 * 支持多时间标签 [00:12.34][01:23.45]同一行歌词
 */
export function parseLrc(lrc: string): LyricLine[] {
  const lines: LyricLine[] = [];
  if (!lrc) return lines;

  const timeTagRe = /\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;

  for (const raw of lrc.split(/\r?\n/)) {
    // 跳过元信息标签 [ti:] [ar:] [al:] [by:] [offset:]
    if (/^\[(ti|ar|al|by|offset|re|ve):/i.test(raw.trim())) continue;

    const matches = [...raw.matchAll(timeTagRe)];
    if (matches.length === 0) continue;

    // 去掉所有时间标签，剩下的就是歌词文本
    const text = raw.replace(timeTagRe, '').trim();
    if (!text) continue;

    for (const m of matches) {
      const min = parseInt(m[1], 10);
      const sec = parseInt(m[2], 10);
      let frac = 0;
      if (m[3]) {
        // 根据位数归一化到毫秒：2位=百分秒，3位=毫秒
        const digits = m[3];
        frac = parseInt(digits, 10) / Math.pow(10, digits.length);
      }
      const time = min * 60 + sec + frac;
      lines.push({ time, text });
    }
  }

  lines.sort((a, b) => a.time - b.time);
  return lines;
}

/**
 * 根据当前播放时间，找到当前应高亮的歌词行索引
 */
export function findCurrentLine(lines: LyricLine[], time: number): number {
  if (lines.length === 0) return -1;
  let idx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].time <= time) {
      idx = i;
    } else {
      break;
    }
  }
  return idx;
}

// ── 获取歌词 ─────────────────────────────────────

/**
 * 从后端获取歌词（三级回退：酷狗官方 → LRCLIB 精确 → LRCLIB 模糊）
 * 返回 LRC 文本，找不到返回空字符串
 */
export async function fetchLyric(song: SongMeta): Promise<string> {
  try {
    const lrc = await invoke<string>('get_lyric', {
      name: song.name,
      singer: song.singer,
      duration: song.duration || 0,
      source: song.source,
      songId: song.song_id,
    });
    return lrc || '';
  } catch (e) {
    console.warn('[useLyric] get_lyric 失败:', e);
    return '';
  }
}
