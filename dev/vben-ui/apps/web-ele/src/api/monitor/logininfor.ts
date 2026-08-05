import { requestClient } from '#/api/request';

/**
 * 登录日志（移植自 ruoyi-ui/src/api/monitor/logininfor.js）
 */
export interface SysLogininfor {
  infoId: number;
  userName: string;
  ipaddr: string;
  loginLocation?: string;
  browser?: string;
  os?: string;
  status: string;
  msg?: string;
  loginTime: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

// 查询登录日志列表
export function listLogininfor(query: Record<string, any>) {
  return requestClient.get<TableResult<SysLogininfor>>(
    '/monitor/logininfor/list',
    { params: query },
  );
}

// 删除登录日志
export function delLogininfor(infoId: number | number[]) {
  return requestClient.delete(`/monitor/logininfor/${infoId}`);
}

// 解锁用户登录状态
export function unlockLogininfor(userName: string) {
  return requestClient.get(`/monitor/logininfor/unlock/${userName}`);
}

// 清空登录日志
export function cleanLogininfor() {
  return requestClient.delete('/monitor/logininfor/clean');
}
