import { reactive, ref } from 'vue';

/**
 * 分页 composable：统一若依列表页的分页/查询状态。
 *
 * 若依列表接口约定：GET /xxx/list，参数 pageNum/pageSize + 业务筛选字段，
 * 返回 {code, msg, rows:[], total}（rows/total 在顶层，非 data）。
 *
 * @example
 * const { queryParams, dateRange, handleQuery, resetQuery } = usePagination()
 */
export function usePagination<T extends Record<string, any> = Record<string, any>>(defaultParams: T = {} as T) {
  const queryParams = reactive({
    pageNum: 1,
    pageSize: 10,
    ...defaultParams,
  }) as { pageNum: number; pageSize: number } & T;
  const dateRange = ref<[string, string] | []>([]);
  const total = ref(0);

  /** 搜索：重置到第一页 */
  function handleQuery() {
    queryParams.pageNum = 1;
  }

  /** 重置查询条件 */
  function resetQuery() {
    dateRange.value = [];
    Object.keys(defaultParams).forEach((key) => {
      (queryParams as any)[key] = defaultParams[key];
    });
    queryParams.pageNum = 1;
    queryParams.pageSize = 10;
  }

  return { queryParams, dateRange, total, handleQuery, resetQuery };
}
