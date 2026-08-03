import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Tauri 需要监听所有接口
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
  },
  // 清除前缀，Tauri 用文件路径
  base: './',
  build: {
    outDir: 'dist',
    rollupOptions: {
      input: {
        main: 'index.html',
        player: 'player.html',
      },
    },
  },
})
