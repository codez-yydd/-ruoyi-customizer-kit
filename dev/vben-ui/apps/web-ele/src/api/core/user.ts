import type { UserInfo } from '@vben/types';

import { requestClient } from '#/api/request';
import { setPwdChrType } from '#/utils/passwordRule';

/**
 * 若依 /getInfo 响应结构（原始）
 * { code, msg, user, roles, permissions, pwdChrtype, isDefaultModifyPwd, isPasswordExpired }
 * 其中 user 是 SysUser 实体，roles/permissions 是权限标识字符串数组。
 */
interface RuoYiGetInfoResponse {
  user: {
    userId: number;
    deptId?: number;
    userName: string;
    nickName: string;
    email?: string;
    phonenumber?: string;
    sex?: string;
    avatar?: string;
    status?: string;
    dept?: Record<string, any>;
    roles?: string[];
  };
  roles: string[];
  permissions: string[];
  pwdChrtype?: string;
  isDefaultModifyPwd?: boolean;
  isPasswordExpired?: boolean;
}

/**
 * 获取用户信息（适配若依 GET /getInfo）
 *
 * 若依返回 {user, roles, permissions}，这里映射成 vben 的 UserInfo：
 * - userId / username / realName(←nickName) / avatar / roles
 * - permissions 暂存到模块级变量，供 hasPermi 权限指令取用（若依按钮权限标识）
 *
 * 注意：requestClient 的响应拦截器会自动解包 data，故此处拿到的是 data 内容本身。
 */
let cachedPermissions: string[] = [];
export function getCachedPermissions(): string[] {
  return cachedPermissions;
}

export async function getUserInfoApi() {
  const raw = await requestClient.get<RuoYiGetInfoResponse>('/getInfo');

  // 缓存若依权限码，供 v-hasPermi 指令使用
  cachedPermissions = raw.permissions ?? [];

  // 同步密码字符范围规则，供个人中心改密等前端校验与后端配置一致
  setPwdChrType(raw.pwdChrtype);

  // 头像 URL 处理：若依 avatar 存的是相对路径（如 /profile/avatar/.../xx.png），
  // 需拼上 API 前缀（开发态 /api，由 vite proxy 转发；生产态同理）。
  // 若是完整 http(s) URL 则原样使用；为空则交由组件回退到默认头像。
  const rawAvatar = raw.user.avatar ?? '';
  const avatar = /^https?:\/\//i.test(rawAvatar)
    ? rawAvatar
    : rawAvatar
      ? `${import.meta.env.VITE_GLOB_API_URL}${rawAvatar}`
      : '';

  const userInfo: UserInfo = {
    userId: String(raw.user.userId),
    username: raw.user.userName,
    realName: raw.user.nickName,
    avatar,
    roles: raw.roles ?? [],
    desc: raw.user.dept?.deptName ?? '',
    // 登录后默认进入首页（分析页 /analytics）。
    // 工作台在 /workspace，可在侧边栏切换。
    homePath: '/analytics',
    token: '',
  };

  // 缓存完整用户信息（邮箱/手机/部门等，供个人中心等页面取用，避免重复请求）
  cachedRuoYiUser = raw.user;

  return userInfo;
}

/**
 * 缓存若依 /getInfo 返回的原始 user 对象，
 * 供个人中心等需要更多字段的页面取用（避免再请求一次 /getInfo）。
 */
let cachedRuoYiUser: RuoYiGetInfoResponse['user'] | null = null;
export function getCachedRuoYiUser() {
  return cachedRuoYiUser;
}
