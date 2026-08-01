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
  optimizeDeps: {
    // 预构建「仅在部分页面使用」的 Element Plus 组件样式依赖。
    // 这些样式由 unplugin-vue-components 的 sideEffects 在运行时注入，
    // 若不预构建，首次访问对应页面时 Vite 会发现新依赖 → 重新优化 → 强制
    // full reload（曾导致「点下一步→白屏跳首页」）。
    // 必须写 style/css 路径（组件 index.mjs 不引用样式，写主入口无效）。
    // 生产构建不受影响。
    include: [
      // 基础样式（所有 EP 组件共用，必须最先）
      'element-plus/es/components/base/style/css',
      // 运行时新发现的组件样式（与 Vite 实际优化日志对齐）
      // 注意：必须覆盖全部懒加载页面首次进入时才会导入的组件样式，
      // 缺一个就会触发 Vite「新依赖→重新预构建→full-reload」，
      // 导致页面 reload、内存状态丢失、被导航守卫弹回首页。
      'element-plus/es/components/alert/style/css',
      'element-plus/es/components/button/style/css',
      'element-plus/es/components/dialog/style/css',
      'element-plus/es/components/divider/style/css',
      'element-plus/es/components/empty/style/css',
      'element-plus/es/components/form/style/css',
      'element-plus/es/components/form-item/style/css',
      'element-plus/es/components/input/style/css',
      'element-plus/es/components/input-number/style/css',
      'element-plus/es/components/radio/style/css',
      'element-plus/es/components/radio-group/style/css',
      'element-plus/es/components/switch/style/css',
      'element-plus/es/components/table/style/css',
      'element-plus/es/components/table-column/style/css',
      'element-plus/es/components/tag/style/css',
      // 服务式 API（ElMessage/ElMessageBox）的样式
      'element-plus/es/components/message/style/css',
      'element-plus/es/components/message-box/style/css'
    ]
  },
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
