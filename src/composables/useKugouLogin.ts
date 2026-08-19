/**
 * Composable: useKugouLogin
 * 路径: src/composables/useKugouLogin.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 酷狗扫码登录状态管理 + UI 面板控制
 *
 * 工作流:
 *   1. kugou_auth_status()  → 检查是否已登录
 *   2. kugou_qr_key()       → 获取二维码（base64 PNG）
 *   3. [前端轮询 3s] kugou_qr_check() → 检查扫码状态
 *   4. status=4 → kugou_save_qr_token() → 保存 token
 *   5. kugou_logout()       → 退出登录
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ── 状态 ────────────────────────────────────────────

export const isLoggedIn = ref(false)
export const userid = ref('')
export const nickname = ref('')
export const qrKey = ref('')
export const qrImg = ref('')        // "data:image/png;base64,..."
export const qrStatus = ref<number | null>(null)
export const qrStatusText = ref('')
export const isLoading = ref(false)
export const error = ref('')
export const showPanel = ref(false)

let pollTimer: ReturnType<typeof setInterval> | null = null

// ── 内部工具 ───────────────────────────────────────

async function cmd(name: string, args: Record<string, unknown> = {}) {
  return invoke(name, args)
}

// ── 登录状态 ───────────────────────────────────────

export async function initKugouLogin() {
  await checkStatus()
  // 监听全局 auth_updated 事件（其他窗口触发）
  listen<{ logged_in: boolean; userid: string; nickname: string }>(
    'kugou_auth_updated',
    (event) => {
      isLoggedIn.value = event.payload.logged_in
      userid.value = event.payload.userid || ''
      nickname.value = event.payload.nickname || ''
      if (event.payload.logged_in) {
        showPanel.value = false
      }
    }
  )
}

export async function checkStatus() {
  try {
    const res = await cmd('kugou_auth_status') as {
      logged_in: boolean
      userid: number
      nickname: string
    }
    isLoggedIn.value = res.logged_in
    userid.value = String(res.userid || '')
    nickname.value = res.nickname || ''
    return res.logged_in
  } catch {
    isLoggedIn.value = false
    return false
  }
}

// ── 扫码登录 ───────────────────────────────────────

export async function startQrLogin() {
  console.warn('[useKugouLogin] startQrLogin 被调用')
  isLoading.value = true
  error.value = ''
  showPanel.value = true
  console.warn('[useKugouLogin] showPanel=', showPanel.value, ' isLoading=', isLoading.value)

  try {
    console.warn('[useKugouLogin] 调用 kugou_qr_key IPC...')
    const r = await cmd('kugou_qr_key') as { qrcode_key: string; qrcode_img: string }
    console.warn('[useKugouLogin] IPC返回 qrcode_key=', r.qrcode_key?.slice(0,8), ' img_len=', r.qrcode_img?.length)
    qrKey.value = r.qrcode_key
    qrImg.value = r.qrcode_img
    qrStatus.value = null
    qrStatusText.value = '等待扫码...'
    stopPolling()
    pollTimer = setInterval(pollQrStatus, 3000)
  } catch (e: unknown) {
    console.warn('[useKugouLogin] ❌ IPC报错:', e)
    error.value = String(e)
    qrStatusText.value = '获取二维码失败'
  } finally {
    isLoading.value = false
  }
}

async function pollQrStatus() {
  if (!qrKey.value) return
  try {
    const r = await cmd('kugou_qr_check', { qrcodeKey: qrKey.value }) as {
      status: number
      status_text: string
      token?: string
      userid?: string
      nickname?: string
    }
    qrStatus.value = r.status
    qrStatusText.value = r.status_text

    if (r.status === 4) {
      // 登录成功 → 保存 token
      stopPolling()
      await cmd('kugou_save_qr_token', {
        token: r.token || '',
        userid: r.userid || '',
        nickname: r.nickname || '',
      })
      isLoggedIn.value = true
      userid.value = r.userid || ''
      nickname.value = r.nickname || ''
      showPanel.value = false
    } else if (r.status === 0) {
      // 过期
      stopPolling()
    }
    // 1=等待扫码 2=已扫待确认 → 继续轮询
  } catch {
    // 轮询错误不中断，继续
  }
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

export async function refreshQr() {
  stopPolling()
  await startQrLogin()
}

// ── 退出登录 ───────────────────────────────────────

export async function logout() {
  stopPolling()
  try {
    await cmd('kugou_logout')
  } catch { /* ignore */ }
  isLoggedIn.value = false
  userid.value = ''
  nickname.value = ''
  qrKey.value = ''
  qrImg.value = ''
  qrStatus.value = null
  qrStatusText.value = ''
  showPanel.value = false
  error.value = ''
}

export function closePanel() {
  stopPolling()
  showPanel.value = false
  error.value = ''
}
