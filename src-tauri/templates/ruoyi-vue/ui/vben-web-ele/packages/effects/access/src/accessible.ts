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
 * 关键改动（对齐上游 Vben + 若依）：
 * 1. 动态路由挂到 path='/' 的 Root 下，由 Root 统一 BasicLayout，避免多壳重建。
 * 2. 菜单树保持嵌套（侧栏要显示「日志管理」等目录）；注册到 vue-router 时把
 *    无 component 的中间目录（原 ParentView）子节点提升，与若依 filterChildren 一致。
 *    否则嵌套空组件路由匹配失败，会落到 Root 外的 404，整壳卸载，像整页刷新。
 */
async function generateAccessible(
  mode: AccessModeType,
  options: GenerateMenuAndRoutesOptions,
) {
  const { router } = options;

  options.routes = cloneDeep(options.routes);
  // 生成路由（嵌套树）
  const accessibleRoutes = await generateRoutes(mode, options);
  // 侧栏用嵌套结构；addRoute 用扁平化树。
  // 不能用 lodash cloneDeep：会破坏懒加载 component 函数。
  const menuRouteTree = cloneRouteTree(accessibleRoutes);
  const routerRouteTree = flattenEmptyDirRoutes(
    cloneRouteTree(accessibleRoutes),
  );

  const root = router.getRoutes().find((item) => item.name === 'Root');
  const rootName = root?.name;

  // 动态添加到 router：默认作为 Root 子路由，避免多个并列 BasicLayout
  routerRouteTree.forEach((route) => {
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

  // 菜单仍按嵌套树生成，目录层级与若依侧栏一致
  const accessibleMenus = await generateMenus(menuRouteTree, options.router);

  return { accessibleMenus, accessibleRoutes: routerRouteTree };
}

/** 拷贝路由树，component 等函数保持同一引用（供菜单/注册分流） */
function cloneRouteTree(routes: RouteRecordRaw[]): RouteRecordRaw[] {
  return routes.map((route) => ({
    ...route,
    meta: route.meta ? { ...route.meta } : route.meta,
    children: route.children ? cloneRouteTree(route.children) : undefined,
  }));
}

/**
 * 将「无 component 且仅有 children」的中间目录扁平化（对齐若依 ParentView → filterChildren）。
 * 只提升嵌套目录的子节点，保留一级目录（如 /system），避免侧栏父 path 在路由表中消失。
 */
function flattenEmptyDirRoutes(routes: RouteRecordRaw[]): RouteRecordRaw[] {
  return routes.map((route) => {
    if (!route.children?.length) {
      return route;
    }
    return {
      ...route,
      children: hoistEmptyDirChildren(route.children),
    };
  });
}

/** 递归提升无 component 的中间目录子路由 */
function hoistEmptyDirChildren(children: RouteRecordRaw[]): RouteRecordRaw[] {
  const result: RouteRecordRaw[] = [];
  for (const child of children) {
    const next: RouteRecordRaw = { ...child };
    if (next.children && next.children.length > 0) {
      next.children = hoistEmptyDirChildren(next.children);
      if (!next.component) {
        // ParentView / 已剥离 Layout 的目录：子页提升到本层，目录本身不注册
        result.push(...next.children);
        continue;
      }
    }
    result.push(next);
  }
  return result;
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
