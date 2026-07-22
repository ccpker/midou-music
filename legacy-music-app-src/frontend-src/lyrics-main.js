/**
 * 歌词窗口入口 — Phase 4
 * 独立 Vite 入口，挂载 LyricsWindow 组件
 */
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import LyricsWindow from './components/LyricsWindow.vue'

const app = createApp(LyricsWindow)
app.use(createPinia())
app.mount('#lyrics-app')
