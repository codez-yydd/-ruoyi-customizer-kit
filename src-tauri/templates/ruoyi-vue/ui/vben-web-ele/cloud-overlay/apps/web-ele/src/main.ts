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

  // 动态站点信息：从后端获取站点标题/Logo 与版权年份、ICP 备案号（免登录公开接口），
  // 「后台设置 → 站点设置」页面保存后即时生效。不 await——后端未就绪时保留上面的
  // 构建期静态值，也不阻塞启动。
  void syncSiteInfoFromServer();

  // 启动应用并挂载
  const { bootstrap } = await import('./bootstrap');
  await bootstrap(namespace);

  // 移除并销毁loading
  unmountGlobalLoading();
}

/**
 * 请求 /system/webInfo 同步动态站点信息。
 * 后端返回 { code: 200, data: { copyrightYear, icp, title, logo } }：
 * - title / logo：站点标题与后台 Logo（后台设置页维护，空值跳过、保留静态默认）
 * - copyrightYear：版权起始年，与当前年不同则显示区间（如 2026-2027）
 * - icp：ICP 备案号（后台设置页 DB 值优先，回退 application.yaml 的 ruoyi.icp）
 */
async function syncSiteInfoFromServer() {
  try {
    const baseUrl = import.meta.env.VITE_GLOB_API_URL || '/prod-api';
    const res = await fetch(`${baseUrl}/system/webInfo`);
    const body: {
      code?: number;
      data?: { copyrightYear?: string; icp?: string; logo?: string; title?: string };
    } = await res.json();
    if (body?.code !== 200 || !body.data) {
      return;
    }
    const { copyrightYear, icp, logo, title } = body.data;

    const now = new Date().getFullYear();
    const start = Number.parseInt(copyrightYear ?? '', 10) || now;
    const date = now > start ? `${start}-${now}` : `${now}`;
    const siteTitle = title || import.meta.env.VITE_APP_TITLE || '';

    updatePreferences({
      app: {
        name: siteTitle,
      },
      // 上传的 Logo 为 /profile/upload/... 相对路径，需带接口前缀访问后端静态资源
      logo: logo ? { source: `${baseUrl}${logo}` } : {},
      copyright: {
        date,
        companyName: `${siteTitle}. All Rights Reserved.`,
        icp: icp || '',
        icpLink: icp ? 'https://beian.miit.gov.cn/' : '',
      },
    });
  } catch {
    // 后端未就绪/接口异常：静默保留静态值
  }
}

initApplication();
