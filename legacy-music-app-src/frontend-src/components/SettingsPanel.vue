<template>
  <div class="settings-panel">
    <h2 class="panel-title">⚙️ 设置</h2>

    <!-- 库管理 -->
    <section class="settings-section">
      <h3 class="section-title">📚 库管理</h3>

      <!-- 现有库列表 -->
      <div v-if="libStore.libraries.length > 0" class="lib-list">
        <div
          v-for="lib in libStore.libraries"
          :key="lib.id"
          class="lib-card"
          :class="{ active: lib.active, master: lib.is_master }"
        >
          <div class="lib-info">
            <div class="lib-name">
              {{ lib.name }}
              <span v-if="lib.is_master" class="master-badge">主库</span>
              <span v-if="lib.active" class="active-badge">当前</span>
            </div>
            <div class="lib-path">{{ lib.path }}</div>
            <div class="lib-stats">
              {{ lib.folders?.length || 0 }} 文件夹 · {{ totalSongsInLib(lib) }} 首歌
            </div>
          </div>
          <div class="lib-actions">
            <button
              v-if="!lib.active"
              class="action-btn"
              @click="onSwitch(lib.id)"
            >切换</button>
            <button
              v-if="!lib.is_master"
              class="action-btn"
              @click="onSetMaster(lib.id)"
            >设主</button>
            <button
              v-if="libStore.libraries.length > 1 && !lib.active"
              class="action-btn danger"
              @click="onDelete(lib.id)"
            >删除</button>
          </div>
        </div>
      </div>

      <div v-else class="empty-hint">
        暂无音乐库，请创建第一个库
      </div>

      <!-- 新建库 -->
      <div class="create-lib-form">
        <h4>新建音乐库</h4>
        <div class="form-row">
          <input
            v-model="newLibName"
            placeholder="库名称"
            class="text-input"
          />
        </div>
        <div class="form-row">
          <input
            v-model="newLibPath"
            placeholder="音乐库目录路径"
            class="text-input"
          />
          <button class="small-btn" @click="selectLibPath">选择...</button>
        </div>
        <div class="form-row">
          <button class="small-btn primary" :disabled="!canCreate" @click="onCreate">
            创建空库
          </button>
          <!-- U2: 添加本地非主库（选文件夹 + 自动扫描入库，不复制文件） -->
          <button
            v-if="libStore.libraries.length > 0"
            class="small-btn primary"
            :disabled="libStore.importing || !canAddLocal"
            @click="onAddLocalLibrary"
          >
            {{ libStore.importing ? '扫描中...' : '添加本地库（不复制文件）' }}
          </button>
        </div>
        <p v-if="createError" class="error-msg">{{ createError }}</p>
        <p v-if="libStore.importProgress" class="scan-msg">{{ libStore.importProgress }}</p>
      </div>

      <!-- 合并库 -->
      <div v-if="libStore.libraries.length >= 2" class="merge-form">
        <h4>合并库</h4>
        <div class="form-row">
          <select v-model="mergeFrom" class="text-input">
            <option value="">源库...</option>
            <option v-for="lib in libStore.libraries" :key="lib.id" :value="lib.id">
              {{ lib.name }}
            </option>
          </select>
          <span class="merge-arrow">→</span>
          <select v-model="mergeTo" class="text-input">
            <option value="">目标库...</option>
            <option v-for="lib in libStore.libraries" :key="lib.id" :value="lib.id">
              {{ lib.name }}
            </option>
          </select>
        </div>
        <button class="small-btn" :disabled="!canMerge" @click="onMerge">
          合并
        </button>
      </div>
    </section>

    <!-- 音源登录 -->
    <section class="settings-section">
      <h3 class="section-title">🔐 音源登录</h3>
      <div class="source-row">
        <span class="source-label">🟠 酷狗</span>
        <template v-if="kugouLoggedIn">
          <span class="source-status">✅ 已登录 {{ kugouUsername }}</span>
          <button class="small-btn" :disabled="syncing" @click="doSyncKugou">
            {{ syncing ? '同步中...' : '同步收藏歌单' }}
          </button>
          <button class="small-btn danger-btn" @click="doKugouLogout">退出</button>
        </template>
        <p v-if="syncMsg" class="sync-msg">{{ syncMsg }}</p>
        <template v-else>
          <button class="small-btn" @click="doQrLogin">扫码登录</button>
          <button class="small-btn" @click="showPwdModal = true">账号登录</button>
        </template>
      </div>
    </section>

    <!-- 主题选择 -->
    <section class="settings-section">
      <h3 class="section-title">🎨 主题</h3>
      <div class="theme-grid">
        <div
          v-for="t in themes"
          :key="t.id"
          class="theme-card"
          :class="{ active: currentTheme === t.id }"
          @click="setTheme(t.id)"
        >
          <div class="theme-preview">
            <div class="tp-bg" :style="{ background: t.colors.bg }">
              <div class="tp-card" :style="{ background: t.colors.card, borderColor: t.colors.border }">
                <div class="tp-accent" :style="{ background: t.colors.accent }"></div>
                <div class="tp-line" :style="{ background: t.colors.muted }"></div>
                <div class="tp-line short" :style="{ background: t.colors.border }"></div>
              </div>
            </div>
          </div>
          <div class="theme-name">{{ t.name }}</div>
          <div class="theme-check" v-if="currentTheme === t.id">✓</div>
        </div>
      </div>
    </section>

    <!-- Kugou 扫码登录弹窗 -->
    <Teleport to="body">
      <div v-if="showQrModal" class="qr-overlay" @click.self="closeQrModal">
        <div class="qr-modal">
          <h4>酷狗扫码登录</h4>
          <div class="qr-wrap">
            <img v-if="qrDataUrl" :src="qrDataUrl" alt="二维码" class="qr-image" />
            <div v-else class="qr-placeholder">生成中...</div>
          </div>
          <p class="qr-status">{{ qrStatusText }}</p>
          <button class="small-btn" @click="closeQrModal">取消</button>
        </div>
      </div>
    </Teleport>

    <!-- 酷狗密码登录弹窗 -->
    <Teleport to="body">
      <div v-if="showPwdModal" class="qr-overlay" @click.self="closePwdModal">
        <div class="qr-modal">
          <h4>酷狗账号登录</h4>
          <div class="pwd-form">
            <input
              v-model="pwdUsername"
              class="text-input"
              placeholder="手机号"
              type="tel"
            />
            <input
              v-model="pwdPassword"
              class="text-input"
              placeholder="密码"
              type="password"
            />
            <p v-if="pwdError" class="error-msg">{{ pwdError }}</p>
            <div class="pwd-actions">
              <button
                class="small-btn primary"
                :disabled="pwdLoading"
                @click="doPwdLogin"
              >
                {{ pwdLoading ? '登录中...' : '登录' }}
              </button>
              <button class="small-btn" @click="closePwdModal">取消</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { open } from '@tauri-apps/plugin-dialog'
