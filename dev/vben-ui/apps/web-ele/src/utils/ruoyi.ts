/**
 * 若依工具函数（移植自 ruoyi-ui/src/utils/ruoyi.js，Vue3 版）
 */

/**
 * 时间格式化（移植自若依 parseTime）
 * @param time 时间对象/字符串/时间戳
 * @param pattern 格式，默认 '{y}-{m}-{d} {h}:{i}:{s}'
 */
export function parseTime(
  time: string | number | Date | null | undefined,
  pattern?: string,
): string {
  if (!time) return '';
  const format = pattern || '{y}-{m}-{d} {h}:{i}:{s}';
  let date: Date;
  if (typeof time === 'object') {
    date = time as Date;
  } else {
    if (typeof time === 'string' && /^\d+$/.test(time)) {
      time = parseInt(time, 10);
    }
    if (typeof time === 'number' && time.toString().length === 10) {
      time = time * 1000;
    }
    date = new Date(time as any);
  }
  const formatObj: Record<string, number> = {
    y: date.getFullYear(),
    m: date.getMonth() + 1,
    d: date.getDate(),
    h: date.getHours(),
    i: date.getMinutes(),
    s: date.getSeconds(),
    a: date.getDay(),
  };
  return format.replace(/\{([ymdhis])\}/g, (_result, key) => {
    const value = formatObj[key] ?? 0;
    return (
      key === 'a'
        ? ['日', '一', '二', '三', '四', '五', '六'][value]
        : value.toString().padStart(2, '0')
    ) as string;
  });
}

/**
 * 合并日期范围到查询参数（移植自若依 addDateRange）
 * 若依列表接口接收 beginXxx / endXxx 时间范围参数
 * @param params 原始查询参数
 * @param dateRange [开始, 结束] 日期数组
 * @param propName 字段名前缀（如 createTime → beginCreateTime / endCreateTime）
 */
export function addDateRange<T extends Record<string, any>>(
  params: T,
  dateRange: [string, string] | [],
  propName?: string,
): T {
  const result = { ...params } as any;
  if (dateRange && dateRange.length === 2) {
    if (propName) {
      result[`begin${propName}`] = dateRange[0];
      result[`end${propName}`] = dateRange[1];
    } else {
      result.beginTime = dateRange[0];
      result.endTime = dateRange[1];
    }
  }
  return result;
}

/**
 * 翻译字典值（手动回显，不依赖 DictTag 组件的场景）
 */
export function selectDictLabel(
  dicts: { dictValue: string; dictLabel: string }[],
  value: string,
): string {
  return dicts.find((d) => d.dictValue === value)?.dictLabel ?? '';
}

/**
 * 字符串空值转空字符串
 */
export function parseStrEmpty(str: any): string {
  if (!str || str === 'undefined' || str === 'null') {
    return '';
  }
  return String(str);
}

/**
 * 深拷贝（简化版，用于树形数据操作）
 */
export function deepClone<T>(obj: T): T {
  return JSON.parse(JSON.stringify(obj));
}
