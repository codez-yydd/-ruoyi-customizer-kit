/**
 * 将逗号分隔字符串转为快速查找函数（Vue 模板编译器常用工具）
 * @param str 逗号分隔的键名列表
 * @param expectsLowerCase 是否按小写匹配
 */
export function makeMap(str: string, expectsLowerCase?: boolean) {
  const map: Record<string, true> = Object.create(null);
  const list = str.split(',');
  for (let i = 0; i < list.length; i++) {
    const key = list[i];
    if (key) {
      map[key] = true;
    }
  }
  return expectsLowerCase
    ? (val: string) => !!map[val.toLowerCase()]
    : (val: string) => !!map[val];
}
