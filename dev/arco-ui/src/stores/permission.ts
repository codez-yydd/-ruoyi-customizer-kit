import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { Component } from 'vue'
import type { RouteMeta, RouteRecordRaw } from 'vue-router'
import { getRouters } from '@/api/login'
import type { MenuNode, RouterVo } from '@/api/types'
import ParentView from '@/components/ParentView.vue'
import InnerLink from '@/layouts/components/InnerLink.vue'
import { isHttp } from '@/utils/validate'

/** 页面组件懒加载器 */
type LazyComponent = () => Promise<Component>

/**
 * 页面组件映射表：key 形如 ../views/system/user/index.vue，
 * 与后端 component 字符串（system/user/index）按约定拼接匹配
 */
const pageModules = import.meta.glob('../views/**/*.vue')

/**
 * Cloud 官方日志菜单 component 为 system/operlog|logininfor，
 * Arco 页面在 monitor/ 下；仅在原路径找不到时才走别名。
 */
const VIEW_ALIASES: Record<string, string> = {
  'system/operlog/index': 'monitor/operlog/index',
  'system/logininfor/index': 'monitor/logininfor/index'
}

/** component 字符串 -> 懒加载组件；匹配不到时按别名再找，仍没有则回退 404 并告警 */
function loadView(component: string): LazyComponent {
  const key = `../views/${component}.vue`
  let loader = pageModules[key]
  if (!loader) {
    const aliased = VIEW_ALIASES[component]
    if (aliased) {
      loader = pageModules[`../views/${aliased}.vue`]
    }
  }
  if (!loader) {
    console.warn(`[permission] 未找到页面组件: ${key}，已回退到 404 页面`)
    return () => import('../views/error/404.vue')
  }
  return loader as LazyComponent
}

