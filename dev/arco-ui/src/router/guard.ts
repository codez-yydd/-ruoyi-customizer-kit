import type { Router } from 'vue-router'
import { usePermissionStore } from '@/stores/permission'
import { useUserStore } from '@/stores/user'

/** 无需登录即可访问的白名单 */
const WHITE_LIST = ['/login', '/register']

/**
 * 全局路由守卫：
 * 1. 白名单直接放行
 * 2. 无 token -> 重定向 /login?redirect=xxx
 * 3. 有 token 访问 /login -> 跳转第一个可见菜单
 * 4. 有 token 且动态路由未生成 -> getInfo + generateRoutes + addRoute 后重放导航
 *    （next({ ...to, replace: true }) 保证刷新动态路由页面不落 404）
 * 5. 加载失败 -> 清 token 回登录页
 */
export function setupRouterGuard(router: Router): void {
  router.beforeEach(async (to) => {
    // 白名单放行
    if (WHITE_LIST.includes(to.path)) {
      // 注册开关关闭时访问 /register：重定向 /login（其余白名单页不受影响）
      if (to.path === '/register' && import.meta.env.VITE_APP_REGISTER !== 'true') {
        return { path: '/login' }
      }
      return true
    }

    const userStore = useUserStore()
    const permissionStore = usePermissionStore()

    // 无 token：全部去登录页，并记录目标地址
    if (!userStore.token) {
      return to.path === '/'
        ? { path: '/login' }
        : { path: '/login', query: { redirect: to.fullPath } }
    }

    // 已登录访问 /login：跳转第一个可见菜单（无菜单则回首页）
    if (to.path === '/login') {
      return permissionStore.firstMenuPath
    }

    // 动态路由未生成（首次进入/刷新页面）
    if (!permissionStore.isGenerated) {
      try {
        await userStore.getInfo()
        const records = await permissionStore.generateRoutes()
        // 后端一级菜单逐个挂载到 Root（/）之下
        for (const record of records) {
          router.addRoute('Root', record)
        }
        // 重放当前导航，使新注册的路由参与匹配
        return { path: to.path, query: to.query, hash: to.hash, replace: true }
      } catch {
        // getInfo/generateRoutes 失败（token 失效等）：清 token 回登录页
        userStore.resetToken()
        permissionStore.reset()
        return { path: '/login', query: { redirect: to.fullPath } }
      }
    }

    return true
  })
}
