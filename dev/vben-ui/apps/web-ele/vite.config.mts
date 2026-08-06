import { defineConfig } from '@vben/vite-config';

/**
 * 若依适配说明：
 * - bootstrap.ts 已全局 app.use(ElementPlus) + 引入 element-plus/dist/index.css
 * - 禁止再使用 unplugin-element-plus 按需注入 `.../style/css`
 *   否则开发态切换菜单时 Vite 会不断 `new dependencies optimized` → `reloading`，
 *   表现为整页刷新（与路由无关）。
 */
export default defineConfig(async () => {
  return {
    application: {},
    vite: {
      server: {
        port: 5777,
        proxy: {
          '/api': {
            changeOrigin: true,
            // 若依后端接口无 /api 前缀，rewrite 去掉 /api 后正好命中
            rewrite: (path) => path.replace(/^\/api/, ''),
            // 代理目标：本地若依后端（springboot3，端口 14001）
            target: 'http://localhost:14001',
            ws: true,
          },
          // SpringDoc Swagger UI 的 configUrl / urls 是绝对路径 /v3/api-docs/**，
          // 不会走 /api 前缀；需单独代理（与 ruoyi-ui vue.config.js 一致），
          // 否则 iframe 内嵌系统接口页会 404。
          '/v3/api-docs': {
            changeOrigin: true,
            target: 'http://localhost:14001',
          },
        },
      },
    },
  };
});
