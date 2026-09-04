import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'
import { downloadBlob } from '@/utils/download'
import type { AxiosResponse } from 'axios'

/**
 * 代码生成业务表（type alias：满足 a-table 行数据索引签名兼容）。
 * list 接口返回全量字段，db/list（待导入库表）大多数字段为 null。
 */
export type GenTable = {
  tableId: number
  tableName: string
  tableComment?: string
  className?: string
  tplCategory?: string
  tplWebType?: string
  packageName?: string
  moduleName?: string
  businessName?: string
  functionName?: string
  functionAuthor?: string
  formColNum?: number
  genType?: string
  genPath?: string
  remark?: string
  createTime?: string
  updateTime?: string
}

/** 业务表查询参数 */
export type GenQuery = PageQuery & {
  tableName?: string
  tableComment?: string
}

/** 业务表字段行（GET /tool/gen/{tableId} 的 rows） */
export interface GenTableColumn {
  columnId: number
  tableId: number
  columnName: string
  columnComment?: string
  columnType?: string
  javaType?: string
  javaField?: string
  isPk?: string
  isIncrement?: string
  isRequired?: string
  isInsert?: string
  isEdit?: string
  isList?: string
  isQuery?: string
  queryType?: string
  htmlType?: string
  dictType?: string
  sort?: number
}

/** GET /tool/gen/{tableId} 响应 data（实测：{info, rows, tables}） */
export interface GenDetailResult {
  info: GenTable
  rows: GenTableColumn[]
  /** 关联子表/父表信息（主表场景可能为 null） */
  tables?: unknown
}

/** GET /tool/gen/preview/{tableId} 响应 data：模板路径 -> 代码内容 */
export type GenPreviewResult = Record<string, string>

/** 业务表分页列表：GET /tool/gen/list */
export function listGen(query: GenQuery): Promise<PageResult<GenTable>> {
  return request.get<PageResult<GenTable>, PageResult<GenTable>>('/tool/gen/list', {
    params: query
  })
}

/** 业务表详情（含字段列表）：GET /tool/gen/{tableId} */
export function getGenTable(tableId: number): Promise<GenDetailResult> {
  return request.get<GenDetailResult, GenDetailResult>(`/tool/gen/${tableId}`)
}

/** 待导入库表分页列表：GET /tool/gen/db/list */
export function listGenDb(query: GenQuery): Promise<PageResult<GenTable>> {
  return request.get<PageResult<GenTable>, PageResult<GenTable>>('/tool/gen/db/list', {
    params: query
  })
}

/**
 * 导入业务表：POST /tool/gen/importTable
 * 后端为 @RequestParam 形式，tables 多个逗号拼接
 */
export function importGenTable(tables: string, tplWebType: string): Promise<void> {
  return request.post('/tool/gen/importTable', undefined, {
    params: { tables, tplWebType }
  })
}

/** 批量删除业务表：DELETE /tool/gen/{tableIds}（多个逗号拼接） */
export function delGenTable(tableIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/tool/gen/${tableIds}`)
}

/** 同步表结构：GET /tool/gen/synchDb/{tableName} */
export function synchGenTable(tableName: string): Promise<void> {
  return request.get(`/tool/gen/synchDb/${encodeURIComponent(tableName)}`)
}

/** 生成预览：GET /tool/gen/preview/{tableId} */
export function previewGenTable(tableId: number): Promise<GenPreviewResult> {
  return request.get<GenPreviewResult, GenPreviewResult>(`/tool/gen/preview/${tableId}`)
}

/** 生成代码下载（zip 流）：GET /tool/gen/download/{tableName} */
export async function downloadGenCode(tableName: string, fileName = 'ruoyi.zip'): Promise<void> {
  // blob 场景响应拦截器原样返回 AxiosResponse（见 request.ts）
  const response = await request.get<unknown, AxiosResponse<Blob>>(`/tool/gen/download/${encodeURIComponent(tableName)}`, {
    responseType: 'blob'
  })
  downloadBlob(response, fileName)
}
