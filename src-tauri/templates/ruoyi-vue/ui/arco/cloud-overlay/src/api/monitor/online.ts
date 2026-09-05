import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'

/** 在线用户会话（type alias：满足 a-table 行数据索引签名兼容） */
export type OnlineUser = {
  /** 会话编号（UUID，同时作为强退路径参数） */
  tokenId: string
  deptName?: string
  userName: string
  ipaddr?: string
  loginLocation?: string
  browser?: string
  os?: string
  /** 登录时间（epoch 毫秒） */
  loginTime?: number
}

/** 在线用户查询参数（登录名/IP 模糊） */
export type OnlineQuery = PageQuery & {
  userName?: string
  ipaddr?: string
}

/** 在线用户分页列表：GET /system/online/list（Cloud 网关 Path=/system/** StripPrefix=1） */
export function listOnline(query: OnlineQuery): Promise<PageResult<OnlineUser>> {
  return request.get<PageResult<OnlineUser>, PageResult<OnlineUser>>('/system/online/list', {
    params: query
  })
}

/** 强退在线会话：DELETE /system/online/{tokenId} */
export function forceLogout(tokenId: string): Promise<void> {
  return request.delete(`/system/online/${tokenId}`)
}
