import { fileURLToPath, URL } from 'node:url'
import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), 'VITE_')
  const port = Number(env.VITE_APP_PORT || 5778)

  return {
    plugins: [vue()],
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url))
      }
    },
    server: {
      host: 'localhost',
      port,
      open: false,
      proxy: {
        '/api': {
          target: 'http://localhost:14001',
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api/, '')
        },
        // swagger-ui 页面内的 configUrl/api-docs 为根路径绝对地址，需独立代理
        '/v3/api-docs': {
          target: 'http://localhost:14001',
          changeOrigin: true
        },
        '/webjars': {
          target: 'http://localhost:14001',
          changeOrigin: true
        }
      }
    },
    build: {
      outDir: 'dist',
      sourcemap: false,
      chunkSizeWarningLimit: 2048
    }
  }
})
