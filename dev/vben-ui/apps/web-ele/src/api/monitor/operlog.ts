import { requestClient } from '#/api/request';

/**
 * 操作日志（移植自 ruoyi-ui/src/api/monitor/operlog.js）
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
  return requestClient.get<TableResult<SysOperLog>>('/monitor/operlog/list', {
    params: query,
  });
}

// 删除操作日志
export function delOperlog(operId: number | number[]) {
  return requestClient.delete(`/monitor/operlog/${operId}`);
}

// 清空操作日志
export function cleanOperlog() {
  return requestClient.delete('/monitor/operlog/clean');
}
