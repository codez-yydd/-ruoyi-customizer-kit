import { requestClient } from '#/api/request';

/**
 * 代码生成 API（对应后端 GenController /tool/gen）
 */

export interface GenTable {
  tableId: number;
  tableName: string;
  tableComment?: string;
  className?: string;
  tplCategory?: string;
  tplWebType?: string;
  packageName?: string;
  moduleName?: string;
  businessName?: string;
  functionName?: string;
  functionAuthor?: string;
  genType?: string;
  genPath?: string;
  options?: string;
  remark?: string;
  createTime?: string;
  updateTime?: string;
  /** 表单布局列数 */
  formColNum?: number;
  /** 是否生成详情页（前端勾选，提交时写入 params.genView） */
  view?: boolean;
  parentMenuId?: number | string;
  treeCode?: string;
  treeName?: string;
  treeParentCode?: string;
  subTableName?: string;
  subTableFkName?: string;
  columns?: GenTableColumn[];
  params?: Record<string, any>;
}

export interface GenTableColumn {
  columnId?: number;
  tableId?: number;
  columnName?: string;
  columnComment?: string;
  columnType?: string;
  javaType?: string;
  javaField?: string;
  isPk?: string;
  isIncrement?: string;
  isRequired?: string;
  isInsert?: string;
  isEdit?: string;
  isList?: string;
  isQuery?: string;
  queryType?: string;
  htmlType?: string;
  dictType?: string;
  sort?: number;
}

interface TableResult<T> {
  rows: T[];
  total: number;
}

/** 查询已导入的生成表列表 */
export function listTable(query: Record<string, any>) {
  return requestClient.get<TableResult<GenTable>>('/tool/gen/list', {
    params: query,
  });
}

/** 查询数据库表列表（导入用） */
export function listDbTable(query: Record<string, any>) {
  return requestClient.get<TableResult<GenTable>>('/tool/gen/db/list', {
    params: query,
  });
}

/**
 * 查询表详细信息（含字段、全部表）
 *
 * 后端返回 {code,msg,data:{info,rows,tables}}，拦截器解包后为 {info,rows,tables}。
 */
export function getGenTable(tableId: number | string) {
  return requestClient.get<{
    info: GenTable;
    rows: GenTableColumn[];
    tables: GenTable[];
  }>(`/tool/gen/${tableId}`);
}

/** 修改代码生成配置 */
export function updateGenTable(data: Partial<GenTable>) {
  return requestClient.put('/tool/gen', data);
}

/**
 * 导入表结构
 * 使用 query 参数（与若依原版一致），需保留完整响应以读取 msg。
 */
export function importTable(data: { tables: string; tplWebType: string }) {
  return requestClient.post('/tool/gen/importTable', undefined, {
    params: data,
    rawResponse: true,
  });
}

/**
 * 创建表（执行建表 SQL）
 * 仅 admin 角色可用，参数同样走 query。
 */
export function createTable(data: { sql: string; tplWebType: string }) {
  return requestClient.post('/tool/gen/createTable', undefined, {
    params: data,
    rawResponse: true,
  });
}

/** 预览生成代码，返回模板路径 → 代码内容映射 */
export function previewTable(tableId: number) {
  return requestClient.get<Record<string, string>>(
    `/tool/gen/preview/${tableId}`,
  );
}

/** 删除已导入的表配置 */
export function delTable(tableId: number | number[] | string) {
  return requestClient.delete(`/tool/gen/${tableId}`);
}

/** 生成代码到自定义路径 */
export function genCode(tableName: string) {
  return requestClient.get(`/tool/gen/genCode/${tableName}`);
}

/** 同步数据库表结构 */
export function synchDb(tableName: string) {
  return requestClient.get(`/tool/gen/synchDb/${tableName}`);
}

/**
 * 批量下载 zip 代码包
 * GET 返回二进制流，不能走普通 JSON 拦截解包。
 */
export function downloadBatchGenCode(tables: string) {
  return requestClient.get('/tool/gen/batchGenCode', {
    params: { tables },
    responseType: 'blob',
  });
}
