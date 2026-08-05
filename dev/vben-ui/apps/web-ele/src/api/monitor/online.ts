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
  loginTime: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

// 查询在线用户列表
export function listOnline(query: Record<string, any>) {
  return requestClient.get<TableResult<SysUserOnline>>('/monitor/online/list', {
    params: query,
  });
}

// 强退用户
export function forceLogout(tokenId: string) {
  return requestClient.delete(`/monitor/online/${tokenId}`);
}
