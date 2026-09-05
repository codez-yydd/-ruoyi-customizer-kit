import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/**
 * 定时任务（type alias：满足 a-table 行数据索引签名兼容）。
 * status：0 正常 1 暂停（字典 sys_job_status）；
 * jobGroup：DEFAULT/SYSTEM（字典 sys_job_group）；
 * misfirePolicy：1 立即执行 2 执行一次 3 放弃执行；
 * concurrent：0 允许 1 禁止。
 */
export type SysJob = {
  jobId: number
  jobName: string
  jobGroup: string
  invokeTarget: string
  cronExpression?: string
  misfirePolicy?: string
  concurrent?: string
  status?: string
  remark?: string
  createBy?: string
  createTime?: string
  nextValidTime?: string
}

/** 定时任务查询参数 */
export type JobQuery = PageQuery & {
  jobName?: string
  jobGroup?: string
  status?: string
}

/** 定时任务分页列表：GET /schedule/job/list（Cloud 网关 Path=/schedule/** → job 服务 /job） */
export function listJob(query: JobQuery): Promise<PageResult<SysJob>> {
  return request.get<PageResult<SysJob>, PageResult<SysJob>>('/schedule/job/list', { params: query })
}

/** 任务详情：GET /schedule/job/{jobId} */
export function getJob(jobId: number): Promise<SysJob> {
  return request.get<SysJob, SysJob>(`/schedule/job/${jobId}`)
}

/** 新增任务：POST /schedule/job */
export function addJob(data: Partial<SysJob>): Promise<void> {
  return request.post('/schedule/job', data)
}

/** 修改任务：PUT /schedule/job */
export function updateJob(data: Partial<SysJob>): Promise<void> {
  return request.put('/schedule/job', data)
}

/** 批量删除任务：DELETE /schedule/job/{jobIds}（多个逗号拼接） */
export function delJob(jobIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/schedule/job/${jobIds}`)
}

/** 修改任务状态：PUT /schedule/job/changeStatus */
export function changeJobStatus(jobId: number, status: string): Promise<void> {
  return request.put('/schedule/job/changeStatus', { jobId, status })
}

/** 立即执行一次：PUT /schedule/job/run（后端 @RequestBody SysJob，需带 jobId/jobGroup） */
export function runJobOnce(jobId: number, jobGroup?: string): Promise<void> {
  return request.put('/schedule/job/run', { jobId, jobGroup })
}

/** 导出定时任务：POST /schedule/job/export */
export function exportJob(query: JobQuery, fileName = '定时任务.xlsx'): Promise<void> {
  return exportRequest('/schedule/job/export', query, fileName)
}

/**
 * 调度日志行。
 * status：0 成功 1 失败（字典 sys_common_status）。
 */
export type SysJobLog = {
  jobLogId: number
  jobName?: string
  jobGroup?: string
  invokeTarget?: string
  /** 日志信息 */
  jobMessage?: string
  status?: string
  /** 异常信息 */
  jobException?: string
  createTime?: string
}

/** 调度日志查询参数（时间范围经 params[beginTime]/params[endTime] 传递） */
export type JobLogQuery = PageQuery & {
  jobName?: string
  jobGroup?: string
  status?: string
  params?: { beginTime?: string; endTime?: string }
}

/** 调度日志分页列表：GET /schedule/job/log/list（Cloud Controller 为 /job/log，不是 /jobLog） */
export function listJobLog(query: JobLogQuery): Promise<PageResult<SysJobLog>> {
  return request.get<PageResult<SysJobLog>, PageResult<SysJobLog>>('/schedule/job/log/list', {
    params: query
  })
}

/** 批量删除调度日志：DELETE /schedule/job/log/{jobLogIds}（多个逗号拼接） */
export function delJobLog(jobLogIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/schedule/job/log/${jobLogIds}`)
}

/** 清空调度日志：DELETE /schedule/job/log/clean */
export function cleanJobLog(): Promise<void> {
  return request.delete('/schedule/job/log/clean')
}

/** 导出调度日志：POST /schedule/job/log/export */
export function exportJobLog(query: JobLogQuery, fileName = '调度日志.xlsx'): Promise<void> {
  return exportRequest('/schedule/job/log/export', query, fileName)
}
