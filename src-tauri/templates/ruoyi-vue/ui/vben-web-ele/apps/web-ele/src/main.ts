import { initPreferences, updatePreferences } from '@vben/preferences';
import { unmountGlobalLoading } from '@vben/utils';

import { overridesPreferences } from './preferences';

/**
 * 应用初始化完成之后再进行页面加载渲染
 */
async function initApplication() {
  // name用于指定项目唯一标识
  // 用于区分不同项目的偏好设置以及存储数据的key前缀以及其他一些需要隔离的数据
  const env = import.meta.env.PROD ? 'prod' : 'dev';
  const appVersion = import.meta.env.VITE_APP_VERSION;
  const namespace = `${import.meta.env.VITE_APP_NAMESPACE}-${appVersion}-${env}`;

  // app偏好设置初始化
  await initPreferences({
    namespace,
    overrides: overridesPreferences,
  });

  // 标题/版权以源码与 .env 为准，覆盖 localStorage 旧缓存
  // （否则会长期显示「Vben Admin Ele」等历史品牌名）
  updatePreferences({
    app: {
      name: import.meta.env.VITE_APP_TITLE,
    },
    copyright: overridesPreferences.copyright,
  });

  // 动态版权：从后端获取起始年份与 ICP 备案号（免登录公开接口）。
  // 不 await——后端未就绪时保留上面的构建期静态版权，也不阻塞启动。
  void syncCopyrightFromServer();

  // 启动应用并挂载
  const { bootstrap } = await import('./bootstrap');
  await bootstrap(namespace);

  // 移除并销毁loading
  unmountGlobalLoading();
}

/**
 * 请求 /webInfo 同步动态版权。
 * 后端返回 { code: 200, data: { copyrightYear, icp } }：
 * - copyrightYear：起始年，与当前年不同则显示区间（如 2026-2027）
 * - icp：ICP 备案号（application.yaml 的 ruoyi.icp，备案通过后改配置重启即生效）
 */
async function syncCopyrightFromServer() {
  try {
    const baseUrl = import.meta.env.VITE_GLOB_API_URL || '/prod-api';
    const res = await fetch(`${baseUrl}/webInfo`);
    const body: { code?: number; data?: { copyrightYear?: string; icp?: string } } =
      await res.json();
    if (body?.code !== 200 || !body.data) {
      return;
    }
    const now = new Date().getFullYear();
    const start = Number.parseInt(body.data.copyrightYear ?? '', 10) || now;
    const date = now > start ? `${start}-${now}` : `${now}`;
    const title = import.meta.env.VITE_APP_TITLE || '';
    updatePreferences({
      copyright: {
        date,
        companyName: `${title}. All Rights Reserved.`,
        icp: body.data.icp || '',
        icpLink: 'https://beian.miit.gov.cn/',
      },
    });
  } catch {
    // 后端未就绪/接口异常：静默保留静态版权
  }
}

initApplication();
