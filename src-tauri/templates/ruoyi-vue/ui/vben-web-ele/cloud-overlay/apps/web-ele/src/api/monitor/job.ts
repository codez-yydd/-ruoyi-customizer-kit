import { requestClient } from '#/api/request';

/**
 * 定时任务（移植自 ruoyi-ui/src/api/monitor/job.js）
 * Cloud 网关 Path=/schedule/** → job 服务 /job
 */
export interface SysJob {
  jobId: number;
  jobName: string;
  jobGroup: string;
  invokeTarget: string;
  cronExpression: string;
  /** 下次执行时间（后端按 cron 计算，仅详情回显） */
  nextValidTime?: string;
  misfirePolicy: string;
  concurrent: string;
  status: string;
  remark?: string;
  createBy?: string;
  createTime?: string;
  updateBy?: string;
  updateTime?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listJob(query: Record<string, any>) {
  return requestClient.get<TableResult<SysJob>>('/schedule/job/list', {
    params: query,
  });
}

/**
 * GET /schedule/job/{jobId} —— 任务详情
 *
 * 必须设置 rawResponse: true，跳过全局拦截器对 data 的自动解包。
 * 否则页面里 Object.assign(form, res.data) 的 res.data 会是 undefined，
 * 导致修改弹框无法回显数据。
 */
export function getJob(jobId: number) {
  return requestClient.get<{ data: SysJob }>(`/schedule/job/${jobId}`, {
    rawResponse: true,
  });
}

export function addJob(data: Partial<SysJob>) {
  return requestClient.post('/schedule/job', data);
}

export function updateJob(data: Partial<SysJob>) {
  return requestClient.put('/schedule/job', data);
}

export function delJob(jobId: number | number[]) {
  return requestClient.delete(`/schedule/job/${jobId}`);
}

export function changeJobStatus(jobId: number, status: string) {
  return requestClient.put('/schedule/job/changeStatus', { jobId, status });
}

export function runJob(jobId: number, jobGroup: string) {
  return requestClient.put('/schedule/job/run', { jobId, jobGroup });
}

/**
 * POST /schedule/job/export —— 导出定时任务 Excel。
 *
 * 后端用表单/查询参数绑定 SysJob（无 @RequestBody），故条件走 params。
 * 不能用 requestClient.download（内部会改写成 GET）。
 */
export function exportJob(query?: Record<string, any>) {
  return requestClient.post('/schedule/job/export', null, {
    params: query,
    responseType: 'blob',
  });
}
