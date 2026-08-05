import { defineConfig } from '@vben/vite-config';

import ElementPlus from 'unplugin-element-plus/vite';

export default defineConfig(async () => {
  return {
    application: {},
    vite: {
      plugins: [
        ElementPlus({
          format: 'esm',
        }),
      ],
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
        },
      },
    },
  };
});
