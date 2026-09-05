import { requestClient } from '#/api/request';

/**
 * 定时任务调度日志（移植自 ruoyi-ui/src/api/monitor/jobLog.js）
 * Cloud Controller 为 /job/log，经网关 /schedule/job/log（不是 /jobLog）
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
  return requestClient.get<TableResult<SysJobLog>>('/schedule/job/log/list', {
    params: query,
  });
}

export function delJobLog(jobLogId: number | number[]) {
  return requestClient.delete(`/schedule/job/log/${jobLogId}`);
}

export function cleanJobLog() {
  return requestClient.delete('/schedule/job/log/clean');
}

/**
 * POST /schedule/job/log/export —— 导出调度日志 Excel。
 *
 * 后端用表单/查询参数绑定 SysJobLog（无 @RequestBody），故条件走 params。
 * 不能用 requestClient.download（内部会改写成 GET）。
 */
export function exportJobLog(query?: Record<string, any>) {
  return requestClient.post('/schedule/job/log/export', null, {
    params: query,
    responseType: 'blob',
  });
}
