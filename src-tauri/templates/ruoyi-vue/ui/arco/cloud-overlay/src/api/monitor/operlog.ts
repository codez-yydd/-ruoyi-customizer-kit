import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/**
 * 操作日志行（type alias：满足 a-table 行数据索引签名兼容）。
 * 注意：后端未提供按 operId 的详情接口（GET /{operId} 不支持），
 * 详情弹窗直接使用列表行数据（列表已包含全部展示字段）。
 */
export type SysOperLog = {
  operId: number
  /** 系统模块标题 */
  title?: string
  /** 业务类型（0其它 1新增 2修改 3删除，对应字典 sys_oper_type） */
  businessType?: number
  businessTypes?: number[]
  /** 调用方法（全限定名） */
  method?: string
  /** 请求方式（GET/POST/PUT/DELETE） */
  requestMethod?: string
  /** 操作类别（1后台用户 2手机端用户） */
  operatorType?: number
  operName?: string
  deptName?: string
  operUrl?: string
  operIp?: string
  operLocation?: string
  /** 请求参数（JSON 字符串） */
  operParam?: string
  /** 返回结果（JSON 字符串） */
  jsonResult?: string
  /** 操作状态（0正常 1异常，对应字典 sys_common_status） */
  status?: number
  errorMsg?: string
  operTime?: string
  /** 消耗时间（毫秒） */
  costTime?: number
}

/** 操作日志查询参数（时间范围经 params[beginTime]/params[endTime] 传递） */
export type OperLogQuery = PageQuery & {
  title?: string
  operName?: string
  businessType?: number | string
  status?: number | string
  params?: { beginTime?: string; endTime?: string }
}

/** 操作日志分页列表：GET /system/operlog/list（Cloud 网关 Path=/system/** StripPrefix=1） */
export function listOperlog(query: OperLogQuery): Promise<PageResult<SysOperLog>> {
  return request.get<PageResult<SysOperLog>, PageResult<SysOperLog>>('/system/operlog/list', {
    params: query
  })
}

/** 批量删除操作日志：DELETE /system/operlog/{operIds}（多个逗号拼接） */
export function delOperlog(operIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/operlog/${operIds}`)
}

/** 清空操作日志：DELETE /system/operlog/clean */
export function cleanOperlog(): Promise<void> {
  return request.delete('/system/operlog/clean')
}

/** 导出操作日志：POST /system/operlog/export */
export function exportOperlog(query: OperLogQuery, fileName = '操作日志.xlsx'): Promise<void> {
  return exportRequest('/system/operlog/export', query, fileName)
}
