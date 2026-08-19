<template>
  <Teleport to="body">
    <div v-if="showPanel" class="kg-overlay" @click.self="closePanel">
      <div class="kg-panel">
        <!-- Header -->
        <div class="kg-header">
          <span class="kg-title">酷狗扫码登录</span>
          <button class="kg-close" @click="closePanel">✕</button>
        </div>

        <!-- 已登录 -->
        <div v-if="isLoggedIn" class="kg-logged-in">
          <div class="kg-avatar">{{ nickname || userid?.[0] || 'U' }}</div>
          <div class="kg-user-info">
            <div class="kg-nick">{{ nickname || `用户${userid}` }}</div>
            <div class="kg-sub">ID: {{ userid }}</div>
          </div>
          <button class="kg-logout" @click="logout">退出</button>
        </div>

        <!-- 未登录 -->
        <div v-else class="kg-body">
          <!-- 加载中 -->
          <div v-if="isLoading" class="kg-state kg-loading">
            <div class="spinner" />
            <div>正在连接代理...</div>
          </div>

          <!-- 错误 -->
          <div v-else-if="error" class="kg-state kg-error">
            <div class="kg-err-icon">⚠️</div>
            <div class="kg-err-msg">{{ error }}</div>
            <div class="kg-err-hint">请确保已启动 KuGouMusicApi 代理</div>
            <button class="kg-retry" @click="refreshQr">重试</button>
          </div>

          <!-- 二维码 -->
          <div v-else-if="qrImg" class="kg-qr">
            <img :src="qrImg" alt="酷狗登录二维码" class="kg-qr-img" />
            <div class="kg-status" :class="statusClass">
              <span class="kg-dot" />
              {{ qrStatusText || '等待扫码...' }}
            </div>
            <button class="kg-refresh" @click="refreshQr">刷新二维码</button>
          </div>

          <!-- 未开始 -->
          <div v-else class="kg-state kg-start">
            <div class="kg-start-icon">🐶</div>
            <div>点击下方按钮生成登录二维码</div>
            <button class="kg-start-btn" @click="startQrLogin">生成二维码</button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  isLoggedIn,
  userid,
  nickname,
  qrImg,
  qrStatus,
  qrStatusText,
  isLoading,
  error,
  showPanel,
  startQrLogin,
  refreshQr,
  logout,
  closePanel,
  initKugouLogin,
} from '../composables/useKugouLogin'

const statusClass = computed(() => {
  switch (qrStatus.value) {
    case 0: return 'expired'
    case 1: return 'waiting'
    case 2: return 'confirm'
    case 4: return 'success'
    default: return 'idle'
  }
})

// 全局初始化一次
initKugouLogin()
</script>

<style scoped>
.kg-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}
.kg-panel {
  background: #1e1e2e;
  border: 1px solid #3a3a5c;
  border-radius: 14px;
  width: 320px;
  overflow: hidden;
  font-family: system-ui, -apple-system, sans-serif;
  box-shadow: 0 20px 60px rgba(0,0,0,0.5);
}

/* Header */
.kg-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid #2a2a3e;
  background: #16161e;
}
.kg-title { font-size: 15px; font-weight: 600; color: #e0e0ff; }
.kg-close {
  background: none;
  border: none;
  color: #666;
  font-size: 16px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.15s;
}
.kg-close:hover { background: #3a3a5c; color: #fff; }

/* 已登录 */
.kg-logged-in {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 20px 16px;
}
.kg-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: linear-gradient(135deg, #7c3aed, #db2777);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}
.kg-user-info { flex: 1; min-width: 0; }
.kg-nick { font-size: 14px; font-weight: 600; color: #e0e0ff; }
.kg-sub { font-size: 12px; color: #666; margin-top: 3px; word-break: break-all; }
.kg-logout {
  background: #2a2a3e;
  border: 1px solid #4a3a5c;
  border-radius: 6px;
  color: #aaa;
  font-size: 12px;
  padding: 6px 12px;
  cursor: pointer;
  transition: all 0.15s;
}
.kg-logout:hover { background: #3a2a2a; color: #f87171; border-color: #6b3a3a; }

/* Body */
.kg-body { padding: 24px 20px; }

/* State blocks */
.kg-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  text-align: center;
  color: #888;
  font-size: 13px;
}

/* Loading */
.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #2a2a3e;
  border-top-color: #7c3aed;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* Error */
.kg-err-icon { font-size: 36px; }
.kg-err-msg { color: #f87171; max-width: 260px; line-height: 1.4; }
.kg-err-hint { color: #555; font-size: 12px; margin-top: 2px; }
.kg-retry {
  background: #3a2a5a;
  border: none;
  border-radius: 6px;
  color: #c4b5fd;
  font-size: 13px;
  padding: 8px 20px;
  cursor: pointer;
  margin-top: 6px;
  transition: background 0.15s;
}
.kg-retry:hover { background: #5a3a7a; }

/* QR */
.kg-qr {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}
.kg-qr-img {
  width: 200px;
  height: 200px;
  border-radius: 10px;
  background: #fff;
  display: block;
  image-rendering: pixelated;
}
.kg-refresh {
  background: #2a2a3e;
  border: 1px solid #3a3a5c;
  border-radius: 6px;
  color: #888;
  font-size: 12px;
  padding: 6px 14px;
  cursor: pointer;
  transition: all 0.15s;
}
.kg-refresh:hover { background: #3a3a5c; color: #e0e0ff; }

/* Status badge */
.kg-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  padding: 6px 16px;
  border-radius: 20px;
  background: #2a2a3e;
  color: #888;
}
.kg-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #555;
  flex-shrink: 0;
}
.waiting .kg-dot { background: #f59e0b; animation: pulse 1.5s ease-in-out infinite; }
.waiting { color: #fbbf24; }
.confirm .kg-dot { background: #60a5fa; animation: pulse 1s ease-in-out infinite; }
.confirm { color: #60a5fa; }
.success .kg-dot { background: #34d399; }
.success { color: #34d399; }
.expired .kg-dot { background: #f87171; }
.expired { color: #f87171; }
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

/* Start */
.kg-start-icon { font-size: 44px; }
.kg-start-btn {
  background: linear-gradient(135deg, #7c3aed, #db2777);
  border: none;
  border-radius: 8px;
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  padding: 10px 24px;
  cursor: pointer;
  margin-top: 8px;
  transition: opacity 0.15s;
}
.kg-start-btn:hover { opacity: 0.88; }
</style>
