// KugouLogin.tsx — 酷狗登录板块（自闭环，只管登录/登出）
//
// 两层缓存:
//   模块级变量 = 会话缓存（切页面不丢）
//   LazyStore  = 磁盘持久化（重启不丢）

import { useState, useEffect, useRef } from "react"
import { LazyStore } from "@tauri-apps/plugin-store"
import { invoke } from "@tauri-apps/api/core"

const store = new LazyStore("store.json")

interface KugouAuth { token: string; userid: string; nickname: string }

let cachedAuth: KugouAuth | null = null

interface Props {
  onSuccess: () => void
  onLogout: () => void
}

export default function KugouLogin({ onSuccess, onLogout }: Props) {
  const [phase, setPhase] = useState<"loading" | "logged_out" | "qr_fetching" | "qr_ready" | "qr_scanning" | "logged_in">("loading")
  const [auth, setAuth] = useState<KugouAuth>(cachedAuth ?? { token: "", userid: "", nickname: "" })
  const [userInfo, setUserInfo] = useState<any>(null)
  const [error, setError] = useState("")

  const qrKeyRef = useRef("")
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const imgRef = useRef<HTMLImageElement>(null)
  const mountedRef = useRef(true)

  const stopPoll = () => { if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null } }

  useEffect(() => {
    mountedRef.current = true
    ;(async () => {
      // Priority 1: 模块级缓存（切页面不丢）
      if (cachedAuth?.token) {
        setAuth(cachedAuth)
        setPhase("logged_in")
        await setupAuth(cachedAuth)
        invoke<any>("kugou_user_info").then(info => {
          if (mountedRef.current) setUserInfo(info?.data ?? info)
        }).catch(() => {})
        return
      }
      // Priority 2: Store 持久化（重启不丢）
      try {
        const saved = await store.get<KugouAuth>("kugou_auth")
        if (saved?.token) {
          cachedAuth = { token: saved.token, userid: saved.userid ?? "", nickname: saved.nickname ?? "" }
          if (!mountedRef.current) return
          setAuth(cachedAuth)
          setPhase("logged_in")
          await setupAuth(cachedAuth)
          invoke<any>("kugou_user_info").then(info => {
            if (mountedRef.current) setUserInfo(info?.data ?? info)
          }).catch(() => {})
          return
        }
      } catch (e) { console.error("[kugou] Store 读取失败:", e) }
      if (mountedRef.current) setPhase("logged_out")
    })()
    return () => { mountedRef.current = false; stopPoll() }
  }, [])

  const setupAuth = async (auth: KugouAuth) => {
    try {
      await invoke("kugou_set_auth", {
        token: auth.token,
        userid: Number(auth.userid),  // 统一转 number，兼容 QR 返回(number) + Store 反序列化(string)
      })
    } catch (e) { console.error("[kugou] set_auth 失败:", e) }
  }

  const fetchQr = async () => {
    setPhase("qr_fetching"); setError("")
    try {
      const res = await invoke<{ qrcode_key: string; qrcode_img: string }>("kugou_qr_key")
      qrKeyRef.current = res.qrcode_key; setPhase("qr_ready")
      if (imgRef.current) { imgRef.current.src = res.qrcode_img; imgRef.current.style.display = "block" }
      startPoll(res.qrcode_key)
    } catch (e: any) { setError(`获取失败: ${e?.toString?.() || String(e)}`); setPhase("logged_out") }
  }

  const startPoll = (key: string) => {
    stopPoll(); let done = false
    const poll = async () => {
      if (done) return
      try {
        const res = await invoke<any>("kugou_qr_check", { qrcodeKey: key })
        if (res.status === 4) {
          done = true; stopPoll()
          const newAuth: KugouAuth = { token: res.token, userid: res.userid, nickname: res.nickname }
          cachedAuth = newAuth; setAuth(newAuth); setPhase("logged_in")
          await store.set("kugou_auth", newAuth); await store.save()
          await setupAuth(newAuth)  // ← 必须 await，确保 token 推到 Rust 内存后再拉歌单
          try { const info = await invoke<any>("kugou_user_info"); setUserInfo(info?.data ?? info) } catch {}
          onSuccess()
        } else if (res.status === 2) {
          setPhase("qr_scanning")
        } else if (res.status === 0) {
          stopPoll(); setError("二维码已过期，请重新获取"); setPhase("qr_ready")
        } else {
          // status=1 等待扫码，保持 qr_ready
          setPhase("qr_ready")
        }
      } catch (e) { console.error("[kugou] poll:", e) }
    }
    poll(); pollRef.current = setInterval(poll, 2000)
  }

  const logout = async () => {
    stopPoll(); cachedAuth = null
    setAuth({ token: "", userid: "", nickname: "" })
    setUserInfo(null); setPhase("logged_out")
    await store.delete("kugou_auth"); await store.save()
    onLogout()
  }

  // ════════════════ UI ════════════════

  if (phase === "loading") {
    return <div className="p-6 text-sm text-zinc-400 text-center">检查登录状态...</div>
  }

  if (phase === "logged_in") {
    return (
      <div className="flex flex-col items-center justify-center flex-1 p-6">
        <div className="flex flex-col items-center gap-4 max-w-xs w-full">
          <div className="w-16 h-16 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center text-2xl">
            {userInfo?.nickname ? userInfo.nickname[0] : "👤"}
          </div>
          <div className="text-center">
            <div className="text-base font-medium">{userInfo?.nickname || auth.nickname || "酷狗用户"}</div>
            <div className="text-xs text-zinc-400 mt-1">ID: {auth.userid}</div>
            {userInfo?.vip === 1 && <div className="text-xs text-amber-500 mt-0.5">VIP 会员</div>}
          </div>
          <button
            onClick={logout}
            className="px-4 py-2 text-sm rounded bg-zinc-100 dark:bg-zinc-800 hover:bg-red-100 dark:hover:bg-red-900 hover:text-red-600 transition-colors"
          >
            退出登录
          </button>
        </div>
      </div>
    )
  }

  // ── 扫码界面 ──
  return (
    <div className="flex flex-col items-center justify-center flex-1 p-6">
      <div className="flex flex-col items-center gap-3 max-w-xs w-full">
        <h3 className="text-sm font-medium">酷狗扫码登录</h3>
        <div className="w-44 h-44 border-2 border-zinc-300 dark:border-zinc-600 rounded-lg flex items-center justify-center bg-white">
          <img ref={imgRef} alt="二维码" className="hidden w-40 h-40" />
        </div>
        <p className="text-xs text-zinc-400 min-h-[16px] text-center">
          {phase === "qr_fetching" ? "获取中..." :
           phase === "qr_scanning" ? "已扫码，请在APP确认" :
           phase === "qr_ready" ? "请用酷狗APP扫码" :
           "点击下方按钮获取二维码"}
        </p>
        {error && <p className="text-xs text-red-400">{error}</p>}
        <button onClick={fetchQr} disabled={phase === "qr_fetching"}
          className="px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:bg-blue-300 text-white rounded text-sm">
          {phase === "qr_fetching" ? "获取中..." : "获取二维码"}
        </button>
      </div>
    </div>
  )
}
