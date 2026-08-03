// AuthContext.tsx — 全局登录态（React Context + Tauri IPC）
// 组件挂载/卸载都不丢状态，restart 后从 DB 恢复

import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

// ── 数据结构 ──────────────────────────────

interface KugouAuthState {
  logged_in: boolean
  userid: string
  nickname: string
}

interface AuthContextType {
  kugou: KugouAuthState
  /** 主动刷新登录状态（登录成功后调用） */
  refreshKugou: () => Promise<void>
  /** 强制设为已登录（扫码回调用，避免等待轮询） */
  setKugouLoggedIn: (nickname: string) => void
}

const AuthContext = createContext<AuthContextType | null>(null)

// ── Provider ─────────────────────────────

export function AuthProvider({ children }: { children: ReactNode }) {
  const [kugou, setKugou] = useState<KugouAuthState>({
    logged_in: false, userid: "", nickname: "",
  })

  const refreshKugou = useCallback(async () => {
    try {
      const s = await invoke<KugouAuthState>("kugou_auth_status")
      console.log("[AuthContext] kugou_auth_status:", s)
      setKugou(s)
    } catch (e) {
      console.error("[AuthContext] kugou_auth_status 失败:", e)
    }
  }, [])

  /** 扫码回调：不等轮询，直接设 */
  const setKugouLoggedIn = useCallback((nickname: string) => {
    setKugou({ logged_in: true, userid: "", nickname })
  }, [])

  // 首次挂载查 DB
  useEffect(() => {
    refreshKugou()
  }, [refreshKugou])

  // 监听 Rust 侧 emit 的 kugou_auth_updated 事件
  useEffect(() => {
    let unlisten: UnlistenFn | null = null
    ;(async () => {
      try {
        unlisten = await listen<KugouAuthState>("kugou_auth_updated", (event) => {
          console.log("[AuthContext] kugou_auth_updated 事件:", event.payload)
          setKugou(event.payload)
        })
      } catch (e) {
        console.error("[AuthContext] 监听 kugou_auth_updated 失败:", e)
      }
    })()
    return () => { unlisten?.() }
  }, [])

  return (
    <AuthContext.Provider value={{ kugou, refreshKugou, setKugouLoggedIn }}>
      {children}
    </AuthContext.Provider>
  )
}

// ── Hook ─────────────────────────────────

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error("useAuth 必须在 AuthProvider 内使用")
  return ctx
}
