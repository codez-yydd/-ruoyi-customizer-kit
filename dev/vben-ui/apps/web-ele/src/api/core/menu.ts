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
 * 将若依菜单转换为 vben 的 RouteRecordStringComponent 结构。
 *
 * 核心转换：
 * 1. component 特殊值映射：Layout → BasicLayout（一级目录布局），InnerLink → IFrameView（内嵌外链），
 *    ParentView → 移除 component（vben 靠 children 自动处理多级菜单父视图）
 * 2. meta 字段透传（title / icon / noCache / link）
 * 3. hidden → meta.hideInMenu（vben 用此字段控制侧边栏显隐）
 * 4. noCache 反转 → meta.keepAlive（若依 noCache=true 表示不缓存，vben keepAlive=true 表示缓存）
 * 5. query 解析进 meta.query（JSON 字符串 → 对象）
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

    const transformed: RouteRecordStringComponent = {
      name: menu.name,
      path: menu.path,
      component: mappedComponent as any,
      redirect: menu.redirect,
      meta: {
        title: meta?.title ?? menu.name,
        icon: meta?.icon,
        // vben 隐藏菜单用 hideInMenu
        hideInMenu: menu.hidden,
        // 若依 noCache=true 表示不缓存；vben keepAlive=true 表示缓存，需反转
        keepAlive: meta?.noCache === false,
        link: meta?.link,
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
 * 获取用户所有菜单（适配若依 GET /getRouters）
 *
 * 若依 /getRouters 返回 {code:200, data:[菜单树]}，requestClient 拦截器解包 data，
 * 再经 transformRuoYiMenu 转成 vben 期望的结构。
 */
export async function getAllMenusApi() {
  const raw = await requestClient.get<RuoYiRouter[]>('/getRouters');
  return transformRuoYiMenu(raw ?? []);
}
