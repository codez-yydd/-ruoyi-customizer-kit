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
 *
 * 挂载方式：使用 router.addRoute('Root', route) 追加子路由，
 * 避免对 getRoutes() 返回的 normalize 记录做 remove/add 整棵 Root，
 * 降低刷新后布局/菜单异常的风险。
 */
async function generateAccessible(
  mode: AccessModeType,
  options: GenerateMenuAndRoutesOptions,
) {
  const { router } = options;

  options.routes = cloneDeep(options.routes);
  // 生成路由
  const accessibleRoutes = await generateRoutes(mode, options);

  const root = router.getRoutes().find((item) => item.name === 'Root');
  const rootName = root?.name;

  // 动态添加到 router：默认作为 Root 子路由，避免多个并列 BasicLayout
  accessibleRoutes.forEach((route) => {
    if (rootName && !route.meta?.noBasicLayout) {
      // 含有子路由时去掉自身 component，防止嵌套多层 BasicLayout
      if (route.children && route.children.length > 0) {
        delete route.component;
      }
      // 同名路由先移除再挂到 Root，保证切换用户后一级目录能更新
      if (route.name && router.hasRoute(route.name)) {
        router.removeRoute(route.name);
      }
      router.addRoute(rootName, route);
    } else {
      if (route.name && router.hasRoute(route.name)) {
        router.removeRoute(route.name);
      }
      router.addRoute(route);
    }
  });

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
