// ════════════════════════════════════════════════
// Composable: useKugou
// 路径: src/composables/useKugou.ts
// ────────────────────────────────────────────
// 功能: 酷狗登录状态 + 登录/登出
// ════════════════════════════════════════════════

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface KugouUser {
  userid: number
  vip_type: number
  vip_token: string
}

const loggedIn = ref(false)
const user = ref<KugouUser | null>(null)
const loading = ref(false)
const error = ref('')

// 初始化：从后端拉取登录状态
export async function initKugouAuth() {
  try {
    const res = await invoke<{ logged_in: boolean; userid: number; vip_type: number }>('kugou_auth_status')
    loggedIn.value = res.logged_in
    if (res.logged_in) {
      user.value = { userid: res.userid, vip_type: res.vip_type, vip_token: '' }
    }
  } catch (e) {
    // 未登录或出错，忽略
    loggedIn.value = false
    user.value = null
  }
}

export async function kugouLogin(username: string, password: string): Promise<void> {
  loading.value = true
  error.value = ''
  try {
    const res = await invoke<{ success: boolean; userid: number; vip_type: number }>('kugou_login', {
      username,
      password,
    })
    if (res.success) {
      loggedIn.value = true
      user.value = { userid: res.userid, vip_type: res.vip_type, vip_token: '' }
    }
  } catch (e: unknown) {
    error.value = String(e)
    throw e
  } finally {
    loading.value = false
  }
}

export async function kugouLogout(): Promise<void> {
  loading.value = true
  error.value = ''
  try {
    await invoke('kugou_logout')
    loggedIn.value = false
    user.value = null
  } catch (e: unknown) {
    error.value = String(e)
    throw e
  } finally {
    loading.value = false
  }
}

export function useKugou() {
  return { loggedIn, user, loading, error }
}
