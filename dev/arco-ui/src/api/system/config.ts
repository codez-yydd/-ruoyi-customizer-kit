import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/** 参数配置行（type alias：满足 a-table 行数据索引签名兼容） */
export type SysConfig = {
  configId: number
  configName: string
  configKey: string
  configValue?: string
  /** 系统内置（Y 是 N 否） */
  configType?: string
  remark?: string
  createTime?: string
}

/** 参数分页查询参数（创建时间范围经 params[beginTime]/params[endTime] 传递） */
export type ConfigQuery = PageQuery & {
  configName?: string
  configKey?: string
  configType?: string
  params?: { beginTime?: string; endTime?: string }
}

/** 参数新增/修改入参 */
export type SysConfigForm = Partial<SysConfig>

/** 参数分页列表：GET /system/config/list */
export function listConfig(query: ConfigQuery): Promise<PageResult<SysConfig>> {
  return request.get<PageResult<SysConfig>, PageResult<SysConfig>>('/system/config/list', {
    params: query
  })
}

/** 参数详情：GET /system/config/{configId} */
export function getConfig(configId: number): Promise<SysConfig> {
  return request.get<SysConfig, SysConfig>(`/system/config/${configId}`)
}

/** 新增参数：POST /system/config */
export function addConfig(data: SysConfigForm): Promise<void> {
  return request.post('/system/config', data)
}

/** 修改参数：PUT /system/config */
export function updateConfig(data: SysConfigForm): Promise<void> {
  return request.put('/system/config', data)
}

/** 删除参数：DELETE /system/config/{configIds}（多个逗号拼接） */
export function delConfig(configIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/config/${configIds}`)
}

/** 刷新参数缓存：DELETE /system/config/refreshCache */
export function refreshConfigCache(): Promise<void> {
  return request.delete('/system/config/refreshCache')
}

/** 导出参数：POST /system/config/export（查询条件经 query string 传递） */
export function exportConfig(query: ConfigQuery, fileName = '参数数据.xlsx'): Promise<void> {
  return exportRequest('/system/config/export', query, fileName)
}
