import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'

/** 通知公告行（type alias：满足 a-table 行数据索引签名兼容） */
export type SysNotice = {
  noticeId: number
  noticeTitle: string
  /** 公告类型（1 通知 2 公告，字典 sys_notice_type） */
  noticeType?: string
  /** 公告内容（富文本 HTML） */
  noticeContent?: string
  /** 状态（0 正常 1 关闭，字典 sys_notice_status） */
  status?: string
  /** 当前用户是否已读（本后端定制字段） */
  isRead?: boolean
  /** 创建者 */
  createBy?: string
  createTime?: string
  remark?: string
}

/** 公告分页查询参数 */
export type NoticeQuery = PageQuery & {
  noticeTitle?: string
  noticeType?: string
  /** 操作人员（按 create_by 模糊查询） */
  createBy?: string
}

/** 公告新增/修改入参 */
export type SysNoticeForm = Partial<SysNotice>

/** 公告分页列表：GET /system/notice/list */
export function listNotice(query: NoticeQuery): Promise<PageResult<SysNotice>> {
  return request.get<PageResult<SysNotice>, PageResult<SysNotice>>('/system/notice/list', {
    params: query
  })
}

/** 公告详情：GET /system/notice/{noticeId} */
export function getNotice(noticeId: number): Promise<SysNotice> {
  return request.get<SysNotice, SysNotice>(`/system/notice/${noticeId}`)
}

/** 新增公告：POST /system/notice */
export function addNotice(data: SysNoticeForm): Promise<void> {
  return request.post('/system/notice', data)
}

/** 修改公告：PUT /system/notice */
export function updateNotice(data: SysNoticeForm): Promise<void> {
  return request.put('/system/notice', data)
}

/** 删除公告：DELETE /system/notice/{noticeIds}（多个逗号拼接） */
export function delNotice(noticeIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/notice/${noticeIds}`)
}
