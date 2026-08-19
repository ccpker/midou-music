/**
 * Composable: useDiagnostics
 * 路径: src/composables/useDiagnostics.ts
 * ────────────────────────────────────────────────────────────
 * 功能: 全 app 最小功能独立性检查 + 全链路诊断
 *
 * 设计原则:
 *   - 不依赖外部代理（KuGouMusicApi 已用 Rust 内置）
 *   - 按音源逐个跑：登录状态 → 搜索 → 播放 → 收藏
 *   - 一键全链路测试 + 报告
 *   - 不动现有业务代码（diagnostic only）
 */

import { invoke } from '@tauri-apps/api/core'

export interface CheckItem {
  name: string
  source: string        // kuwo / bili / kugou / qq
  category: 'auth' | 'search' | 'play' | 'fav'
  status: 'pass' | 'fail' | 'skip' | 'pending'
  duration_ms?: number
  detail?: string
}

export interface CheckReport {
  started_at: string
  finished_at?: string
  total: number
  passed: number
  failed: number
  skipped: number
  items: CheckItem[]
}

// ── 单项测试工具 ───────────────────────────────────

async function timeIt<T>(fn: () => Promise<T>): Promise<{ ok: true; data: T; ms: number } | { ok: false; err: string; ms: number }> {
  const t0 = performance.now()
  try {
    const data = await fn()
    return { ok: true, data, ms: Math.round(performance.now() - t0) }
  } catch (e) {
    return { ok: false, err: String(e), ms: Math.round(performance.now() - t0) }
  }
}

function record(item: CheckItem, res: { ok: boolean; ms: number; err?: string; data?: any }): CheckItem {
  return {
    ...item,
    status: res.ok ? 'pass' : 'fail',
    duration_ms: res.ms,
    detail: res.ok ? (res.data ? JSON.stringify(res.data).slice(0, 200) : 'OK') : res.err,
  }
}

// ── 主诊断函数 ─────────────────────────────────────

export async function runFullDiagnostic(): Promise<CheckReport> {
  const items: CheckItem[] = []
  const start = new Date().toISOString()

  // ── 阶段 1: IPC 链路 ──────────────────────
  items.push(await checkIpcPing())

  // ── 阶段 2: 各音源独立检查 ────────────────
  const sources = ['kuwo', 'bili', 'kugou']
  for (const src of sources) {
    items.push(await checkSearch(src, '周杰伦'))
    if (src === 'kuwo' || src === 'bili') {
      // 酷狗播放需要登录（除匿名可听的少数）
      items.push(await checkAuthStatus(src))
    }
    if (src === 'kugou') {
      items.push(await checkAuthStatus(src))
    }
  }

  // ── 阶段 3: 酷狗扫码（如果未登录则触发） ─
  const kugouAuthItem = items.find(i => i.source === 'kugou' && i.category === 'auth')
  if (kugouAuthItem?.status === 'fail') {
    items.push(await checkKugouQrLogin())
  }

  // ── 阶段 4: 播放链接（拿第一首） ────────
  for (const src of sources) {
    const searchItem = items.find(i => i.source === src && i.category === 'search' && i.status === 'pass')
    if (searchItem?.detail) {
      try {
        const parsed = JSON.parse(searchItem.detail)
        const firstSong = parsed?.[0] || parsed?.data?.[0]
        if (firstSong) {
          items.push(await checkPlayUrl(src, firstSong))
        } else {
          items.push({
            name: `${src} 播放`,
            source: src,
            category: 'play',
            status: 'skip',
            detail: '搜索无结果，跳过播放',
          })
        }
      } catch {
        items.push({
          name: `${src} 播放`,
          source: src,
          category: 'play',
          status: 'skip',
          detail: '解析搜索结果失败',
        })
      }
    }
  }

  // ── 汇总 ──────────────────────────────────
  const passed = items.filter(i => i.status === 'pass').length
  const failed = items.filter(i => i.status === 'fail').length
  const skipped = items.filter(i => i.status === 'skip').length

  return {
    started_at: start,
    finished_at: new Date().toISOString(),
    total: items.length,
    passed,
    failed,
    skipped,
    items,
  }
}

// ── 具体检查项 ─────────────────────────────────────

async function checkIpcPing(): Promise<CheckItem> {
  const item: CheckItem = { name: 'IPC 链路', source: '-', category: 'auth', status: 'pending' }
  const res = await timeIt(async () => {
    // 用 kugou_auth_status 当 ping（最轻量）
    return await invoke('kugou_auth_status')
  })
  return record(item, res)
}

async function checkSearch(source: string, keyword: string): Promise<CheckItem> {
  const item: CheckItem = { name: `${source} 搜索 "${keyword}"`, source, category: 'search', status: 'pending' }
  const res = await timeIt(async () => {
    return await invoke('search', { source, keyword, page: 1 })
  })
  return record(item, res)
}

async function checkAuthStatus(source: string): Promise<CheckItem> {
  const item: CheckItem = { name: `${source} 登录状态`, source, category: 'auth', status: 'pending' }
  if (source === 'kugou') {
    const res = await timeIt(async () => {
      return await invoke('kugou_auth_status')
    })
    return record(item, res)
  }
  // 其他源暂未实现登录
  return { ...item, status: 'skip', detail: '未实现登录态查询' }
}

async function checkKugouQrLogin(): Promise<CheckItem> {
  const item: CheckItem = { name: '酷狗扫码二维码生成', source: 'kugou', category: 'auth', status: 'pending' }
  const res = await timeIt(async () => {
    return await invoke('kugou_qr_key')
  })
  return record(item, res)
}

async function checkPlayUrl(source: string, song: any): Promise<CheckItem> {
  const item: CheckItem = { name: `${source} 播放链接`, source, category: 'play', status: 'pending' }
  // 不同源 song 结构不同，构造正确的 song_id 格式
  let songId = song.song_id || song.id || song.hash || song.songmid
  if (!songId) {
    return { ...item, status: 'skip', detail: '歌曲无 id 字段' }
  }
  // 酷狗需要加前缀
  if (source === 'kugou' && !songId.startsWith('kugou:')) {
    songId = `kugou:${songId}`
  }
  const res = await timeIt(async () => {
    return await invoke('play_url', { songId })
  })
  return record(item, res)
}

// ── 报告格式化 ─────────────────────────────────────

export function formatReport(r: CheckReport): string {
  const lines: string[] = []
  lines.push(`═══ 全链路诊断报告 ═══`)
  lines.push(`开始: ${r.started_at}`)
  lines.push(`结束: ${r.finished_at}`)
  lines.push(`汇总: ${r.passed}✅ ${r.failed}❌ ${r.skipped}⏭  / 总 ${r.total}`)
  lines.push(``)
  for (const it of r.items) {
    const icon = it.status === 'pass' ? '✅' : it.status === 'fail' ? '❌' : '⏭'
    const ms = it.duration_ms ? ` (${it.duration_ms}ms)` : ''
    lines.push(`${icon} [${it.source}/${it.category}] ${it.name}${ms}`)
    if (it.detail && it.status !== 'pass') {
      lines.push(`   → ${it.detail.slice(0, 150)}`)
    }
  }
  return lines.join('\n')
}