import QRCode from 'qrcode'

const libStore = useLibraryStore()

// ── Theme ──
const currentTheme = ref(document.documentElement.getAttribute('data-theme') || 'dark')

const themes = [
  // ── 纯CSS自定义主题 ──
  { id: 'dark',  name: '暗夜',  colors: { bg: '#0f172a', card: '#1e293b', accent: '#3b82f6', border: '#334155', muted: '#475569' } },
  { id: 'light', name: '皎白',  colors: { bg: '#f8fafc', card: '#ffffff', accent: '#2563eb', border: '#e2e8f0', muted: '#cbd5e1' } },
  { id: 'ocean', name: '深海',  colors: { bg: '#0a1628', card: '#0f2847', accent: '#14b8a6', border: '#155e75', muted: '#0f766e' } },
  { id: 'rose',  name: '玫瑰',  colors: { bg: '#1c101d', card: '#2d1b2e', accent: '#e879f9', border: '#4a2040', muted: '#701a75' } },
  { id: 'amber', name: '琥珀',  colors: { bg: '#1c160e', card: '#29221a', accent: '#f59e0b', border: '#4a3520', muted: '#78350f' } },
  // ── daisyUI 预设主题 ──
  { id: 'night',     name: '夜空',   colors: { bg: '#0f0f23', card: '#1a1a2e', accent: '#c678dd', border: '#3b3b5c', muted: '#565c7a' } },
  { id: 'synthwave', name: '霓虹',   colors: { bg: '#1a0a2e', card: '#2d1b4e', accent: '#ff71ce', border: '#6c2fb5', muted: '#c956df' } },
  { id: 'coffee',    name: '咖啡',   colors: { bg: '#1a1513', card: '#2d2017', accent: '#c8a27a', border: '#4a3528', muted: '#7a5c42' } },
  { id: 'autumn',    name: '秋枫',   colors: { bg: '#19110a', card: '#2a1a10', accent: '#e66a35', border: '#5c3020', muted: '#8b4a2f' } },
  { id: 'sunset',    name: '晚霞',   colors: { bg: '#1a1020', card: '#2d1a30', accent: '#f7797d', border: '#6b2f45', muted: '#a04560' } },
  { id: 'business',  name: '商务',   colors: { bg: '#17202a', card: '#1e2d38', accent: '#0078d4', border: '#2f4150', muted: '#4a6570' } },
  { id: 'lemonade',  name: '柠檬',   colors: { bg: '#f3f5e8', card: '#ffffff', accent: '#b5e313', border: '#d4e8a0', muted: '#8faa50' } },
  { id: 'emerald',   name: '翡翠',   colors: { bg: '#1a2419', card: '#243522', accent: '#00c853', border: '#2f5a2d', muted: '#3d7539' } },
]

