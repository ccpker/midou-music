import { useEffect, useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { RefreshCw, CheckCircle, XCircle } from "lucide-react"

/** 侧边栏底部：sidecar 状态指示器 */
export default function SidecarPanel() {
  const [alive, setAlive] = useState<boolean | null>(null)
  const [restarting, setRestarting] = useState(false)
  const [lastCheck, setLastCheck] = useState("")

  // 定时探活（每 10 秒）
  const probe = useCallback(async () => {
    try {
      const r = await invoke<{ alive: boolean }>("sidecar_health")
      setAlive(r.alive)
      setLastCheck(new Date().toLocaleTimeString())
    } catch {
      setAlive(false)
    }
  }, [])

  useEffect(() => {
    probe()
    const timer = setInterval(probe, 10_000)
    return () => clearInterval(timer)
  }, [probe])

  const handleRestart = async () => {
    setRestarting(true)
    try {
      await invoke("sidecar_restart")
      setAlive(true)
      setLastCheck(new Date().toLocaleTimeString())
    } catch (e) {
      console.error("重启失败:", e)
      setAlive(false)
    } finally {
      setRestarting(false)
    }
  }

  return (
    <div className="border-t border-zinc-200 dark:border-zinc-800 px-3 py-2">
      <div className="flex items-center gap-2 text-xs">
        {/* 状态灯 */}
        {alive === null ? (
          <span className="w-2 h-2 rounded-full bg-yellow-400 animate-pulse" />
        ) : alive ? (
          <CheckCircle className="w-3 h-3 text-green-500" />
        ) : (
          <XCircle className="w-3 h-3 text-red-500" />
        )}
        <span className="text-zinc-500 flex-1">
          酷狗引擎 {alive === null ? "检测中" : alive ? "在线" : "离线"}
        </span>

        {/* 重启按钮 */}
        <button
          onClick={handleRestart}
          disabled={restarting}
          className="p-1 rounded hover:bg-zinc-200 dark:hover:bg-zinc-700 transition-colors cursor-pointer disabled:opacity-50"
          title="重启引擎"
        >
          <RefreshCw className={`w-3 h-3 text-zinc-400 ${restarting ? "animate-spin" : ""}`} />
        </button>
      </div>
      {lastCheck && (
        <div className="text-[10px] text-zinc-400 mt-0.5">
          上次检测 {lastCheck}
        </div>
      )}
    </div>
  )
}
