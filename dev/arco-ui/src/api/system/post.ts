import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/** 岗位行（type alias：满足 a-table 行数据索引签名兼容） */
export type SysPost = {
  postId: number
  postCode: string
  postName: string
  /** 岗位排序 */
  postSort?: number
  /** 状态（0 正常 1 停用） */
  status?: string
  /** 后端实体自带标记位（序列化输出，无业务含义） */
  flag?: boolean
  remark?: string
  createTime?: string
}

/** 岗位分页查询参数 */
export type PostQuery = PageQuery & {
  postCode?: string
  postName?: string
  status?: string
}

/** 岗位新增/修改入参 */
export type SysPostForm = Partial<SysPost>

/** 岗位分页列表：GET /system/post/list */
export function listPost(query: PostQuery): Promise<PageResult<SysPost>> {
  return request.get<PageResult<SysPost>, PageResult<SysPost>>('/system/post/list', {
    params: query
  })
}

/** 岗位详情：GET /system/post/{postId} */
export function getPost(postId: number): Promise<SysPost> {
  return request.get<SysPost, SysPost>(`/system/post/${postId}`)
}

/** 新增岗位：POST /system/post */
export function addPost(data: SysPostForm): Promise<void> {
  return request.post('/system/post', data)
}

/** 修改岗位：PUT /system/post */
export function updatePost(data: SysPostForm): Promise<void> {
  return request.put('/system/post', data)
}

/** 删除岗位：DELETE /system/post/{postIds}（多个逗号拼接） */
export function delPost(postIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/post/${postIds}`)
}

/** 导出岗位：POST /system/post/export（查询条件经 query string 传递） */
export function exportPost(query: PostQuery, fileName = '岗位数据.xlsx'): Promise<void> {
  return exportRequest('/system/post/export', query, fileName)
}