async function setTheme(theme) {
  currentTheme.value = theme
  document.documentElement.setAttribute('data-theme', theme)
  try { localStorage.setItem('music-app-theme', theme) } catch {}
  // 通知歌词窗口同步主题
  try {
    const { emit } = await import('@tauri-apps/api/event')
    await emit('theme-changed', { theme })
  } catch {}
}

// ── 新建库 ──
const newLibName = ref('')
const newLibPath = ref('')
const createError = ref('')

const canCreate = computed(() => newLibName.value.trim() && newLibPath.value.trim())
const canAddLocal = computed(() => newLibName.value.trim() && newLibPath.value.trim())

// ── U2: 添加本地非主库（选文件夹 + 扫描入库，不复制文件） ──
async function onAddLocalLibrary() {
  createError.value = ''
  try {
    await libStore.scanLocalLibrary(newLibName.value.trim(), newLibPath.value.trim())
    newLibName.value = ''
    newLibPath.value = ''
    await libStore.loadLibraries()
  } catch (err) {
    createError.value = String(err)
  }
}

async function selectLibPath() {
  const selected = await open({
    multiple: false,
    directory: true,
    title: '选择音乐库目录',
  })
  if (selected) {
    newLibPath.value = selected
  }
}

async function onCreate() {
  createError.value = ''
  try {
    await libStore.createLibrary(newLibName.value.trim(), newLibPath.value.trim())
    newLibName.value = ''
    newLibPath.value = ''
  } catch (err) {
    createError.value = String(err)
  }
}

// ── 库操作 ──
async function onSwitch(id) {
  try {
    await libStore.switchLibrary(id)
  } catch (err) {
    console.error('[settings] switch error:', err)
  }
}

async function onDelete(id) {
  if (!confirm('确定删除此库？不动物理文件。')) return
  try {
    await libStore.deleteLibrary(id)
  } catch (err) {
    console.error('[settings] delete error:', err)
  }
}

async function onSetMaster(id) {
  try {
    await libStore.setMasterLibrary(id)
  } catch (err) {
    console.error('[settings] setMaster error:', err)
  }
}

// ── 合并库 ──
const mergeFrom = ref('')
const mergeTo = ref('')

const canMerge = computed(() =>
  mergeFrom.value && mergeTo.value && mergeFrom.value !== mergeTo.value
)

async function onMerge() {
  if (!confirm('确定将源库合并到目标库？源库将被移除。')) return
  try {
    await libStore.mergeLibrary(mergeFrom.value, mergeTo.value)
    mergeFrom.value = ''
    mergeTo.value = ''
  } catch (err) {
    console.error('[settings] merge error:', err)
  }
}

// ── 统计 ──
function totalSongsInLib(lib) {
  let total = 0
  for (const f of lib.folders || []) {
    total += (f.song_ids || []).length
  }
  return total
}

const validThemes = themes.map(t => t.id)

onMounted(() => {
  libStore.loadLibraries()
  checkKugouStatus()
  // Init theme from localStorage
  const saved = localStorage.getItem('music-app-theme')
  if (saved && validThemes.includes(saved)) {
    setTheme(saved)
  }
})

// ── 酷狗登录 ──
const apiBase = ref('http://127.0.0.1:8899')

;(function getPort() {
  const params = new URLSearchParams(window.location.search)
  const port = params.get('port')
  if (port) apiBase.value = `http://127.0.0.1:${port}`
})()

const kugouLoggedIn = ref(false)
const kugouUsername = ref('')

async function checkKugouStatus() {
  try {
    const resp = await fetch(`${apiBase.value}/api/kugou/status`)
    const data = await resp.json()
    kugouLoggedIn.value = data.logged_in
    kugouUsername.value = data.username || ''
  } catch (_) {}
}

