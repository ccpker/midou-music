import { useState, useCallback, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { LazyStore } from "@tauri-apps/plugin-store"
import { Button } from "./components/ui/button"
import Sidebar from "./components/Sidebar"
import SongList from "./components/SongList"
import SearchBar from "./components/SearchBar"
import KugouLogin from "./components/KugouLogin"
import type { Song } from "./types"

const store = new LazyStore("store.json")

export default function App() {
  return <AppShell />
}

function AppShell() {
  const [activeSection, setActiveSection] = useState("kuwo")
  const [songs, setSongs] = useState<Song[]>([])
  const [currentSong, setCurrentSong] = useState<Song | null>(null)
  const [isPlaying, setIsPlaying] = useState(false)
  const [searching, setSearching] = useState(false)

  // ── 酷狗登录态 + 歌单（App 层管）──
  const [kugouLoggedIn, setKugouLoggedIn] = useState(false)
  const [kugouPlaylists, setKugouPlaylists] = useState<any[]>([])

  const loadPlaylists = useCallback(async () => {
    try {
      const pl = await invoke<any[]>("kugou_playlists")
      setKugouPlaylists(pl ?? [])
    } catch (e) { console.error("[App] 歌单加载失败:", e) }
  }, [])

  // 初始化：等 sidecar 就绪 → 注册设备 + 恢复酷狗登录态 → 拉歌单
  useEffect(() => {
    let unlisten: (() => void) | null = null
    let timer: ReturnType<typeof setTimeout> | null = null
    
    const init = async () => {
      try {
        console.log("[App] 开始初始化...")
        // 先注册设备
        try { await invoke("kugou_register_device") } catch (e) { console.warn("[App] 设备注册失败:", e) }
        // 恢复登录态
        const saved = await store.get<any>("kugou_auth")
        if (saved?.token) {
          await invoke("kugou_set_auth", { token: saved.token, userid: Number(saved.userid) })
          setKugouLoggedIn(true)
          await loadPlaylists()
        }
        // 初始化完成，启用托盘
        await invoke("init_tray")
        console.log("[App] 初始化完成，托盘已启用")
      } catch (e) { console.error("[App] 初始化失败:", e) }
    }
    
    // 监听 sidecar 就绪事件
    listen<{ready: boolean}>("sidecar_status", (event) => {
      if (event.payload.ready) {
        console.log("[App] 收到 sidecar 就绪事件")
        init()
      }
    }).then((fn) => { unlisten = fn })
    
    // 兜底：3秒后还没收到事件就直接初始化
    timer = setTimeout(() => {
      console.log("[App] 3秒超时，直接初始化")
      init()
    }, 3000)
    
    return () => {
      if (timer) clearTimeout(timer)
      if (unlisten) unlisten()
    }
  }, [loadPlaylists])

  // ── 导航 ──
  const handleNavigate = useCallback((section: string) => {
    setActiveSection(section)
  }, [])

  // ── 搜索 ──
  const handleSearch = useCallback(async (keyword: string, source: string) => {
    if (!keyword.trim()) return
    setSearching(true)
    try {
      const results = await invoke<Song[]>("search", { keyword, source })
      setSongs(results)
    } catch (e) {
      console.error("[App] 搜索失败:", e)
      setSongs([])
    } finally {
      setSearching(false)
    }
  }, [])

  // ── 歌单点击 → 加载歌曲到主面板 ──
  const handlePlaylistClick = useCallback(async (listId: number) => {
    setActiveSection(`kugou_pl_${listId}`)
    setSearching(true)
    try {
      const raw = await invoke<any>("kugou_playlist_songs", { listId })
      const songsArr = raw?.data?.info ?? raw?.info ?? raw ?? []
      setSongs(songsArr.map((s: any) => ({
        song_id: `kugou:${s.hash}:${s.audio_id}`,
        name: s.name || s.filename || s.title || "?",
        singer: s.singerinfo?.[0]?.name ?? s.singer ?? "",
        album: s.album || "",
        duration: Number(s.duration) || 0,
        source: "kugou" as const,
      })))
    } catch (e) {
      console.error("[App] 歌单歌曲加载失败:", e)
      setSongs([])
    } finally {
      setSearching(false)
    }
  }, [])

  // ── 酷狗登录成功回调 ──
  const handleKugouLoginSuccess = useCallback(async () => {
    setKugouLoggedIn(true)
    setActiveSection("kugou")
    await loadPlaylists()
  }, [])

  // ── 酷狗登出回调 ──
  const handleKugouLogout = useCallback(() => {
    setKugouLoggedIn(false)
    setKugouPlaylists([])
  }, [])

  // ── 播放控制 ──
  const handlePlay = useCallback(async (song: Song) => {
    try {
      const playResult = await invoke<any>("play_url", { songId: song.song_id })
      if (!playResult.url) {
        alert("暂无可用播放链接")
        return
      }
      setCurrentSong(song)
      setIsPlaying(true)
      await invoke("audio_play", { url: playResult.url })
    } catch (e: any) {
      alert(`播放失败: ${e?.toString?.() || String(e)}`)
      setIsPlaying(false)
    }
  }, [])

  const handlePause = useCallback(async () => {
    try { await invoke("audio_pause"); setIsPlaying(false) } catch (e) { console.error(e) }
  }, [])

  const handleResume = useCallback(async () => {
    try { await invoke("audio_resume"); setIsPlaying(true) } catch (e) { console.error(e) }
  }, [])

  const handleNext = useCallback(async () => {
    // TODO
  }, [])

  // ── 主面板：歌单模式 vs 搜索模式 ──
  const isKugouPl = activeSection.startsWith("kugou_pl_")
  const kugouPlTitle = isKugouPl
    ? kugouPlaylists.find(p => `kugou_pl_${p.listid}` === activeSection)?.name ?? "歌单"
    : ""

  return (
    <div className="flex h-screen w-screen">
      <Sidebar
        activeSection={activeSection}
        onNavigate={handleNavigate}
        kugouLoggedIn={kugouLoggedIn}
        kugouPlaylists={kugouPlaylists}
        onPlaylistClick={handlePlaylistClick}
      />

      <div className="flex-1 flex flex-col min-w-0">
        {activeSection === "kugou_login" ? (
          <KugouLogin
            onSuccess={handleKugouLoginSuccess}
            onLogout={handleKugouLogout}
          />
        ) : isKugouPl ? (
          <>
            <div className="h-12 flex items-center gap-2 px-4 border-b border-zinc-200 dark:border-zinc-800 shrink-0">
              <span className="text-sm font-medium">🎵 {kugouPlTitle}</span>
              <span className="text-xs text-zinc-400">{songs.length} 首</span>
            </div>
            <div className="flex-1 overflow-auto">
              <SongList
                songs={songs}
                searching={searching}
                currentSong={currentSong}
                isPlaying={isPlaying}
                onPlay={handlePlay}
                onPause={handlePause}
                onResume={handleResume}
                onNext={handleNext}
              />
            </div>
          </>
        ) : (
          <>
            <SearchBar source={activeSection} onSearch={handleSearch} />
            <div className="flex-1 overflow-auto">
              <SongList
                songs={songs}
                searching={searching}
                currentSong={currentSong}
                isPlaying={isPlaying}
                onPlay={handlePlay}
                onPause={handlePause}
                onResume={handleResume}
                onNext={handleNext}
              />
            </div>
          </>
        )}

        {/* 底部播放状态栏 */}
        {currentSong && (
          <div className="h-16 border-t border-zinc-200 dark:border-zinc-800 px-4 flex items-center gap-3 bg-zinc-50 dark:bg-zinc-950 shrink-0">
            <span className="text-sm font-medium truncate max-w-[200px]">{currentSong.name}</span>
            <span className="text-xs text-zinc-400">{currentSong.singer}</span>
            <div className="flex-1" />
            <Button size="sm" variant="ghost" onClick={isPlaying ? handlePause : handleResume}>
              {isPlaying ? "⏸" : "▶"}
            </Button>
            <Button size="sm" variant="ghost" onClick={handleNext}>⏭</Button>
          </div>
        )}
      </div>
    </div>
  )
}
