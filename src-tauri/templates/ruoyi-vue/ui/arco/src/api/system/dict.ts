import request from '@/api/request'
import type { PageQuery, PageResult } from '@/api/types'

/**
 * 字典数据选项（useDict / DictTag 使用）
 * 来源：GET /system/dict/data/type/{dictType}
 */
export interface DictDataOption {
  dictLabel: string
  dictValue: string
  dictType: string
  /** 自定义样式类名（后端配置，可能为 null） */
  cssClass: string | null
  /** 标签色彩：default/primary/success/info/warning/danger */
  listClass: string
  dictSort: number
}

/** 字典类型行（type alias：满足 CrudRecord/TableData 索引签名兼容） */
export type SysDictType = {
  dictId: number
  dictName: string
  dictType: string
  /** 状态（0 正常 1 停用） */
  status: string
  createTime?: string
  remark?: string
}

/** 字典数据行（type alias：满足 CrudRecord/TableData 索引签名兼容） */
export type SysDictData = {
  dictCode: number
  dictLabel: string
  dictValue: string
  dictType: string
  cssClass: string
  listClass: string
  dictSort: number
  /** 是否系统内置（Y 是 N 否，字典 sys_yes_no） */
  isDefault: string
  /** 状态（0 正常 1 停用） */
  status: string
  createTime?: string
  remark?: string
}

/** 字典类型查询参数（创建时间范围经 params[beginTime]/params[endTime] 传递） */
export interface DictTypeQuery extends PageQuery {
  dictName?: string
  dictType?: string
  status?: string
  params?: { beginTime?: string; endTime?: string }
}

/** 字典数据查询参数 */
export interface DictDataQuery extends PageQuery {
  dictLabel?: string
  dictType?: string
  status?: string
}

/** 字典类型新增/修改入参 */
export type SysDictTypeForm = Partial<SysDictType>

/** 字典数据新增/修改入参 */
export type SysDictDataForm = Partial<SysDictData>

/* ==================== 字典业务数据（useDict 缓存数据源） ==================== */

/** 按字典类型取字典数据：GET /system/dict/data/type/{dictType} */
export function getDictByType(dictType: string): Promise<DictDataOption[]> {
  return request.get<DictDataOption[], DictDataOption[]>(`/system/dict/data/type/${dictType}`)
}

/* ==================== 字典类型管理 ==================== */

/** 字典类型分页列表：GET /system/dict/type/list */
export function listType(query: DictTypeQuery): Promise<PageResult<SysDictType>> {
  return request.get<PageResult<SysDictType>, PageResult<SysDictType>>('/system/dict/type/list', {
    params: query
  })
}

/** 字典类型详情：GET /system/dict/type/{dictId} */
export function getType(dictId: number): Promise<SysDictType> {
  return request.get<SysDictType, SysDictType>(`/system/dict/type/${dictId}`)
}

/** 新增字典类型：POST /system/dict/type */
export function addType(data: SysDictTypeForm): Promise<void> {
  return request.post('/system/dict/type', data)
}

/** 修改字典类型：PUT /system/dict/type */
export function updateType(data: SysDictTypeForm): Promise<void> {
  return request.put('/system/dict/type', data)
}

/** 删除字典类型：DELETE /system/dict/type/{dictIds}（多个逗号拼接） */
export function delType(dictIds: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/dict/type/${dictIds}`)
}

/** 刷新字典缓存：DELETE /system/dict/type/refreshCache */
export function refreshCache(): Promise<void> {
  return request.delete('/system/dict/type/refreshCache')
}

/* ==================== 字典数据管理 ==================== */

/** 字典数据分页列表：GET /system/dict/data/list */
export function listData(query: DictDataQuery): Promise<PageResult<SysDictData>> {
  return request.get<PageResult<SysDictData>, PageResult<SysDictData>>('/system/dict/data/list', {
    params: query
  })
}

/** 字典数据详情：GET /system/dict/data/{dictCode} */
export function getData(dictCode: number): Promise<SysDictData> {
  return request.get<SysDictData, SysDictData>(`/system/dict/data/${dictCode}`)
}

/** 新增字典数据：POST /system/dict/data */
export function addData(data: SysDictDataForm): Promise<void> {
  return request.post('/system/dict/data', data)
}

/** 修改字典数据：PUT /system/dict/data */
export function updateData(data: SysDictDataForm): Promise<void> {
  return request.put('/system/dict/data', data)
}

/** 删除字典数据：DELETE /system/dict/data/{dictCodes}（多个逗号拼接） */
export function delData(dictCodes: number | string | Array<number | string>): Promise<void> {
  return request.delete(`/system/dict/data/${dictCodes}`)
}
