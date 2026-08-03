import { useEffect, useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { Pause, Play } from "lucide-react"
import { Button } from "./ui/button"
import type { Song } from "../types"

interface PlayStateEvent {
  song: Song | null
  url: string | null
  is_playing: boolean
}

function formatTime(sec: number): string {
  if (!sec || sec <= 0) return "0:00"
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  return `${m}:${s.toString().padStart(2, "0")}`
}

export default function PlayerBar() {
  const [song, setSong] = useState<Song | null>(null)
  const [isPlaying, setIsPlaying] = useState(false)
  const [position, setPosition] = useState(0)
  const [duration, setDuration] = useState(0)

  // 监听播放状态变化
  useEffect(() => {
    const unlisten = listen<PlayStateEvent>("play_state", (event) => {
      const { song: s, is_playing } = event.payload
      if (s) {
        setSong(s)
      }
      setIsPlaying(is_playing)
    })

    // 定时轮询音频状态（rodio 在 Rust 里，需要用 IPC 获取位置）
    const timer = setInterval(async () => {
      try {
        const state = await invoke<{
          is_playing: boolean
          position: number
          duration: number
        }>("audio_state")
        setPosition(state.position)
        setDuration(state.duration)
        setIsPlaying(state.is_playing)
      } catch (_) {
        // 可能音频线程未启动
      }
    }, 500)

    return () => {
      unlisten.then(fn => fn())
      clearInterval(timer)
    }
  }, [])

  const handlePlayPause = useCallback(async () => {
    try {
      if (isPlaying) {
        await invoke("audio_pause")
        setIsPlaying(false)
      } else {
        await invoke("audio_resume")
        setIsPlaying(true)
      }
    } catch (e) {
      console.error("播放控制失败:", e)
    }
  }, [isPlaying])

  if (!song) {
    return (
      <div className="h-full flex items-center justify-center text-zinc-400 text-xs select-none">
        暂无播放
      </div>
    )
  }

  const progress = duration > 0 ? (position / duration) * 100 : 0

  return (
    <div className="h-full flex flex-col justify-center px-4 bg-zinc-50 dark:bg-zinc-950 select-none"
         data-tauri-drag-region>
      {/* 进度条 */}
      <div className="w-full h-1 bg-zinc-200 dark:bg-zinc-800 rounded-full mb-2">
        <div
          className="h-full bg-zinc-600 dark:bg-zinc-400 rounded-full transition-all duration-300"
          style={{ width: `${Math.min(progress, 100)}%` }}
        />
      </div>

      {/* 控制栏 */}
      <div className="flex items-center gap-3">
        <Button size="icon" variant="ghost" onClick={handlePlayPause}>
          {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
        </Button>

        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium truncate">{song.name}</div>
          <div className="text-xs text-zinc-400 truncate">{song.singer}</div>
        </div>

        <span className="text-xs text-zinc-400">
          {formatTime(position)} / {formatTime(duration)}
        </span>
      </div>
    </div>
  )
}
