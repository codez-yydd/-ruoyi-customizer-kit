import { requestClient } from '#/api/request';

export interface SysJob {
  jobId: number;
  jobName: string;
  jobGroup: string;
  invokeTarget: string;
  cronExpression: string;
  misfirePolicy: string;
  concurrent: string;
  status: string;
  remark?: string;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

export function listJob(query: Record<string, any>) {
  return requestClient.get<TableResult<SysJob>>('/monitor/job/list', { params: query });
}

export function getJob(jobId: number) {
  return requestClient.get<{ data: SysJob }>(`/monitor/job/${jobId}`);
}

export function addJob(data: Partial<SysJob>) {
  return requestClient.post('/monitor/job', data);
}

export function updateJob(data: Partial<SysJob>) {
  return requestClient.put('/monitor/job', data);
}

export function delJob(jobId: number) {
  return requestClient.delete(`/monitor/job/${jobId}`);
}

export function changeJobStatus(jobId: number, status: string) {
  return requestClient.put('/monitor/job/changeStatus', { jobId, status });
}

export function runJob(jobId: number, jobGroup: string) {
  return requestClient.put('/monitor/job/run', { jobId, jobGroup });
}
