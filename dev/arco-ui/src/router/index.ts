import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

/**
 * 静态路由：
 * - `/`（Root）为 AppLayout 布局壳，后端一级菜单将作为其 children 动态挂载
 * - catch-all 404 放在 constantRoutes，vue-router4 按路径优先级自动兜底，
 *   刷新动态路由页面时由守卫在 next 前完成 addRoute 后重放导航
 */
export const constantRoutes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { hidden: true }
  },
  {
    path: '/register',
    name: 'Register',
    component: () => import('@/views/register/index.vue'),
    meta: { hidden: true }
  },
  {
    path: '/',
    name: 'Root',
    component: () => import('@/layouts/index.vue'),
    children: [
      {
        path: '',
        name: 'Dashboard',
        component: () => import('@/views/dashboard/index.vue'),
        meta: { title: '首页', icon: 'dashboard', affix: true, keepAlive: true }
      },
      {
        // 刷新当前路由的中转页（TabsView 刷新功能依赖）
        path: '/redirect/:path(.*)',
        name: 'Redirect',
        component: () => import('@/views/redirect/index.vue'),
        meta: { hidden: true }
      }
      // 个人中心 /user/profile 等隐藏业务页由 stores/permission.ts 随动态路由注入
    ]
  },
  {
    path: '/403',
    name: 'Forbidden',
    component: () => import('@/views/error/403.vue'),
    meta: { hidden: true, title: '无权限' }
  },
  {
    path: '/500',
    name: 'ServerError',
    component: () => import('@/views/error/500.vue'),
    meta: { hidden: true, title: '服务器错误' }
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'NotFound',
    component: () => import('@/views/error/404.vue'),
    meta: { hidden: true, title: '页面不存在' }
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes: constantRoutes,
  strict: false,
  scrollBehavior: () => ({ left: 0, top: 0 })
})

export default router
