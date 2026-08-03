import { Play, Pause, Loader2 } from "lucide-react"
import type { Song } from "../types"

interface Props {
  songs: Song[]
  searching: boolean
  currentSong: Song | null
  isPlaying: boolean
  onPlay: (song: Song) => void
  onPause: () => void
  onResume: () => void
  onNext: () => void
}

function formatDuration(sec: number): string {
  if (!sec || sec <= 0) return "--:--"
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  return `${m}:${s.toString().padStart(2, "0")}`
}

export default function SongList({
  songs,
  searching,
  currentSong,
  isPlaying,
  onPlay,
  onPause,
  onResume,
}: Props) {
  if (searching) {
    return (
      <div className="flex items-center justify-center h-full text-zinc-400">
        <Loader2 className="w-6 h-6 animate-spin" />
        <span className="ml-2 text-sm">搜索中...</span>
      </div>
    )
  }

  if (songs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-zinc-400 text-sm">
        搜索歌曲开始播放
      </div>
    )
  }

  const isCurrentSong = (song: Song) =>
    currentSong?.song_id === song.song_id && currentSong?.source === song.source

  return (
    <div className="divide-y divide-zinc-100 dark:divide-zinc-900">
      {songs.map((song, idx) => {
        const active = isCurrentSong(song)
        return (
          <div
            key={`${song.source}_${song.song_id}_${idx}`}
            className={`flex items-center gap-3 px-4 py-2 hover:bg-zinc-50 dark:hover:bg-zinc-900 cursor-pointer transition-colors ${
              active ? "bg-zinc-100 dark:bg-zinc-800" : ""
            }`}
            onDoubleClick={() => onPlay(song)}
          >
            {/* 播放按钮 */}
            <button
              onClick={() => {
                if (active && isPlaying) {
                  onPause()
                } else if (active && !isPlaying) {
                  onResume()
                } else {
                  onPlay(song)
                }
              }}
              className="w-8 h-8 rounded-full flex items-center justify-center text-zinc-400 hover:text-black dark:hover:text-white cursor-pointer flex-shrink-0"
            >
              {active && isPlaying
                ? <Pause className="w-4 h-4" />
                : <Play className="w-4 h-4" />
              }
            </button>

            {/* 歌曲信息 */}
            <div className="min-w-0 flex-1">
              <div className={`text-sm truncate ${active ? "font-medium" : ""}`}>
                {song.name}
              </div>
              <div className="text-xs text-zinc-400 truncate">
                {song.singer}
                {song.album ? ` · ${song.album}` : ""}
              </div>
            </div>

            {/* 时长 */}
            <span className="text-xs text-zinc-400 flex-shrink-0">
              {formatDuration(song.duration)}
            </span>

            {/* 音源标记 */}
            <span className="text-[10px] px-1 py-0.5 rounded bg-zinc-100 dark:bg-zinc-800 text-zinc-400 flex-shrink-0">
              {song.source}
            </span>
          </div>
        )
      })}
    </div>
  )
}
