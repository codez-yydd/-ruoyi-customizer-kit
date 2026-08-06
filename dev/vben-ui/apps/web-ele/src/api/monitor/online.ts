import { requestClient } from '#/api/request';

/**
 * 在线用户（移植自 ruoyi-ui/src/api/monitor/online.js）
 */
export interface SysUserOnline {
  tokenId: string;
  userName: string;
  deptName?: string;
  ipaddr: string;
  loginLocation?: string;
  browser?: string;
  os?: string;
  /** 登录时间（后端为 Long 毫秒时间戳） */
  loginTime: number;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

/** 查询在线用户列表（后端从 Redis 返回全量，需前端分页） */
export function listOnline(query: {
  ipaddr?: string;
  userName?: string;
}) {
  return requestClient.get<TableResult<SysUserOnline>>('/monitor/online/list', {
    params: query,
  });
}

/** 强退用户（按会话 tokenId 删除 Redis 登录缓存） */
export function forceLogout(tokenId: string) {
  return requestClient.delete(
    `/monitor/online/${encodeURIComponent(tokenId)}`,
  );
}
