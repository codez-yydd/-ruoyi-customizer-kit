// 路由配置：5 个主流程页面 + 导航守卫（步骤门控）

import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { mark } from '@/utils/diagnostic'

const routes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  {
    path: '/home',
    name: 'home',
    component: () => import('@/views/Home.vue'),
    meta: { title: '首页', step: 0 }
  },
  {
    path: '/detect',
    name: 'detect',
    component: () => import('@/views/ProjectDetect.vue'),
    meta: { title: '项目识别', step: 1 }
  },
  {
    path: '/config',
    name: 'config',
    component: () => import('@/views/ParamConfig.vue'),
    meta: { title: '参数配置', step: 2 }
  },
  {
    path: '/preview',
    name: 'preview',
    component: () => import('@/views/Preview.vue'),
    meta: { title: '执行预览', step: 3 }
  },
  {
    path: '/execute',
    name: 'execute',
    component: () => import('@/views/Execute.vue'),
    meta: { title: '执行改造', step: 4 }
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

// 全局守卫：受步骤门控的页面，未解锁则回首页。
// home(step0) 不受限；detect 及之后需 maxStep 达标。
router.beforeEach((to) => {
  const step = (to.meta.step as number) ?? 0
  if (step < 0) return true // home 始终放行
  const store = useProjectStore()
  mark('guard.check', {
    to: String(to.name),
    step,
    maxStep: store.maxStep,
    recognized: store.projectInfo?.confidence?.recognized
  })
  // 未选项目时（maxStep=0）禁止进入任何后续步骤
  if (step > store.maxStep) {
    mark('guard.blocked', { to: String(to.name), step, maxStep: store.maxStep })
    return { name: 'home' }
  }
  return true
})

export default router
