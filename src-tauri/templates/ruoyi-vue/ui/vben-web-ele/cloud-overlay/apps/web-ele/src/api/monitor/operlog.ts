import { requestClient } from '#/api/request';

/**
 * 操作日志（移植自 ruoyi-ui/src/api/monitor/operlog.js）
 * Cloud 网关 Path=/system/** StripPrefix=1，接口走 /system/operlog
 */
export interface SysOperLog {
  operId: number;
  title: string;
  businessType: number;
  businessTypes?: number[];
  method?: string;
  requestMethod?: string;
  operatorType?: number;
  operName?: string;
  deptName?: string;
  operUrl?: string;
  operIp?: string;
  operLocation?: string;
  operParam?: string;
  jsonResult?: string;
  status: number;
  errorMsg?: string;
  operTime: string;
  costTime?: number;
  costTimeStr?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

// 查询操作日志列表
export function listOperlog(query: Record<string, any>) {
  return requestClient.get<TableResult<SysOperLog>>('/system/operlog/list', {
    params: query,
  });
}

// 删除操作日志
export function delOperlog(operId: number | number[]) {
  return requestClient.delete(`/system/operlog/${operId}`);
}

// 清空操作日志
export function cleanOperlog() {
  return requestClient.delete('/system/operlog/clean');
}

/**
 * POST /system/operlog/export —— 导出操作日志 Excel。
 * 后端无 @RequestBody，条件放在 query 上才能绑定到实体字段与 params Map。
 */
export function exportOperlog(query?: Record<string, any>) {
  return requestClient.post('/system/operlog/export', null, {
    params: query,
    responseType: 'blob',
  });
}
