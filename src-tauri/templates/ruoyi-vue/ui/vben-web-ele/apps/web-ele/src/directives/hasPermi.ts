import type { Directive, DirectiveBinding } from 'vue';

import { getCachedPermissions } from '#/api/core/user';

/**
 * v-hasPermi：若依按钮级权限指令（Vue3 版）
 *
 * 移植自若依 hasPermi.js，权限码来自 /getInfo 返回的 permissions。
 * 超级管理员拥有 "*:*:*" 全权限。
 *
 * 用法：<el-button v-hasPermi="['system:user:add']">新增</el-button>
 *       <el-button v-hasPermi="['system:user:edit']">修改</el-button>
 */
const ALL_PERMISSION = '*:*:*';

function checkPermi(value: string[]): boolean {
  if (!value || value.length === 0) {
    console.warn('v-hasPermi 需要设置权限标识数组');
    return true;
  }
  const permissions = getCachedPermissions();
  return permissions.some(
    (perm) => perm === ALL_PERMISSION || value.includes(perm),
  );
}

export const hasPermi: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    const { value } = binding;
    if (value instanceof Array && value.length > 0) {
      if (!checkPermi(value)) {
        el.parentNode?.removeChild(el);
      }
    } else {
      throw new Error('请设置操作权限标识，如 v-hasPermi="[\'system:user:add\']"');
    }
  },
};

/**
 * v-hasRole：若依角色级权限指令（辅助）
 *
 * 用法：<el-button v-hasRole="['admin']">仅管理员可见</el-button>
 * 超级管理员角色标识为 "admin"。
 */
function checkRole(value: string[]): boolean {
  if (!value || value.length === 0) return true;
  // roles 暂未单独缓存，此处从 getCachedPermissions 同源不可靠；
  // 若依的角色判断通常用 *:*:* 权限或 admin 角色标识，这里用超管权限近似
  const permissions = getCachedPermissions();
  return permissions.includes(ALL_PERMISSION) || value.includes('admin');
}

export const hasRole: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    const { value } = binding;
    if (value instanceof Array && value.length > 0) {
      if (!checkRole(value)) {
        el.parentNode?.removeChild(el);
      }
    }
  },
};
