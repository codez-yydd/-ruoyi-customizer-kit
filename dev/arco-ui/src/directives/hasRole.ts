import type { Directive, DirectiveBinding } from 'vue'
import { checkRole } from '@/utils/permission'

/** 处理元素角色：无角色时从 DOM 移除 */
function handle(el: HTMLElement, binding: DirectiveBinding<string[]>): void {
  const value = binding.value
  if (!Array.isArray(value) || value.length === 0) {
    console.warn('[v-hasRole] 需要传入角色数组，如 v-hasRole="[\'admin\']"')
    return
  }
  if (!checkRole(value)) {
    el.parentNode?.removeChild(el)
  }
}

/**
 * v-hasRole：数组角色校验（有任一角色则保留元素；admin 全通过；无角色 removeChild）
 * 用法：<a-button v-hasRole="['admin']">管理</a-button>
 */
export const hasRole: Directive<HTMLElement, string[]> = {
  mounted: handle,
  updated: handle
}
