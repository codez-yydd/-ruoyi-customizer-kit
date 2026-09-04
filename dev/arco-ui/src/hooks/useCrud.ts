import { computed, nextTick, reactive, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'
import type { FieldRule, FormInstance } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import { useI18n } from 'vue-i18n'
import type { PageQuery, PageResult } from '@/api/types'
import { exportRequest } from '@/utils/download'

/** CRUD 可编辑行（表单/列表行数据） */
export type CrudRecord = Record<string, unknown>

/** 弹窗状态（form 为 reactive 属性，整体替换时保持响应式） */
export interface CrudModal<T extends CrudRecord> {
  title: string
  open: boolean
  form: Partial<T>
  rules?: Record<string, FieldRule[]>
}

/** useCrud 配置 */
export interface UseCrudConfig<T extends CrudRecord, Q extends PageQuery> {
  /** 分页列表接口（返回 {total, rows}） */
  listApi: (query: Q) => Promise<PageResult<T>>
  /** 查询参数初始值（resetQuery 恢复基准），默认空对象 */
  initQuery?: Q
  /** 新增接口（不传则新增提交不生效） */
  addApi?: (data: Partial<T>) => Promise<unknown>
  /** 修改接口（不传则修改提交不生效） */
  updateApi?: (data: Partial<T>) => Promise<unknown>
  /** 删除接口（支持单条 id 或 id 数组） */
  deleteApi?: (ids: string | number | Array<string | number>) => Promise<unknown>
  /** 导出接口地址（如 /system/user/export），不传则不提供导出能力 */
  exportUrl?: string
  /** 导出文件兜底名（优先使用后端 Content-Disposition） */
  exportName?: string
  /** 主键字段名（区分新增/修改、批量删除取 id），默认 'id' */
  pkField?: string
  /** 打开新增弹窗时的表单初始值工厂 */
  formFactory?: () => Partial<T>
  /** 弹窗表单校验规则 */
  rules?: Record<string, FieldRule[]>
}

/** useCrud 返回值 */
export interface UseCrudReturn<T extends CrudRecord, Q extends PageQuery> {
  /** 列表 loading */
  loading: Ref<boolean>
  /** 导出 loading（与列表 loading 分离，避免按钮互锁） */
  exportLoading: Ref<boolean>
  /** 当前行数据 */
  list: Ref<T[]>
  /** 总条数 */
  total: Ref<number>
  /** 查询参数（reactive，直接 v-model 绑定到搜索表单） */
  queryParams: Q
  /** 当前页码（从 1 开始，与后端 pageNum 对应） */
  page: Ref<number>
  /** 每页条数（与后端 pageSize 对应） */
  limit: Ref<number>
  /** 按当前 page/limit/queryParams 查询 */
  getList: () => Promise<void>
  /** 查询（重置页码到第 1 页） */
  handleQuery: () => void
  /** 重置查询参数为初始值并重新查询 */
  resetQuery: () => void
  /** 已选中行（由表格通过 setSelection 回写） */
  selection: Ref<T[]>
  /** 表格选中行回写入口（配合 CrudTable 的 @selection-change） */
  setSelection: (rows: T[]) => void
  /** 选中行主键集合 */
  ids: ComputedRef<Array<string | number>>
  /** 仅选中一条时为 false（控制"修改"按钮禁用） */
  single: ComputedRef<boolean>
  /** 未选中任何行时为 true（控制"删除"按钮禁用） */
  multiple: ComputedRef<boolean>
  /** 弹窗状态 */
  modal: CrudModal<T>
  /** 弹窗表单 ref（模板中绑定 <a-form ref="formRef">） */
  formRef: Ref<FormInstance | undefined>
  /** 打开新增弹窗 */
  handleAdd: () => void
  /** 打开修改弹窗（浅拷贝行数据进表单） */
  handleUpdate: (row: Partial<T>) => void
  /** 删除（传单条 id 或 id 数组；不传则用当前勾选 ids；单条可传 name 用于确认文案），带确认框 */
  handleDelete: (idOrIds?: string | number | Array<string | number>, name?: string) => void
  /** 导出（exportUrl + 当前查询条件） */
  handleExport: () => Promise<void>
  /** 弹窗提交：校验 -> 新增/修改 -> 关弹窗 -> 刷新列表 */
  submitForm: () => Promise<void>
  /** 关闭弹窗 */
  cancel: () => void
}

/**
 * 通用 CRUD 组合式函数：
 * 只负责 page/limit/请求/删除/导出/弹窗状态，查询字段结构与表格渲染由调用方定义。
 */
export function useCrud<T extends CrudRecord, Q extends PageQuery>(
  config: UseCrudConfig<T, Q>
): UseCrudReturn<T, Q> {
  // 组合式函数在页面 setup 上下文中调用，useI18n 可安全获取当前语言的 t
  const { t } = useI18n()
  const pkField = config.pkField ?? 'id'
  const initQuery = { ...config.initQuery } as Q

  const loading = ref(false)
  const exportLoading = ref(false)
  const list = ref([]) as Ref<T[]>
  const total = ref(0)
  const page = ref(1)
  const limit = ref(10)
  // 泛型容器经 reactive/ref 解包后与原类型不可证等价，在创建处统一断言
  const queryParams = reactive({ ...initQuery }) as Q
  const selection = ref([]) as Ref<T[]>
  const formRef = ref<FormInstance>()

  const modal = reactive({
    title: '',
    open: false,
    form: {} as Partial<T>,
    rules: config.rules
  }) as CrudModal<T>

  const ids = computed<Array<string | number>>(() =>
    selection.value
      .map((row) => row[pkField])
      .filter((value): value is string | number => typeof value === 'string' || typeof value === 'number')
  )
  const single = computed(() => selection.value.length !== 1)
  const multiple = computed(() => selection.value.length === 0)

  /** 按当前条件查询列表 */
  async function getList(): Promise<void> {
    loading.value = true
    try {
      const result = await config.listApi({ ...queryParams, pageNum: page.value, pageSize: limit.value })
      list.value = result.rows ?? []
      total.value = result.total ?? 0
    } finally {
      loading.value = false
    }
  }

  /** 查询（重置页码） */
  function handleQuery(): void {
    page.value = 1
    void getList()
  }

  /** 重置查询参数并查询 */
  function resetQuery(): void {
    // 先清空运行期键再恢复初始值，避免遗留
    const target = queryParams as Record<string, unknown>
    for (const key of Object.keys(target)) {
      delete target[key]
    }
    Object.assign(queryParams, initQuery)
    page.value = 1
    void getList()
  }

  /** 表格选中行回写 */
  function setSelection(rows: T[]): void {
    selection.value = rows
  }

  /** 打开弹窗（nextTick 后清空上次的校验痕迹） */
  function openModal(title: string, form: Partial<T>): void {
    modal.title = title
    modal.form = form
    modal.open = true
    void nextTick(() => formRef.value?.clearValidate())
  }

  /** 打开新增弹窗 */
  function handleAdd(): void {
    openModal(t('common.add'), config.formFactory ? config.formFactory() : {})
  }

  /** 打开修改弹窗 */
  function handleUpdate(row: Partial<T>): void {
    openModal(t('common.edit'), { ...row })
  }

  /** 删除（带确认框；当前页删空且非首页时回退一页） */
  function handleDelete(idOrIds?: string | number | Array<string | number>, name?: string): void {
    if (!config.deleteApi) return
    const target = idOrIds ?? ids.value
    const isEmpty = Array.isArray(target) ? target.length === 0 : target == null || target === ''
    if (isEmpty) {
      Message.warning(t('common.pleaseSelectDelete'))
      return
    }
    // 单条删除且传了业务名称时，文案带上名称便于确认；其余保持编号/批量文案
    const content =
      !Array.isArray(target) && name
        ? t('common.confirmDeleteName', { name })
        : Array.isArray(target)
          ? t('common.confirmDeleteSelected', { count: target.length })
          : t('common.confirmDeleteId', { id: String(target) })
    Modal.confirm({
      title: t('common.deleteConfirm'),
      content,
      hideCancel: false,
      onOk: async () => {
        try {
          await config.deleteApi?.(target)
          Message.success(t('common.deleteSuccess'))
          selection.value = []
          if (list.value.length === 1 && page.value > 1) {
            page.value -= 1
          }
          await getList()
        } catch {
          // 错误提示已由响应拦截器统一弹出
        }
      }
    })
  }

  /** 导出（body 为当前查询条件） */
  async function handleExport(): Promise<void> {
    if (!config.exportUrl || exportLoading.value) return
    exportLoading.value = true
    try {
      await exportRequest(
        config.exportUrl,
        { ...queryParams, pageNum: page.value, pageSize: limit.value },
        config.exportName ?? 'export.xlsx'
      )
    } catch {
      // 导出失败（含后端返回 JSON 错误）已由 download.ts/拦截器提示
    } finally {
      exportLoading.value = false
    }
  }

  /** 弹窗提交：无主键调 add，有主键调 update */
  async function submitForm(): Promise<void> {
    if (!config.addApi && !config.updateApi) return
    try {
      await formRef.value?.validate()
    } catch {
      // 校验失败：错误信息已由表单展示
      return
    }
    const data = modal.form
    // 主键存在性区分新增/修改（pkField 为可配置字段名，走通用索引读取）
    const pkValue: unknown = (data as Record<string, unknown>)[pkField]
    const isUpdate = pkValue != null && pkValue !== ''
    try {
      if (isUpdate) {
        if (!config.updateApi) return
        await config.updateApi(data)
        Message.success(t('common.updateSuccess'))
      } else {
        if (!config.addApi) return
        await config.addApi(data)
        Message.success(t('common.addSuccess'))
      }
      modal.open = false
      await getList()
    } catch {
      // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
    }
  }

  /** 关闭弹窗 */
  function cancel(): void {
    modal.open = false
  }

  return {
    loading,
    exportLoading,
    list,
    total,
    queryParams,
    page,
    limit,
    getList,
    handleQuery,
    resetQuery,
    selection,
    setSelection,
    ids,
    single,
    multiple,
    modal,
    formRef,
    handleAdd,
    handleUpdate,
    handleDelete,
    handleExport,
    submitForm,
    cancel
  }
}
