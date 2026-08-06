import type { RouteRecordStringComponent } from '@vben/types';

import { requestClient } from '#/api/request';

/**
 * 若依 /getRouters 返回的原始菜单结构（RouterVo）
 *
 * 字段说明：
 * - name：路由名（springboot3 版优先取菜单 routeName 字段，已是英文；为空则取 path 首字母大写）
 * - path：路由地址
 * - component：组件路径，特殊值 Layout / ParentView / InnerLink，或 system/user/index 这种相对 views 的路径
 * - hidden：是否隐藏
 * - redirect：重定向（若依目录常用占位值 noRedirect，表示侧栏只展开不跳转）
 * - query：路由参数（JSON 字符串）
 * - alwaysShow：始终显示根路由
 * - meta：{ title, icon, noCache, link }
 * - children：子菜单（递归）
 */
interface RuoYiRouter {
  name: string;
  path: string;
  hidden: boolean;
  redirect?: string;
  component?: string;
  query?: string;
  alwaysShow?: boolean;
  meta?: {
    title: string;
    icon?: string;
    noCache?: boolean;
    link?: string;
  };
  children?: RuoYiRouter[];
}

/**
 * 判断是否为外链（http/https 开头）。
 * 若依外链菜单会把完整 URL 直接放在 path 字段（如 http://ruoyi.vip），
 * 而 vue-router 要求 path 必须以 "/" 开头，直接透传会导致 addRoute 抛错。
 */
function isExternalLink(p?: string): boolean {
  return !!p && /^https?:\/\//i.test(p);
}

/**
 * 拼接父子路由 path，兼容相对路径与已是绝对路径的子 path。
 */
function joinRoutePath(parentPath: string, childPath: string): string {
  if (!childPath) return parentPath || '/';
  if (isExternalLink(childPath) || childPath.startsWith('/')) {
    return childPath;
  }
  const base = parentPath.endsWith('/') ? parentPath.slice(0, -1) : parentPath;
  return `${base || ''}/${childPath}`.replace(/\/+/g, '/');
}

/**
 * 若依遗留图标短名 → Element Plus 图标（ep:xxx）映射。
 *
 * 背景：若依原生 ruoyi-ui 用本地 SVG 图标目录（src/assets/icons/svg），数据库存的是
 * 短名（system / user / monitor 等）。vben 侧边栏通过 VbenIcon 渲染，只认 Iconify 全名
 * （prefix:name，如 ep:user），所以这些短名直接传过去无法显示。
 * 这里做一层归一化：已含 ":" 的（新建菜单选的 ep:xxx）原样返回，遗留短名查表映射。
 * 未命中则返回空串（不显示图标，但不报错）。
 */
const RUOYI_ICON_MAP: Record<string, string> = {
  system: 'ep:tools',
  monitor: 'ep:monitor',
  tool: 'ep:set-up',
  user: 'ep:user',
  peoples: 'ep:user-filled',
  role: 'ep:user',
  'tree-table': 'ep:grid',
  tree: 'ep:share',
  menu: 'ep:menu',
  dept: 'ep:office-building',
  post: 'ep:postcard',
  dict: 'ep:document',
  date: 'ep:calendar',
  edit: 'ep:edit',
  log: 'ep:document',
  logininfor: 'ep:document',
  message: 'ep:bell',
  server: 'ep:cpu',
  sql: 'ep:data-line',
  druid: 'ep:coin',
  online: 'ep:user-filled',
  job: 'ep:alarm-clock',
  chart: 'ep:data-line',
  build: 'ep:tools',
  code: 'ep:document-copy',
  swagger: 'ep:link',
  guide: 'ep:link',
  eye: 'ep:view',
  'eye-open': 'ep:view',
  form: 'ep:document',
  number: 'ep:document',
  tab: 'ep:document',
  table: 'ep:grid',
  nested: 'ep:share',
  bug: 'ep:warning',
  star: 'ep:star',
  validCode: 'ep:key',
  wechat: 'ep:chat-dot-round',
  redis: 'ep:data-board',
  list: 'ep:list',
  lock: 'ep:lock',
  slider: 'ep:set-up',
  skill: 'ep:star',
};

