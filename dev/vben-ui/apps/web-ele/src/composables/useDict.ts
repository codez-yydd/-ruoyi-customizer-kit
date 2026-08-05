import { reactive } from 'vue';

import { getDictByTypeApi, type DictData } from '#/api/system/dict';

/**
 * 字典数据模块级缓存。
 *
 * 设计（参考若依 Vue3 版 useDict）：
 * - 以 dictType 为 key 缓存字典数据，避免重复请求
 * - 同一 dictType 并发请求时共享 Promise，防止短时间内多次拉取
 * - 退出登录时调用 clearDictCache() 清空
 */
const dictCache = new Map<string, DictData[]>();
const pendingRequests = new Map<string, Promise<DictData[]>>();

/** 按字典类型获取字典数据（带缓存） */
export async function getDict(dictType: string): Promise<DictData[]> {
  // 命中缓存直接返回
  if (dictCache.has(dictType)) {
    return dictCache.get(dictType)!;
  }
  // 已有进行中的请求，复用
  if (pendingRequests.has(dictType)) {
    return pendingRequests.get(dictType)!;
  }
  // 发起新请求
  const promise = getDictByTypeApi(dictType)
    .then((data) => {
      dictCache.set(dictType, data ?? []);
      pendingRequests.delete(dictType);
      return data ?? [];
    })
    .catch((err) => {
      pendingRequests.delete(dictType);
      console.warn(`获取字典 [${dictType}] 失败：`, err);
      return [];
    });
  pendingRequests.set(dictType, promise);
  return promise;
}

/**
 * useDict：在组件中响应式使用字典
 *
 * @example
 * const { dictMap } = useDict({ sys_user_sex: '', sys_normal_disable: '' })
 * // 模板里用 dictMap.sys_user_sex 渲染下拉选项
 */
export function useDict(dictTypes: Record<string, string>) {
  // 用 reactive 作底层存储；外层包一层 Proxy，访问任意 key 都返回 DictData[]（未加载时为 []），
  // 避免模板里 dictMap.xxx 类型为 DictData[] | undefined 导致的类型报错与 v-for 空值问题。
  const store = reactive<Record<string, DictData[]>>({});

  Object.keys(dictTypes).forEach(async (key) => {
    const dictType = dictTypes[key];
    store[key] = await getDict(dictType!);
  });

  const dictMap = new Proxy(store, {
    get(target, prop: string) {
      return target[prop] ?? [];
    },
  }) as Record<string, DictData[]>;

  return { dictMap };
}

/** 清空字典缓存（退出登录时调用） */
export function clearDictCache() {
  dictCache.clear();
  pendingRequests.clear();
}
