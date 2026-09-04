import type { Directive, DirectiveBinding } from 'vue'
import { checkPermi } from '@/utils/permission'

/** 处理元素权限：无权限时从 DOM 移除 */
function handle(el: HTMLElement, binding: DirectiveBinding<string[]>): void {
  const value = binding.value
  if (!Array.isArray(value) || value.length === 0) {
    console.warn('[v-hasPermi] 需要传入权限数组，如 v-hasPermi="[\'system:user:add\']"')
    return
  }
  if (!checkPermi(value)) {
    el.parentNode?.removeChild(el)
  }
}

/**
 * v-hasPermi：数组权限校验（有任一权限则保留元素；`*:*:*` 全通过；无权限 removeChild）
 * 用法：<a-button v-hasPermi="['system:user:add']">新增</a-button>
 */
export const hasPermi: Directive<HTMLElement, string[]> = {
  mounted: handle,
  updated: handle
}
