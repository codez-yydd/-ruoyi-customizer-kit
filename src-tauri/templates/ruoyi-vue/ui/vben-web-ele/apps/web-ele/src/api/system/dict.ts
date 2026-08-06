import { requestClient } from '#/api/request';

/**
 * 若依 SysDictData 结构
 */
export interface DictData {
  dictCode: number;
  dictSort: number;
  dictLabel: string;
  dictValue: string;
  dictType: string;
  cssClass?: string;
  /** 标签样式类型（default/primary/success/info/warning/danger），DictTag 据此渲染 el-tag type */
  listClass?: string;
  isDefault?: string;
  status?: string;
}

/**
 * 按字典类型查询字典数据（适配若依 GET /system/dict/data/type/{dictType}）
 *
 * 返回 {code, data: DictData[]}，requestClient 拦截器解包 data。
 */
export async function getDictByTypeApi(dictType: string): Promise<DictData[]> {
  return requestClient.get<DictData[]>(
    `/system/dict/data/type/${dictType}`,
  );
}