/** 外链 URL -> 伪路由 path：去协议、去 query/hash，如 https://www.baidu.com -> /www.baidu.com */
function toFakePath(url: string): string {
  const raw = url.replace(/^https?:\/\//, '')
  return '/' + raw.split(/[?#]/)[0]
}

/** 解析后端 query JSON 字符串（容错：解析失败返回 null） */
function parseQuery(query: string | undefined): Record<string, string> | null {
  if (!query) return null
  try {
    const parsed: unknown = JSON.parse(query)
    if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Record<string, string>
    }
    return null
  } catch {
    return null
  }
}

/** RouterVo -> 路由 meta（noCache 取反为 keepAlive） */
function buildMeta(menu: RouterVo, extraLink?: string): RouteMeta {
  const voMeta = menu.meta ?? {}
  return {
    title: voMeta.title,
    icon: voMeta.icon,
    noCache: !!voMeta.noCache,
    keepAlive: !voMeta.noCache,
    hidden: !!menu.hidden,
    alwaysShow: menu.alwaysShow ? true : undefined,
    link: extraLink ?? voMeta.link ?? null,
    query: parseQuery(menu.query)
  }
}

/** 计算子菜单完整 path（绝对 path 直接用，相对 path 拼接父级） */
function joinPath(parentPath: string, childPath: string): string {
  if (childPath.startsWith('/')) return childPath
  if (!parentPath) return '/' + childPath
  return parentPath.replace(/\/$/, '') + '/' + childPath
}

interface TransformResult {
  record: RouteRecordRaw
  /** 侧边栏节点：hidden 菜单路由注册但不进菜单树，此时为 null */
  node: MenuNode | null
}

/**
 * 内置隐藏路由（不在菜单树中显示，但需要登录后可直达）：
 * 必须随动态路由一起注入（constantRoutes 中不注册），否则刷新时
 * 守卫在 generateRoutes 完成前重放导航，路由不存在会落 404。
 */
interface BuiltinRoute {
  path: string
  name: string
  /** 页面组件（相对 views 目录，与后端 component 字符串约定一致） */
  component: string
  title: string
  /** 打开该页时侧边栏高亮的菜单 path */
  activeMenu?: string
}

const BUILTIN_ROUTES: BuiltinRoute[] = [
  {
    path: '/system/user-auth/role/:userId(.*)',
    name: 'AuthRole',
    component: 'system/user/authRole',
    title: '分配角色',
    activeMenu: '/system/user'
  },
  {
    path: '/system/role-auth/user/:roleId(.*)',
    name: 'AuthUser',
    component: 'system/role/authUser',
    title: '分配用户',
    activeMenu: '/system/role'
  },
  {
    path: '/system/dict-data/:dictId(.*)',
    name: 'DictData',
    component: 'system/dict/data',
    title: '字典数据',
    activeMenu: '/system/dict'
  },
  {
    path: '/user/profile',
    name: 'Profile',
    component: 'system/user/profile/index',
    title: '个人中心'
  },
  {
    // 调度日志：后端菜单无独立菜单项（原版为 job 页跳转），故注册为内置隐藏路由
    path: '/monitor/job-log',
    name: 'JobLog',
    component: 'monitor/job/log',
    title: '调度日志',
    activeMenu: '/monitor/job'
  }
]

/** 内置隐藏路由 -> 可注册路由记录（挂到 Root 之下，不进菜单树） */
function buildBuiltinRecords(): RouteRecordRaw[] {
  return BUILTIN_ROUTES.map((item) => ({
    path: item.path,
    name: item.name,
    component: loadView(item.component),
    meta: {
      title: item.title,
      activeMenu: item.activeMenu,
      hidden: true
    }
  }))
}

/**
 * 递归转换 RouterVo 为路由记录 + 菜单节点。
 * 约定：一级菜单（component=Layout）挂在 constantRoutes 的 `/`（Root，AppLayout 壳）之下，
 * 因此 Layout/ParentView 在本工程中统一渲染为嵌套 <router-view/>（ParentView 组件）。
 */
function transformMenu(menu: RouterVo, parentPath: string): TransformResult {
  // 外链菜单：path 为 http(s) 开头，改写为伪路径并将完整 URL 存入 meta.link
  const isExternal = isHttp(menu.path)
  const fullPath = isExternal ? toFakePath(menu.path) : joinPath(parentPath, menu.path)
  const meta = buildMeta(menu, isExternal ? menu.path : undefined)

  let component: RouteRecordRaw['component']
  if (isExternal) {
    // 外链仅注册占位：菜单点击由侧边栏 window.open 新窗口打开（见 Sidebar.vue）；
    // 伪路径本身未注册为路由，直接访问会落 404，InnerLink 仅用于内嵌 InnerLink 类型菜单
    component = InnerLink
  } else {
    switch (menu.component) {
      case 'Layout':
      case 'ParentView':
        component = ParentView
        break
      case 'InnerLink':
        // iframe 内嵌页，URL 在 meta.link
        component = InnerLink
        break
      default:
        component = menu.component
          ? loadView(menu.component)
          : ParentView // 无 component 的目录兜底为空视图
    }
  }

  const children = menu.children ?? []
  const childResults = children.map((child) => transformMenu(child, fullPath))

  // 目录 redirect：'noRedirect' 占位删除；否则指向第一个可见叶子，避免直接访问目录时空白
  let redirect: string | undefined
  if (childResults.length > 0 && menu.redirect !== 'noRedirect') {
    const visibleNodes = childResults.map((r) => r.node).filter((n): n is MenuNode => n !== null)
    const leaf = firstVisibleLeaf(visibleNodes)
    if (leaf) redirect = leaf.path
  }

  const record: RouteRecordRaw = {
    path: menu.path,
    name: menu.name,
    component,
    redirect,
    meta,
    children: childResults.map((r) => r.record)
  }

  const visibleChildrenNodes = childResults
    .map((r) => r.node)
    .filter((n): n is MenuNode => n !== null)

  // 目录叶子（无任何可见子级）不进菜单树
  let node: MenuNode | null = null
  if (!menu.hidden && (visibleChildrenNodes.length > 0 || !children.length)) {
    node = {
      path: fullPath,
      title: meta.title ?? '',
      icon: meta.icon,
      link: meta.link ?? undefined,
      query: meta.query ?? undefined,
      alwaysShow: meta.alwaysShow,
      children: visibleChildrenNodes.length > 0 ? visibleChildrenNodes : undefined
    }
  }

  return { record, node }
}

/** 取菜单树中第一个可见、非外链的叶子节点 */
function firstVisibleLeaf(nodes: MenuNode[]): MenuNode | null {
  for (const node of nodes) {
    if (node.link) continue
    if (node.children && node.children.length > 0) {
      const leaf = firstVisibleLeaf(node.children)
      if (leaf) return leaf
    } else {
      return node
    }
  }
  return null
}

/** 权限路由状态：完整路由（addRoute 用）/ 侧边栏菜单树 / 生成标记 */
export const usePermissionStore = defineStore('permission', () => {
  /** 是否已从后端拉取并转换过动态路由（刷新后由守卫重新生成） */
  const isGenerated = ref(false)
  /** 后端返回的完整动态路由（每项 addRoute 到 Root 之下） */
  const routes = ref<RouteRecordRaw[]>([])
  /** 侧边栏菜单树（已过滤 hidden） */
  const sidebarRoutes = ref<MenuNode[]>([])

  /** keep-alive include 列表（keepAlive=true 的路由 name，与 route.name 对应） */
  const cachedViews = computed<string[]>(() => {
    const names: string[] = []
    const walk = (records: RouteRecordRaw[]): void => {
      for (const record of records) {
        if (record.name && record.meta?.keepAlive) {
          names.push(String(record.name))
        }
        if (record.children?.length) walk(record.children)
      }
    }
    walk(routes.value)
    return names
  })

  /** 登录后第一个可见落地路径（无菜单时回退 '/'） */
  const firstMenuPath = computed<string>(() => {
    const leaf = firstVisibleLeaf(sidebarRoutes.value)
    return leaf?.path ?? '/'
  })

  /** 拉取后端路由并转换为可注册路由 + 菜单树（含内置隐藏路由） */
  async function generateRoutes(): Promise<RouteRecordRaw[]> {
    const menus = await getRouters()
    const records: RouteRecordRaw[] = []
    const sidebar: MenuNode[] = []
    for (const menu of menus ?? []) {
      const result = transformMenu(menu, '')
      records.push(result.record)
      if (result.node) sidebar.push(result.node)
    }
    records.push(...buildBuiltinRecords())
    routes.value = records
    sidebarRoutes.value = sidebar
    isGenerated.value = true
    return records
  }

  /** 退出登录/切换账号时重置 */
  function reset(): void {
    isGenerated.value = false
    routes.value = []
    sidebarRoutes.value = []
  }

  return {
    isGenerated,
    routes,
    sidebarRoutes,
    cachedViews,
    firstMenuPath,
    generateRoutes,
    reset
  }
})
