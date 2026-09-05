import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/** 登录日志行（type alias：满足 a-table 行数据索引签名兼容） */
export type SysLogininfor = {
  infoId: number
  userName?: string
  /** 登录状态（0成功 1失败，对应字典 sys_common_status） */
  status?: string
  ipaddr?: string
  loginLocation?: string
  browser?: string
  os?: string
  /** 提示消息 */
  msg?: string
  loginTime?: string
}

/** 登录日志查询参数（时间范围经 params[beginTime]/params[endTime] 传递） */
export type LogininforQuery = PageQuery & {
  userName?: string
  ipaddr?: string
  status?: string
  params?: { beginTime?: string; endTime?: string }
}

/** 登录日志分页列表：GET /system/logininfor/list（Cloud 网关 Path=/system/** StripPrefix=1） */
export function listLogininfor(query: LogininforQuery): Promise<PageResult<SysLogininfor>> {
  return request.get<PageResult<SysLogininfor>, PageResult<SysLogininfor>>(
    '/system/logininfor/list',
    { params: query }
  )
}

/** 批量删除登录日志：DELETE /system/logininfor/{infoIds}（多个逗号拼接） */
export function delLogininfor(infoIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/logininfor/${infoIds}`)
}

/** 清空登录日志：DELETE /system/logininfor/clean */
export function cleanLogininfor(): Promise<void> {
  return request.delete('/system/logininfor/clean')
}

/**
 * 解锁账号（登录失败次数清零）：GET /system/logininfor/unlock/{userName}
 * 注意：实测后端为 GET 路径参数形式（非 POST query）
 */
export function unlockLogininfor(userName: string): Promise<void> {
  return request.get(`/system/logininfor/unlock/${encodeURIComponent(userName)}`)
}

/** 导出登录日志：POST /system/logininfor/export */
export function exportLogininfor(query: LogininforQuery, fileName = '登录日志.xlsx'): Promise<void> {
  return exportRequest('/system/logininfor/export', query, fileName)
}
