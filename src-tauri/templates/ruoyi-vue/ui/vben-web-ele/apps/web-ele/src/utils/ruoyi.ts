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
 *
 * 若依 Mapper 通过 BaseEntity.params 读取时间范围（如 params.beginTime），
 * 因此必须写入嵌套对象 params，GET 序列化为 params[beginTime]=xxx，
 * Spring 才能正确绑定到 Map，不能写到查询参数顶层。
 *
 * @param params 原始查询参数
 * @param dateRange [开始, 结束] 日期数组
 * @param propName 字段名后缀；不传则使用 beginTime/endTime
 */
export function addDateRange<T extends Record<string, any>>(
  params: T,
  dateRange: [string, string] | [],
  propName?: string,
): T {
  const result = { ...params } as any;
  // 保留已有 params，避免覆盖其它动态查询条件
  result.params =
    typeof result.params === 'object' &&
    result.params !== null &&
    !Array.isArray(result.params)
      ? { ...result.params }
      : {};
  const range = Array.isArray(dateRange) ? dateRange : [];
  if (range.length === 2) {
    if (propName) {
      result.params[`begin${propName}`] = range[0];
      result.params[`end${propName}`] = range[1];
    } else {
      result.params.beginTime = range[0];
      result.params.endTime = range[1];
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

/**
 * 触发浏览器下载一个 Blob（不依赖 file-saver 等三方库）。
 */
function triggerBlobDownload(blob: Blob, filename: string) {
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  window.URL.revokeObjectURL(url);
}

/**
 * 保存若依文件下载响应（导出 Excel / 下载模板）。
 *
 * 若依导出有两种情况：
 *   ① 正常：HTTP 200，Content-Disposition: attachment; filename=xxx.xlsx，
 *      响应体是 Excel 二进制（Blob）。直接保存。
 *   ② 失败：HTTP 200 但业务出错（如权限不足），响应体是 JSON 字符串
 *      {code,msg}（被 responseType:blob 误包成了 Blob）。
 *      需先把 Blob 读成文本，再按 JSON 解析，提示后端返回的 msg。
 *
 * @param response fetch/axios 响应对象（含 headers 和 data:Blob）
 * @param fallbackName 后备文件名（响应头里没带 filename 时用）
 * @returns 成功保存返回 true；导出失败（响应体是 JSON 错误）返回 false 并已提示
 */
export async function saveBlobFile(
  response: {
    headers?: Record<string, any> | any;
    data: Blob;
  },
  fallbackName: string,
): Promise<boolean> {
  const blob = response.data;

  // 若依导出失败时，返回的是 JSON 错误体（被 responseType:blob 包成 Blob），
  // 此类 Blob 的 type 为 application/json，需读取后按业务码提示。
  if (blob && blob.type && blob.type.includes('application/json')) {
    const text = await blob.text();
    try {
      const json = JSON.parse(text);
      // ElMessage 延迟引入，避免循环依赖；此处直接用 window 提示兜底
      const msg = json?.msg || json?.message || '导出失败';
      // 复用 element-plus 提示（这里通过动态导入，避免与 ruoyi.ts 顶层 import 形成环）
      const { ElMessage } = await import('element-plus');
      ElMessage.error(msg);
    } catch {
      console.error('解析导出失败响应失败：', text);
    }
    return false;
  }

  // 从 Content-Disposition 解析文件名
  // 格式：attachment; filename=xxx.xlsx 或 attachment; filename*=UTF-8''编码后的xxx.xlsx
  let filename = fallbackName;
  const headers = response.headers || {};
  const disposition =
    headers['content-disposition'] || headers['Content-Disposition'];
  if (disposition) {
    // 优先匹配 filename*=UTF-8''xxx
    const utf8Match = /filename\*=(?:UTF-8'')?([^;]+)/i.exec(disposition);
    if (utf8Match?.[1]) {
      try {
        filename = decodeURIComponent(utf8Match[1]);
      } catch {
        filename = utf8Match[1];
      }
    } else {
      const match = /filename="?([^";]+)"?/i.exec(disposition);
      if (match?.[1]) {
        filename = match[1];
      }
    }
  }

  triggerBlobDownload(blob, filename);
  return true;
}
