import { createApp, watchEffect } from 'vue';

import { registerAccessDirective } from '@vben/access';
import { preferences } from '@vben/preferences';
import { initStores } from '@vben/stores';
import '@vben/styles';
import '@vben/styles/ele';

import { useTitle } from '@vueuse/core';
// 全局注册 Element Plus：若依业务页（system/monitor 等）大量使用小写 <el-xxx> 标签，
// vben 原工程为追求 tree-shaking 仅按需 import，但移植若依页面时手写 import 极易遗漏
// （el-tree / el-dialog / el-tree-select 等会导致 "Failed to resolve component" 报错刷屏）。
// 这里改为全局注册 + 引入完整样式，与若依 ruoyi-ui 原生做法一致。
import ElementPlus, { ElLoading } from 'element-plus';
import 'element-plus/dist/index.css';

import { $t, setupI18n } from '#/locales';

import { initComponentAdapter } from './adapter/component';
import App from './app.vue';
import { router } from './router';
import { hasPermi, hasRole } from './directives/hasPermi';

async function bootstrap(namespace: string) {
  // 初始化组件适配器
  await initComponentAdapter();
  const app = createApp(App);

  // 全局注册 Element Plus（见文件顶部注释）
  app.use(ElementPlus);

  // 注册Element Plus提供的v-loading指令
  app.directive('loading', ElLoading.directive);

  // 国际化 i18n 配置
  await setupI18n(app);

  // 配置 pinia-tore
  await initStores(app, { namespace });

  // 安装权限指令
  registerAccessDirective(app);

  // 注册若依权限指令（v-hasPermi / v-hasRole，对应若依按钮级权限）
  app.directive('hasPermi', hasPermi);
  app.directive('hasRole', hasRole);

  // 配置路由及路由守卫
  app.use(router);

  // 动态更新标题
  watchEffect(() => {
    if (preferences.app.dynamicTitle) {
      const routeTitle = router.currentRoute.value.meta?.title;
      const pageTitle =
        (routeTitle ? `${$t(routeTitle)} - ` : '') + preferences.app.name;
      useTitle(pageTitle);
    }
  });

  app.mount('#app');
}

export { bootstrap };
