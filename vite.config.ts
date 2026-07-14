import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'
import { fileURLToPath, URL } from 'node:url'

// Tauri 期望的固定开发端口，与 tauri.conf.json 中 devUrl 保持一致
const host = process.env.TAURI_DEV_HOST || 'localhost'
const port = 1420

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    // Element Plus 按需自动导入（API）
    AutoImport({
      resolvers: [ElementPlusResolver()]
    }),
    // Element Plus 按需自动注册（组件）
    Components({
      resolvers: [ElementPlusResolver()]
    })
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  // Tauri 要求的构建配置
  clearScreen: false,
  server: {
    port,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421
        }
      : undefined,
    watch: {
      // 忽略 Rust 侧变更，避免触发前端热更新
      ignored: ['**/src-tauri/**']
    }
  }
}))
