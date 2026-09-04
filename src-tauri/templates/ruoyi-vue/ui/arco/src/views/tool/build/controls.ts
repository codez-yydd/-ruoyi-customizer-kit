/** 表单构建支持的控件类型 */
export type FieldType = 'input' | 'textarea' | 'select' | 'radio' | 'checkbox' | 'switch' | 'date' | 'rate'

/** 画布表单项 */
export interface BuildField {
  /** 字段名（唯一） */
  field: string
  /** 标签名称 */
  label: string
  type: FieldType
  placeholder: string
  /** 是否必填 */
  required: boolean
  /** 选项文本（select/radio/checkbox 使用） */
  options: string[]
}

/** 控件类型是否需要选项编辑 */
export function hasOptions(type: FieldType): boolean {
  return type === 'select' || type === 'radio' || type === 'checkbox'
}
