import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { MenuNode } from '@/api/types'
import { usePermissionStore } from '@/stores/permission'
import { isHttp } from '@/utils/validate'

/**
 * 菜单导航共享逻辑（侧边栏与顶部水平菜单复用）：
 * - 选中态：meta.activeMenu 优先（详情页高亮菜单场景）
 * - 点击：外链新窗口打开；其余路由跳转（携带后端下发的 query）
 */
export function useMenuNav() {
  const route = useRoute()
  const router = useRouter()
  const permissionStore = usePermissionStore()

  const selectedKeys = computed<string[]>(() => {
    const active = route.meta.activeMenu
    return [typeof active === 'string' && active ? active : route.path]
  })

  function onMenuClick(key: string | number | undefined): void {
    const target = String(key ?? '')
    if (!target) return
    if (isHttp(target)) {
      window.open(target, '_blank', 'noopener,noreferrer')
      return
    }
    const node = findNode(permissionStore.sidebarRoutes, target)
    if (node?.query && Object.keys(node.query).length > 0) {
      router.push({ path: target, query: node.query })
    } else {
      router.push(target)
    }
  }

  /** 在菜单树中查找目标 path 节点 */
  function findNode(nodes: MenuNode[], path: string): MenuNode | undefined {
    for (const node of nodes) {
      if (node.path === path) return node
      const found = node.children ? findNode(node.children, path) : undefined
      if (found) return found
    }
    return undefined
  }

  return { selectedKeys, onMenuClick }
}
