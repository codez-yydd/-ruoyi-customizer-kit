import { requestClient } from '#/api/request';

/**
 * 定时任务调度日志（移植自 ruoyi-ui/src/api/monitor/jobLog.js）
 */
export interface SysJobLog {
  jobLogId: number;
  jobName: string;
  jobGroup: string;
  invokeTarget: string;
  jobMessage?: string;
  /** 执行状态（0正常 1失败） */
  status: string;
  exceptionInfo?: string;
  startTime?: string;
  endTime?: string;
  createTime?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listJobLog(query: Record<string, any>) {
  return requestClient.get<TableResult<SysJobLog>>('/monitor/jobLog/list', {
    params: query,
  });
}

export function delJobLog(jobLogId: number | number[]) {
  return requestClient.delete(`/monitor/jobLog/${jobLogId}`);
}

export function cleanJobLog() {
  return requestClient.delete('/monitor/jobLog/clean');
}

/**
 * POST /monitor/jobLog/export —— 导出调度日志 Excel。
 *
 * 后端用表单/查询参数绑定 SysJobLog（无 @RequestBody），故条件走 params。
 * 不能用 requestClient.download（内部会改写成 GET）。
 */
export function exportJobLog(query?: Record<string, any>) {
  return requestClient.post('/monitor/jobLog/export', null, {
    params: query,
    responseType: 'blob',
  });
}
