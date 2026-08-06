import { defineConfig } from '@vben/vite-config';

/**
 * 若依适配说明：
 * - bootstrap.ts 已全局 app.use(ElementPlus) + 引入 element-plus/dist/index.css
 * - 禁止再使用 unplugin-element-plus 按需注入 `.../style/css`
 *   否则开发态切换菜单时 Vite 会不断 `new dependencies optimized` → `reloading`，
 *   表现为整页刷新（与路由无关）。
 * - 生产 API 前缀见 .env.production：VITE_GLOB_API_URL=/prod-api（与 ruoyi-ui 的
 *   VUE_APP_BASE_API、Nginx location /prod-api/ 同一套模板）。
 * - 打包产物文件名带 hash，输出到 static/，与 ruoyi-ui assetsDir 习惯一致。
 */
export default defineConfig(async () => {
  return {
    application: {},
    vite: {
      build: {
        rollupOptions: {
          output: {
            assetFileNames: 'static/[ext]/[name]-[hash].[ext]',
            chunkFileNames: 'static/js/[name]-[hash].js',
            entryFileNames: 'static/js/[name]-[hash].js',
          },
        },
      },
      server: {
        port: 5777,
        proxy: {
          '/api': {
            changeOrigin: true,
            // 若依后端接口无 /api 前缀，rewrite 去掉 /api 后正好命中
            rewrite: (path) => path.replace(/^\/api/, ''),
            // 代理目标：本地若依后端（springboot3，端口 14001）
            target: '{{API_BASE_URL_DEV}}',
            ws: true,
          },
          // SpringDoc Swagger UI 的 configUrl / urls 是绝对路径 /v3/api-docs/**，
          // 不会走 /api 前缀；需单独代理（与 ruoyi-ui vue.config.js 一致），
          // 否则 iframe 内嵌系统接口页会 404。
          '/v3/api-docs': {
            changeOrigin: true,
            target: '{{API_BASE_URL_DEV}}',
          },
        },
      },
    },
  };
});