export function normalizeMenuIcon(icon?: string): string {
  if (!icon) return '';
  // 已是 Iconify 全名（ep:xxx / lucide:xxx 等）直接用
  if (icon.includes(':')) return icon;
  // http 外链图标直接用（VbenIcon 支持 img）
  if (/^https?:\/\//i.test(icon)) return icon;
  return RUOYI_ICON_MAP[icon] ?? '';
}

/**
 * 为外链生成一个合法的路由 path（以 "/" 开头）。
 * 取 URL 的 host 作为路径段，避免与既有路由冲突；无法解析时回退到固定占位。
 */
function toExternalRoutePath(url: string): string {
  try {
    const { host, pathname } = new URL(url);
    // 去掉末尾斜杠，拼成 /host/path 形式
    const tail = (pathname || '').replace(/\/+$/, '');
    return `/${host}${tail}`;
  } catch {
    return `/external/${Date.now()}`;
  }
}

/**
 * 将若依菜单转换为 vben 的 RouteRecordStringComponent 结构。
 *
 * 核心转换：
 * 1. component 特殊值：
 *    - Layout → 不设 component（目录节点）。真正的壳由 Root 的 BasicLayout 统一提供，
 *      避免每个一级菜单各自挂 BasicLayout，跨模块切换整壳重建。
 *    - InnerLink → IFrameView，并把 meta.link 写入 meta.iframeSrc（内嵌 iframe）
 *    - ParentView → 不设 component（多级目录，靠 children 嵌套；菜单层级保留）
 * 2. meta 字段透传（title / icon / noCache / link）
 * 3. hidden → meta.hideInMenu
 * 4. noCache 反转 → meta.keepAlive
 * 5. query 解析进 meta.query
 * 6. 外链：path 上的 http(s) URL 挪到 meta.link，并生成合法 /path；新窗口打开
 * 7. redirect=noRedirect：删除（vue-router 不能把它当真实 redirect），并尽量指向首个可见子路由
 */
function transformRuoYiMenu(
  menus: RuoYiRouter[],
  parentPath = '',
): RouteRecordStringComponent[] {
  return menus.map((menu) => {
    const { component, meta } = menu;
    const isInnerLink = component === 'InnerLink';

    // component 特殊值映射：Layout/ParentView 仅作目录，不渲染独立布局组件
    let mappedComponent: string | undefined = component;
    if (component === 'Layout' || component === 'ParentView') {
      mappedComponent = undefined;
    } else if (isInnerLink) {
      mappedComponent = 'IFrameView';
    }

    // query 解析（若依用 JSON 字符串，如 {"id": 1}）
    let query: Record<string, any> | undefined;
    if (menu.query) {
      try {
        query = JSON.parse(menu.query);
      } catch {
        query = undefined;
      }
    }

    // 外链 path：完整 URL 挪到 meta.link，并生成合法路由 path
    const external = isExternalLink(menu.path);
    const rawPath = external ? toExternalRoutePath(menu.path) : menu.path;
    // 相对 path 拼到父级，便于 redirect 与菜单 path 使用绝对地址
    const finalPath =
      parentPath && !external && !rawPath.startsWith('/')
        ? joinRoutePath(parentPath, rawPath)
        : rawPath;

    // 外链新窗口：meta.link = 原始 URL
    // 内嵌 InnerLink：只用 iframeSrc，避免 generateMenus 用 link 覆盖 path 后被当成外链 window.open
    const finalLink = external ? menu.path : isInnerLink ? undefined : meta?.link;
    const iframeSrc = isInnerLink ? meta?.link : undefined;

    const transformed: RouteRecordStringComponent = {
      name: menu.name,
      path: finalPath,
      component: mappedComponent as any,
      meta: {
        title: meta?.title ?? menu.name,
        icon: normalizeMenuIcon(meta?.icon),
        hideInMenu: menu.hidden,
        // 若依 noCache=true 表示不缓存；vben keepAlive=true 表示缓存，需反转
        keepAlive: meta?.noCache === false,
        link: finalLink,
        iframeSrc,
        query,
        order: 0,
      },
    };

    if (menu.children && menu.children.length > 0) {
      transformed.children = transformRuoYiMenu(menu.children, finalPath);

      // 清洗若依目录占位 redirect，并补到第一个可见子路由（绝对 path）
      if (!menu.redirect || menu.redirect === 'noRedirect') {
        const firstVisibleChild = transformed.children.find(
          (child) => !child.meta?.hideInMenu,
        );
        if (firstVisibleChild?.path) {
          transformed.redirect = firstVisibleChild.path;
        }
      } else if (menu.redirect !== 'noRedirect') {
        transformed.redirect = menu.redirect;
      }
    } else if (menu.redirect && menu.redirect !== 'noRedirect') {
      transformed.redirect = menu.redirect;
    }

    return transformed;
  });
}

/**
 * vben 原生静态菜单（首页 / 工作台）
 *
 * 背景：preferences 中 accessMode 为 'backend'，后端模式下只会用后端 /getRouters
 * 返回的菜单生成路由，vben 自带的 dashboard 静态路由不会出现。这里把首页/工作台
 * 以若依菜单结构注入到后端菜单最前面，使其在侧边栏重新可见。
 *
 * 组件路径说明：generateRoutesByBackend 会用 import.meta.glob 扫描 views 下所有 .vue
 * 生成 pageMap 做匹配，key 经 normalizeViewPath 处理后形如
 * '/dashboard/analytics/index.vue'，故此处 component 写成 'dashboard/analytics/index'
 * （不带 /views 前缀、不带 .vue 后缀）。
 *
 * 一级不再需要 Layout 组件：Root 已提供唯一 BasicLayout。
 */
const builtinMenus: RuoYiRouter[] = [
  {
    name: 'Dashboard',
    path: '/dashboard',
    hidden: false,
    component: 'Layout',
    meta: {
      // 与 locales/zh-CN/page.json 的 page.dashboard.title 保持一致
      title: '概览',
      icon: 'lucide:layout-dashboard',
      noCache: false,
    },
    children: [
      {
        name: 'Analytics',
        path: '/analytics',
        hidden: false,
        component: 'dashboard/analytics/index',
        meta: {
          title: '分析页',
          icon: 'lucide:area-chart',
          noCache: false,
        },
      },
      {
        name: 'Workspace',
        path: '/workspace',
        hidden: false,
        component: 'dashboard/workspace/index',
        meta: {
          title: '工作台',
          icon: 'carbon:workspace',
          noCache: false,
        },
      },
    ],
  },
  // 个人中心：后端菜单无此页；必须走动态路由注入，不能放 coreRoutes，
  // 否则刷新时守卫按 coreRouteNames 直接放行，侧边栏菜单不会生成。
  {
    name: 'Profile',
    path: '/user/profile',
    hidden: true,
    component: 'system/user/profile/index',
    meta: {
      title: '个人中心',
      icon: 'user',
      noCache: false,
    },
  },
  // 分配角色页：后端菜单表无此路由，隐藏注入；一级仅作目录，实际布局由 Root 提供
  {
    name: 'SystemUserAuth',
    path: '/system/user-auth',
    hidden: true,
    component: 'Layout',
    meta: {
      title: '分配角色',
      noCache: true,
    },
    children: [
      {
        name: 'SystemUserAuthRole',
        path: 'role/:userId(.*)',
        hidden: true,
        component: 'system/user/authRole',
        meta: {
          title: '分配角色',
          noCache: true,
        },
      },
    ],
  },
  // 调度日志页：若依原版为前端静态隐藏路由，后端菜单表无此条目
  {
    name: 'MonitorJobLogRoot',
    path: '/monitor/job-log',
    hidden: true,
    component: 'Layout',
    meta: {
      title: '调度日志',
      noCache: true,
    },
    children: [
      {
        name: 'MonitorJobLog',
        path: 'index/:jobId(.*)',
        hidden: true,
        component: 'monitor/job/log',
        meta: {
          title: '调度日志',
          noCache: true,
        },
      },
    ],
  },
  // 代码生成编辑页：若依原版为前端静态隐藏路由 /tool/gen-edit/index/:tableId
  {
    name: 'ToolGenEditRoot',
    path: '/tool/gen-edit',
    hidden: true,
    component: 'Layout',
    meta: {
      title: '修改生成配置',
      noCache: true,
    },
    children: [
      {
        name: 'ToolGenEdit',
        path: 'index/:tableId(.*)',
        hidden: true,
        component: 'tool/gen/editTable',
        meta: {
          title: '修改生成配置',
          noCache: true,
        },
      },
    ],
  },
];

/**
 * 获取用户所有菜单（适配若依 GET /getRouters）
 *
 * 若依 /getRouters 返回 {code:200, data:[菜单树]}，requestClient 拦截器解包 data，
 * 再经 transformRuoYiMenu 转成 vben 期望的结构。
 *
 * 这里在转换后的若依菜单前，注入 vben 原生首页/工作台菜单，使二者并存于侧边栏。
 * 返回的顶层路由由 generateAccessible 挂到 Root.children，共享唯一 BasicLayout。
 */
export async function getAllMenusApi() {
  const raw = await requestClient.get<RuoYiRouter[]>('/getRouters');
  const builtin = transformRuoYiMenu(builtinMenus);
  // 将首页菜单 order 置为 -1，确保它排在所有若依业务菜单（默认 order 0）之前
  if (builtin[0]?.meta) {
    (builtin[0].meta as any).order = -1;
  }
  return [...builtin, ...transformRuoYiMenu(raw ?? [])];
}