async function doKugouLogout() {
  // 先调后端清除 token
  try { await fetch(`${apiBase.value}/api/logout`, { method: 'POST' }) } catch {}
  await fetch(`${apiBase.value}/api/kugou/logout`, { method: 'POST' })
  kugouLoggedIn.value = false
  kugouUsername.value = ''
}

// ── 扫码登录 ──
const showQrModal = ref(false)
const qrDataUrl = ref('')
const qrStatusText = ref('')
let qrPollTimer = null
let qrKey = ''

async function doQrLogin() {
  showQrModal.value = true
  qrDataUrl.value = ''
  qrStatusText.value = '正在获取二维码...'
  try {
    const resp = await fetch(`${apiBase.value}/api/kugou/qr-key`)
    const data = await resp.json()
    if (!data.key) { qrStatusText.value = '获取二维码失败'; return }
    qrKey = data.key
    const qrUrl = `https://h5.kugou.com/apps/loginQRCode/html/index.html?qrcode=${qrKey}`
    qrDataUrl.value = await QRCode.toDataURL(qrUrl)
    qrStatusText.value = '请用酷狗APP扫码'
    startQrPoll()
  } catch (e) {
    qrStatusText.value = '获取二维码失败'
  }
}

function startQrPoll() {
  if (qrPollTimer) clearInterval(qrPollTimer)
  let elapsed = 0
  qrPollTimer = setInterval(async () => {
    elapsed += 2
    if (elapsed > 300) { qrStatusText.value = '二维码已超时'; stopQrPoll(); return }
    try {
      const resp = await fetch(`${apiBase.value}/api/kugou/qr-check?key=${qrKey}`)
      const data = await resp.json()
      const status = data.status ?? data.data?.status
      if (status === 4) {
        qrStatusText.value = '登录成功！'
        stopQrPoll()
        setTimeout(() => { showQrModal.value = false; checkKugouStatus() }, 800)
      } else if (status === 0) {
        qrStatusText.value = '二维码已过期'; stopQrPoll()
      } else if (status === 2) {
        qrStatusText.value = '已扫码，请确认...'
      }
    } catch (_) {}
  }, 2000)
}

function stopQrPoll() {
  if (qrPollTimer) { clearInterval(qrPollTimer); qrPollTimer = null }
}

function closeQrModal() { stopQrPoll(); showQrModal.value = false; qrKey = '' }

// ── 密码登录 ──
const showPwdModal = ref(false)
const pwdUsername = ref('')
const pwdPassword = ref('')
const pwdError = ref('')
const pwdLoading = ref(false)

async function doPwdLogin() {
  pwdError.value = ''
  if (!pwdUsername.value.trim() || !pwdPassword.value) {
    pwdError.value = '请输入手机号和密码'; return
  }
  pwdLoading.value = true
  try {
    const resp = await fetch(`${apiBase.value}/api/kugou/login-pwd`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: pwdUsername.value.trim(), password: pwdPassword.value })
    })
    const data = await resp.json()
    if (data.ok) { closePwdModal(); await checkKugouStatus() }
    else { pwdError.value = data.error || '登录失败' }
  } catch (e) {
    pwdError.value = '网络错误'
  } finally { pwdLoading.value = false }
}

function closePwdModal() {
  showPwdModal.value = false; pwdUsername.value = ''; pwdPassword.value = ''; pwdError.value = ''
}

// P8-2: 同步酷狗收藏歌单
const syncing = ref(false)
const syncMsg = ref('')
async function doSyncKugou() {
  syncing.value = true; syncMsg.value = '正在拉取歌单...'
  try {
    const resp = await fetch(`${apiBase.value}/api/kugou/sync-playlists`, { method: 'POST' })
    const data = await resp.json()
    if (data.synced !== undefined) {
      syncMsg.value = `已同步 ${data.synced} 首歌、${data.playlists.length} 个歌单`
    } else {
      syncMsg.value = data.error || '同步失败'
    }
  } catch(e) { syncMsg.value = '网络错误' }
  finally { syncing.value = false }
}

onUnmounted(() => { stopQrPoll() })
</script>

<style scoped>
.settings-panel {
  max-width: 640px;
}

.panel-title {
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 16px 0;
  color: var(--text);
}

.settings-section {
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border);
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--text);
}

.muted {
  font-size: 12px;
  color: var(--muted);
}

.empty-hint {
  font-size: 13px;
  color: var(--muted);
  padding: 12px 0;
}

