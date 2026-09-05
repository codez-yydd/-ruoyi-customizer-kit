import { requestClient } from '#/api/request';

/**
 * 登录日志（移植自 ruoyi-ui/src/api/monitor/logininfor.js）
 * Cloud 网关 Path=/system/** StripPrefix=1，接口走 /system/logininfor
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
    '/system/logininfor/list',
    { params: query },
  );
}

// 删除登录日志
export function delLogininfor(infoId: number | number[]) {
  return requestClient.delete(`/system/logininfor/${infoId}`);
}

// 解锁用户登录状态
export function unlockLogininfor(userName: string) {
  return requestClient.get(`/system/logininfor/unlock/${userName}`);
}

// 清空登录日志
export function cleanLogininfor() {
  return requestClient.delete('/system/logininfor/clean');
}

/**
 * POST /system/logininfor/export —— 导出登录日志 Excel。
 * 后端无 @RequestBody，条件放在 query 上才能绑定到实体字段与 params Map。
 */
export function exportLogininfor(query?: Record<string, any>) {
  return requestClient.post('/system/logininfor/export', null, {
    params: query,
    responseType: 'blob',
  });
}
