/**
 * Composable: useKugouVip
 * 路径: src/composables/useKugouVip.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 酷狗签到领 VIP + VIP 状态查询
 *
 * 机制（moekoe 同款）:
 *   扫码 web token → 签到 receive_vip_listen_song → 账号获得
 *   概念版 tvip（1 天）→ 就能播 VIP 完整版
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

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

// ── 查询 VIP 状态 ─────────────────────────────────────

export async function fetchVipStatus() {
  try {
    const raw = await invoke('kugou_vip_status') as any
    const data = raw?.data || {}
    // 从 busi_vip 数组里找 tvip 权益
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
      // 刷新 VIP 状态
      await fetchVipStatus()
    } else if (code === 131001) {
      signError.value = '今天已经签到过了'
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
