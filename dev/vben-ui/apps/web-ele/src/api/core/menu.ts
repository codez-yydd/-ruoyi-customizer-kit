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
 * - redirect：重定向
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
 * 1. component 特殊值映射：Layout → BasicLayout（一级目录布局），InnerLink → IFrameView（内嵌外链），
 *    ParentView → 移除 component（vben 靠 children 自动处理多级菜单父视图）
 * 2. meta 字段透传（title / icon / noCache / link）
 * 3. hidden → meta.hideInMenu（vben 用此字段控制侧边栏显隐）
 * 4. noCache 反转 → meta.keepAlive（若依 noCache=true 表示不缓存，vben keepAlive=true 表示缓存）
 * 5. query 解析进 meta.query（JSON 字符串 → 对象）
 * 6. 外链处理：若依把完整 URL 放 path，vue-router 不接受；需把 URL 移到 meta.link，
 *    并生成合法 /path，菜单点击时 vben 的 use-navigation 检测到 http URL 会在新标签页打开。
 */
function transformRuoYiMenu(menus: RuoYiRouter[]): RouteRecordStringComponent[] {
  return menus.map((menu) => {
    const { component, meta } = menu;

    // component 特殊值映射
    let mappedComponent: string | undefined = component;
    if (component === 'Layout') {
      mappedComponent = 'BasicLayout';
    } else if (component === 'InnerLink') {
      mappedComponent = 'IFrameView';
    } else if (component === 'ParentView') {
      // ParentView 在 vben 中不需要独立组件，靠 children 嵌套自动渲染
      mappedComponent = undefined;
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

    // 外链 path 处理：若依外链菜单 path 是完整 URL，需移到 meta.link 并生成合法 /path
    // （否则 router.addRoute 会抛 "Route paths should start with a /"）
    const external = isExternalLink(menu.path);
    const finalPath = external ? toExternalRoutePath(menu.path) : menu.path;
    const finalLink = external ? menu.path : meta?.link;

    const transformed: RouteRecordStringComponent = {
      name: menu.name,
      path: finalPath,
      component: mappedComponent as any,
      redirect: menu.redirect,
      meta: {
        title: meta?.title ?? menu.name,
        icon: normalizeMenuIcon(meta?.icon),
        // vben 隐藏菜单用 hideInMenu
        hideInMenu: menu.hidden,
        // 若依 noCache=true 表示不缓存；vben keepAlive=true 表示缓存，需反转
        keepAlive: meta?.noCache === false,
        link: finalLink,
        query,
        // 保留若依原始标识，便于调试
        order: 0,
      },
    };

    if (menu.alwaysShow) {
      (transformed.meta as any).noBasicLayout = false;
    }

    if (menu.children && menu.children.length > 0) {
      transformed.children = transformRuoYiMenu(menu.children);
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
 */
const builtinMenus: RuoYiRouter[] = [
  {
    name: 'Dashboard',
    path: '/dashboard',
    hidden: false,
    component: 'Layout',
    meta: {
      title: 'Dashboard',
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
  // 分配角色页：若依原版为独立页面，但后端菜单表无此路由（通过用户列表按钮进入）。
  // 这里以隐藏菜单注入：一级用 Layout（BasicLayout）承载布局，子路由带 :userId 参数，
  // component 指向 views/system/user/authRole.vue。hidden=true 不出现在侧边栏。
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
];

/**
 * 获取用户所有菜单（适配若依 GET /getRouters）
 *
 * 若依 /getRouters 返回 {code:200, data:[菜单树]}，requestClient 拦截器解包 data，
 * 再经 transformRuoYiMenu 转成 vben 期望的结构。
 *
 * 这里在转换后的若依菜单前，注入 vben 原生首页/工作台菜单，使二者并存于侧边栏。
 */
export async function getAllMenusApi() {
  const raw = await requestClient.get<RuoYiRouter[]>('/getRouters');
  // builtinMenus 同样走 transformRuoYiMenu，使其 Layout→BasicLayout 等映射与后端菜单一致
  const builtin = transformRuoYiMenu(builtinMenus);
  // 将首页菜单 order 置为 -1，确保它排在所有若依业务菜单（默认 order 0）之前
  if (builtin[0]?.meta) {
    (builtin[0].meta as any).order = -1;
  }
  return [...builtin, ...transformRuoYiMenu(raw ?? [])];
}
