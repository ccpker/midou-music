/**
 * Composable: useKugouVip
 * 路径: src/composables/useKugouVip.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 酷狗签到领 VIP + VIP 状态查询 + 自动签到
 *
 * 机制（moekoe 同款）:
 *   扫码 web token → 签到 receive_vip_listen_song → 账号获得
 *   概念版 tvip（1 天）→ 就能播 VIP 完整版
 *
 * 自动签到:
 *   - 应用启动时已登录 → 静默签
 *   - 扫码登录成功 → 静默签
 *   - 签到接口幂等，error_code=131001 表示「今天已签过」，静默忽略
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { isLoggedIn } from './useKugouLogin'

// ── 状态 ────────────────────────────────────────────

export interface VipStatus {
  is_vip: number
  vip_type: number
  // tvip 权益（概念版畅听 VIP），无则 null
  tvip: {
    is_vip: number
    vip_end_time: string
    vip_begin_time: string
  } | null
}

export const vipStatus = ref<VipStatus | null>(null)
export const signLoading = ref(false)
export const signError = ref('')
export const signSuccess = ref('')
export const adLoading = ref(false)
export const adError = ref('')
export const adResult = ref('')

// 今日是否已签到（tvip 的 vip_begin_time 是今天）
export const isSignedToday = computed(() => {
  const t = vipStatus.value?.tvip
  if (!t?.vip_begin_time) return false
  const begin = new Date(t.vip_begin_time.replace(' ', 'T'))
  const now = new Date()
  return (
    begin.getFullYear() === now.getFullYear() &&
    begin.getMonth() === now.getMonth() &&
    begin.getDate() === now.getDate()
  )
})

// ── 查询 VIP 状态 ─────────────────────────────────────

export async function fetchVipStatus() {
  try {
    const raw = await invoke('kugou_vip_status') as any
    const data = raw?.data || {}
    // 从 busi_vip 数组里找 tvip 权益（is_vip=1 且在有效期内）
    let tvip: VipStatus['tvip'] = null
    const busi = Array.isArray(data.busi_vip) ? data.busi_vip : []
    for (const b of busi) {
      if (b && b.product_type === 'tvip' && b.is_vip === 1) {
        tvip = {
          is_vip: b.is_vip,
          vip_end_time: b.vip_end_time || '',
          vip_begin_time: b.vip_begin_time || '',
        }
        break
      }
    }
    vipStatus.value = {
      is_vip: data.is_vip ?? 0,
      vip_type: data.vip_type ?? 0,
      tvip,
    }
  } catch (e) {
    signError.value = String(e)
  }
  return vipStatus.value
}

// ── 签到领 VIP ───────────────────────────────────────

export async function signVip() {
  signLoading.value = true
  signError.value = ''
  signSuccess.value = ''
  try {
    const raw = await invoke('kugou_sign_vip') as any
    const status = raw?.status
    const code = raw?.error_code
    if (status === 1 && (code === 0 || code === undefined)) {
      signSuccess.value = '签到成功，已领取 1 天 VIP 🎉'
      await fetchVipStatus()
    } else if (code === 131001) {
      signSuccess.value = '今天已经签到过了'
      await fetchVipStatus()
    } else {
      signError.value = raw?.error_msg || `签到失败 (code=${code})`
    }
  } catch (e) {
    signError.value = String(e)
  } finally {
    signLoading.value = false
  }
  return { ok: !signError.value, error: signError.value, success: signSuccess.value }
}

// ── 自动签到（静默，不打扰用户）─────────────────────────

export async function autoSignVip() {
  // 未登录或已签今日，直接跳过
  if (!isLoggedIn.value) return
  if (isSignedToday.value) return
  try {
    const raw = await invoke('kugou_sign_vip') as any
    // 成功或已签（131001）都静默刷新状态；失败也静默（下次再试）
    if (raw?.status === 1 || raw?.error_code === 131001) {
      await fetchVipStatus()
    }
  } catch {
    /* 自动签到失败静默，不打扰 */
  }
}

// 监听登录态变化：登录成功后自动签到
let _autoSignBound = false
export function bindAutoSign() {
  if (_autoSignBound) return
  _autoSignBound = true
  listen('kugou_auth_updated', (event) => {
    const p = event.payload as { logged_in: boolean }
    if (p?.logged_in) {
      // 登录成功后延时静默签到
      setTimeout(async () => {
        await fetchVipStatus()
        await autoSignVip()
      }, 1000)
    }
  })
}

// ── 看广告领时长 ───────────────────────────────────

export async function watchAd() {
  adLoading.value = true
  adError.value = ''
  adResult.value = ''
  try {
    const raw = await invoke('kugou_watch_ad') as any
    const hours = raw?.total_hours ?? 0
    const done = raw?.done_count ?? 0
    const limit = raw?.total_limit ?? 0
    if (hours > 0) {
      adResult.value = `已领取 ${hours} 小时 VIP（看广告 ${done}/${limit} 次）🎉`
      await fetchVipStatus()
    } else if (done > 0) {
      adResult.value = `已看广告 ${done}/${limit} 次`
    } else {
      adError.value = '看广告未领取到时长，可能已达今日上限'
    }
  } catch (e) {
    adError.value = String(e)
  } finally {
    adLoading.value = false
  }
  return { ok: !adError.value, result: adResult.value, error: adError.value }
}