/* ── 库卡片 ── */
.lib-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.lib-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  gap: 12px;
}

.lib-card.active {
  border-color: var(--accent);
}

.lib-card.master {
  border-left: 3px solid var(--accent);
}

.lib-info {
  flex: 1;
  min-width: 0;
}

.lib-name {
  font-size: 13px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text);
}

.master-badge {
  font-size: 10px;
  padding: 1px 5px;
  background: var(--accent);
  color: #fff;
  border-radius: 3px;
}

.active-badge {
  font-size: 10px;
  padding: 1px 5px;
  background: #22c55e;
  color: #fff;
  border-radius: 3px;
}

.lib-path {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lib-stats {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
}

.lib-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.action-btn {
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font-size: 11px;
  transition: all 0.15s;
}

.action-btn:hover {
  background: var(--accent);
  color: #fff;
}

.action-btn.danger:hover {
  background: #e94560;
  border-color: #e94560;
}

/* ── 创建表单 ── */
.create-lib-form, .merge-form {
  margin-top: 12px;
  padding: 12px;
  background: var(--bg);
  border-radius: var(--radius);
  border: 1px solid var(--border);
}

.create-lib-form h4, .merge-form h4 {
  font-size: 12px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: var(--text);
}

.form-row {
  display: flex;
  gap: 6px;
  margin-bottom: 8px;
  align-items: center;
}

.text-input {
  flex: 1;
  padding: 5px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  font-size: 12px;
}

.text-input:focus {
  outline: none;
  border-color: var(--accent);
}

.merge-arrow {
  color: var(--muted);
  font-size: 14px;
}

.small-btn {
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--card);
  color: var(--text);
  cursor: pointer;
  font-size: 11px;
  transition: all 0.15s;
  white-space: nowrap;
}

.small-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}

.small-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.small-btn.primary {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}

.error-msg {
  font-size: 11px;
  color: #e94560;
  margin-top: 4px;
}

.scan-msg {
  font-size: 11px;
  color: var(--accent);
  margin-top: 4px;
}

/* ── 酷狗登录 ── */
.source-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
}

.source-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.source-status {
  font-size: 12px;
  color: #22c55e;
  flex: 1;
}

.danger-btn {
  color: #e94560;
  border-color: #e94560;
}

.sync-msg {
  font-size: 12px;
  color: var(--accent);
  margin-top: 4px;
}

/* ── 弹窗 ── */
.qr-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.qr-modal {
  background: var(--card);
  border-radius: 12px;
  padding: 24px;
  text-align: center;
  min-width: 280px;
  max-width: 320px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.qr-modal h4 {
  margin: 0 0 16px 0;
  font-size: 15px;
  color: var(--text);
}

.qr-wrap {
  width: 200px;
  height: 200px;
  margin: 0 auto 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #fff;
  border-radius: 8px;
}

.qr-image {
  width: 180px;
  height: 180px;
}

.qr-placeholder {
  color: #999;
  font-size: 12px;
}

.qr-status {
  font-size: 12px;
  color: var(--muted);
  margin: 8px 0;
}

.pwd-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.pwd-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}

/* ── 主题选择器（可视化卡片） ── */
.theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 10px;
}

.theme-card {
  position: relative;
  cursor: pointer;
  border-radius: var(--radius);
  border: 2px solid transparent;
  overflow: hidden;
  transition: border-color 0.2s, transform 0.15s;
  padding: 0;
}

.theme-card:hover {
  transform: translateY(-2px);
  border-color: var(--muted);
}

.theme-card.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.theme-preview {
  width: 100%;
  aspect-ratio: 16 / 10;
  overflow: hidden;
}

.tp-bg {
  width: 100%;
  height: 100%;
  padding: 8px;
  display: flex;
  align-items: flex-start;
  justify-content: center;
}

.tp-card {
  width: 100%;
  height: 100%;
  border-radius: 4px;
  border: 1px solid;
  padding: 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.tp-accent {
  width: 18px;
  height: 4px;
  border-radius: 2px;
}

.tp-line {
  width: 100%;
  height: 3px;
  border-radius: 2px;
  opacity: 0.5;
}

.tp-line.short {
  width: 60%;
  opacity: 0.3;
}

.theme-name {
  text-align: center;
  font-size: 12px;
  padding: 6px 0;
  color: var(--text);
  font-weight: 500;
}

.theme-check {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 18px;
  height: 18px;
  background: var(--accent);
  color: #fff;
  border-radius: 50%;
  font-size: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
}
</style>
