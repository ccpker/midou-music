import { Home, Music, Play, Disc, Radio, Folder, Settings, ChevronDown, ChevronRight, LogIn, LogOut } from "lucide-react"
import { useState } from "react"
import SidecarPanel from "./SidecarPanel"

interface PlaylistItem {
  listid: number
  name: string
  count?: number
  m_count?: number
}

interface Props {
  activeSection: string
  onNavigate: (section: string) => void
  kugouLoggedIn: boolean
  kugouPlaylists: PlaylistItem[]
  onPlaylistClick: (listId: number) => void
}

const NAV_ITEMS = [
  { id: "kuwo", label: "酷我音乐", icon: <Music className="w-4 h-4" /> },
  { id: "bili", label: "B站音频", icon: <Play className="w-4 h-4" /> },
  { id: "kugou", label: "酷狗音乐", icon: <Radio className="w-4 h-4" /> },
  { id: "qq", label: "QQ音乐", icon: <Disc className="w-4 h-4" /> },
  { id: "local", label: "本地音乐", icon: <Folder className="w-4 h-4" /> },
]

export default function Sidebar({ activeSection, onNavigate, kugouLoggedIn, kugouPlaylists, onPlaylistClick }: Props) {
  const [expanded, setExpanded] = useState<string | null>("kuwo")

  const toggleExpand = (id: string) => {
    setExpanded(expanded === id ? null : id)
  }

  return (
    <div className="w-56 h-full border-r border-zinc-200 dark:border-zinc-800 flex flex-col bg-zinc-50/50 dark:bg-zinc-950/50">
      <div className="h-14 flex items-center gap-2 px-4 border-b border-zinc-200 dark:border-zinc-800">
        <Home className="w-5 h-5" />
        <span className="font-semibold text-sm">米豆音乐</span>
      </div>
      <nav className="flex-1 overflow-auto py-2">
        {/* 音源列表 */}
        {NAV_ITEMS.map((item) => (
          <div key={item.id}>
            <button
              onClick={() => {
                toggleExpand(item.id)
                onNavigate(item.id)
              }}
              className={`w-full flex items-center gap-2 px-4 py-2 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                activeSection === item.id || activeSection.startsWith(`${item.id}_`)
                  ? "bg-zinc-100 dark:bg-zinc-800 font-medium"
                  : ""
              }`}
            >
              {item.icon}
              <span className="flex-1 text-left">{item.label}</span>
              {expanded === item.id
                ? <ChevronDown className="w-3 h-3 text-zinc-400" />
                : <ChevronRight className="w-3 h-3 text-zinc-400" />
              }
            </button>

            {/* 展开子菜单 */}
            {expanded === item.id && item.id === "kugou" && (
              <div className="space-y-0.5">
                {/* 搜索 */}
                <button
                  onClick={() => onNavigate("kugou")}
                  className={`w-full flex items-center gap-2 pl-12 pr-4 py-1.5 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                    activeSection === "kugou" ? "text-black dark:text-white font-medium" : "text-zinc-500"
                  }`}
                >
                  🔍 搜索
                </button>

                {/* 登录/登出 */}
                <button
                  onClick={() => onNavigate("kugou_login")}
                  className={`w-full flex items-center gap-2 pl-12 pr-4 py-1.5 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                    activeSection === "kugou_login" ? "text-black dark:text-white font-medium" : "text-zinc-500"
                  }`}
                >
                  {kugouLoggedIn ? <LogOut className="w-3 h-3" /> : <LogIn className="w-3 h-3" />}
                  {kugouLoggedIn ? "账号管理" : "登录"}
                </button>

                {/* ★ 我的歌单 — 动态加载 */}
                {kugouLoggedIn && (
                  <div className="mt-1">
                    <div className="text-[10px] text-zinc-400 font-medium px-12 py-0.5">我的歌单</div>
                    {kugouPlaylists.length === 0 ? (
                      <div className="text-[10px] text-zinc-400 italic px-12 py-0.5">加载中...</div>
                    ) : (
                      kugouPlaylists.map((pl) => (
                        <button
                          key={pl.listid}
                          onClick={() => onPlaylistClick(pl.listid)}
                          className={`w-full flex items-center gap-1 pl-12 pr-2 py-1 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                            activeSection === `kugou_pl_${pl.listid}`
                              ? "text-black dark:text-white font-medium bg-zinc-100 dark:bg-zinc-800"
                              : "text-zinc-500"
                          }`}
                        >
                          <span className="flex-1 truncate text-left">{pl.name}</span>
                          {(pl.m_count ?? pl.count ?? 0) > 0 && (
                            <span className="text-[10px] text-zinc-400 shrink-0">{pl.m_count ?? pl.count}</span>
                          )}
                        </button>
                      ))
                    )}
                  </div>
                )}

                {/* 非酷狗 — 通用搜索入口 */}
                {item.id !== "kugou" && (
                  <button
                    onClick={() => onNavigate(item.id)}
                    className={`w-full flex items-center gap-2 pl-12 pr-4 py-1.5 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                      activeSection === item.id ? "text-black dark:text-white font-medium" : "text-zinc-500"
                    }`}
                  >
                    🔍 搜索
                  </button>
                )}
              </div>
            )}

            {/* 非酷狗展开 */}
            {expanded === item.id && item.id !== "kugou" && (
              <button
                onClick={() => onNavigate(item.id)}
                className={`w-full flex items-center gap-2 pl-12 pr-4 py-1.5 text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
                  activeSection === item.id ? "text-black dark:text-white font-medium" : "text-zinc-500"
                }`}
              >
                🔍 搜索
              </button>
            )}
          </div>
        ))}
      </nav>

      {/* 底部: 引擎状态 + 设置 */}
      <SidecarPanel />
      <div className="border-t border-zinc-200 dark:border-zinc-800">
        <button
          onClick={() => onNavigate("settings")}
          className={`w-full flex items-center gap-2 px-4 py-3 text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors cursor-pointer ${
            activeSection === "settings" ? "bg-zinc-100 dark:bg-zinc-800" : ""
          }`}
        >
          <Settings className="w-4 h-4" />
          <span>设置</span>
        </button>
      </div>
    </div>
  )
}
