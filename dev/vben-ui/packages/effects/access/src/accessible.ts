import type { Component, DefineComponent } from 'vue';

import type {
  AccessModeType,
  GenerateMenuAndRoutesOptions,
  RouteRecordRaw,
} from '@vben/types';

import { defineComponent, h } from 'vue';

import {
  cloneDeep,
  generateMenus,
  generateRoutesByBackend,
  generateRoutesByFrontend,
  isFunction,
  isString,
  mapTree,
} from '@vben/utils';

/**
 * 生成可访问的菜单与路由。
 *
 * 关键改动（对齐上游 Vben）：动态路由默认挂到 path='/' 的 Root 下，
 * 由 Root 统一使用 BasicLayout。若依每个一级目录都带 Layout，若各自
 * addRoute 成独立 BasicLayout，跨模块切换会卸载重建整壳（侧栏/顶栏/Tab），
 * 表现为“偶发整页刷新”。
 */
async function generateAccessible(
  mode: AccessModeType,
  options: GenerateMenuAndRoutesOptions,
) {
  const { router } = options;

  options.routes = cloneDeep(options.routes);
  // 生成路由
  const accessibleRoutes = await generateRoutes(mode, options);

  const root = router.getRoutes().find((item) => item.path === '/');

  // 获取已有的路由名称列表（core 里已挂到 Root 的子路由，如个人中心）
  const names = root?.children?.map((item) => item.name) ?? [];

  // 动态添加到 router：默认作为 Root 子路由，避免多个并列 BasicLayout
  accessibleRoutes.forEach((route) => {
    if (root && !route.meta?.noBasicLayout) {
      // 含有子路由时去掉自身 component，防止嵌套多层 BasicLayout
      if (route.children && route.children.length > 0) {
        delete route.component;
      }
      // 同名路由已存在则更新，避免切换用户后一级目录残留导致 404
      if (names?.includes(route.name)) {
        const index = root.children?.findIndex(
          (item) => item.name === route.name,
        );
        if (index !== undefined && index !== -1 && root.children) {
          root.children[index] = route;
        }
      } else {
        root.children?.push(route);
      }
    } else {
      router.addRoute(route);
    }
  });

  // 重新注册 Root，使新增的 children 生效
  if (root) {
    if (root.name) {
      router.removeRoute(root.name);
    }
    router.addRoute(root);
  }

  // 生成菜单（仍用 accessibleRoutes 顶层树，侧边栏结构不变）
  const accessibleMenus = await generateMenus(accessibleRoutes, options.router);

  return { accessibleMenus, accessibleRoutes };
}

/**
 * Generate routes
 * @param mode
 * @param options
 */
async function generateRoutes(
  mode: AccessModeType,
  options: GenerateMenuAndRoutesOptions,
) {
  const { forbiddenComponent, roles, routes } = options;

  let resultRoutes: RouteRecordRaw[] = routes;
  switch (mode) {
    case 'backend': {
      resultRoutes = await generateRoutesByBackend(options);
      break;
    }
    case 'frontend': {
      resultRoutes = await generateRoutesByFrontend(
        routes,
        roles || [],
        forbiddenComponent,
      );
      break;
    }
  }

  /**
   * 调整路由树：
   * 1. keepAlive 时把懒加载组件名对齐到路由 name，保证缓存命中
   * 2. 对未配置 redirect 的目录补全到第一个可访问子路由
   */
  resultRoutes = mapTree(resultRoutes, (route) => {
    // keepAlive：用路由 name 包装异步组件，否则 KeepAlive 无法按 name 缓存
    if (
      route.meta?.keepAlive &&
      isFunction(route.component) &&
      route.name &&
      isString(route.name)
    ) {
      const originalComponent = route.component as () => Promise<{
        default: Component | DefineComponent;
      }>;
      route.component = async () => {
        const component = await originalComponent();
        if (!component.default) return component;
        return defineComponent({
          name: route.name as string,
          setup(props, { attrs, slots }) {
            return () => h(component.default, { ...props, ...attrs }, slots);
          },
        });
      };
    }

    // 已有 redirect、无子路由，或 redirect 为若依占位 noRedirect 时跳过自动补全
    // （noRedirect 会在 menu 转换层被清除；此处再兜底忽略）
    if (
      (route.redirect && route.redirect !== 'noRedirect') ||
      !route.children ||
      route.children.length === 0
    ) {
      return route;
    }

    const firstChild = route.children[0];

    // 子路由不是以 / 开头时需拼接父 path，这里不做复杂计算（转换层会尽量写成绝对路径）
    if (!firstChild?.path || !firstChild.path.startsWith('/')) {
      return route;
    }

    route.redirect = firstChild.path;
    return route;
  });

  return resultRoutes;
}

export { generateAccessible };
