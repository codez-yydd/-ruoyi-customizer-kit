<template>
  <div class="crud-table">
    <!-- 搜索区：显隐由 RightToolbar 切换 -->
    <div v-if="hasSearchSlot" v-show="searchVisible" class="crud-table__search">
      <slot name="search" />
    </div>

    <!-- 工具栏：左侧按钮区 + 右侧 RightToolbar -->
    <div class="crud-table__toolbar">
      <div class="crud-table__toolbar-main">
        <slot name="toolbar" />
      </div>
      <RightToolbar
        :show-search="hasSearchSlot ? searchVisible : undefined"
        :columns="hasTableSlot ? undefined : toolbarColumns"
        @update:show-search="onSearchVisibleChange"
        @update:columns="onToolbarColumnsChange"
        @refresh="emit('query')"
      />
    </div>

    <!-- 表格：默认内置 a-table（columns 驱动）；#table 插槽可完全接管 -->
    <slot v-if="hasTableSlot" name="table" />
    <a-table
      v-else
      class="crud-table__table"
      :data="data"
      :loading="loading"
      :pagination="false"
      :row-key="rowKey"
      :row-selection="rowSelection"
      :scroll="{ x: '100%' }"
      @selection-change="onSelectionChange"
    >
      <template #columns>
        <a-table-column v-if="showIndex" :width="64" align="center">
          <template #cell="{ rowIndex }">{{ (page - 1) * limit + rowIndex + 1 }}</template>
        </a-table-column>
        <a-table-column
          v-for="col in visibleColumns"
          :key="col.key"
          :title="col.label"
          :data-index="col.key"
          :width="col.width"
          :min-width="col.minWidth"
          :align="col.align"
          :ellipsis="col.ellipsis"
          :tooltip="col.tooltip"
          :fixed="col.fixed"
        >
          <template v-if="$slots[`cell-${col.key}`]" #cell="cellScope">
            <slot :name="`cell-${col.key}`" v-bind="cellScope" />
          </template>
        </a-table-column>
      </template>
    </a-table>

    <!-- 分页 -->
    <div class="crud-table__footer">
      <Pagination
        :page="page"
        :limit="limit"
        :total="total"
        @update:page="onPageChange"
        @update:limit="onLimitChange"
        @change="emit('query')"
      />
    </div>
  </div>
</template>

<script lang="ts">
/** 内置表格列定义 */
export interface CrudColumn {
  /** 字段名（与表格 data-index 对应，同时用于列设置与单元格插槽名 cell-{key}） */
  key: string
  /** 列标题 */
  label: string
  width?: number
  minWidth?: number
  align?: 'left' | 'center' | 'right'
  /** 超出省略 */
  ellipsis?: boolean
  /** 省略时悬浮提示完整内容 */
  tooltip?: boolean
  /** 固定列（横向滚动时生效；操作列由组件内部统一处理，页面无需声明） */
  fixed?: 'left' | 'right'
}
</script>

<script setup lang="ts" generic="T extends TableData">
import { computed, ref, useSlots, watch } from 'vue'
import type { TableData, TableRowSelection } from '@arco-design/web-vue'
import Pagination from '@/components/Pagination/index.vue'
import RightToolbar from '@/components/RightToolbar/index.vue'
import type { ToolbarColumn } from '@/components/RightToolbar/index.vue'

/**
 * 通用 CRUD 页脚手架：
 * - 扁平化布局：搜索区（#search）+ 工具栏（#toolbar + RightToolbar）+ 表格 + 分页
 * - 内置 a-table 由 columns 驱动：自动带序号列（(page-1)*limit+rowIndex+1）、多选列、
 *   自定义单元格插槽 cell-{key}；列显隐由 RightToolbar 维护，父级无需关心
 * - #table 插槽存在时完全接管表格区（父级自写 a-table，行选择自行配置），
 *   此时列设置按钮自动隐藏，分页/刷新/搜索显隐仍由本组件托管
 * - 页码/页大小变化与工具栏刷新统一 emit('query')，父级绑定 getList 即可
 * - 泛型 T 为行类型（type alias，满足 TableData 索引签名），与 useCrud 行类型配套
 */
const props = withDefaults(
  defineProps<{
    /** 表格数据（useCrud 的 list） */
    data?: T[]
    /** loading（useCrud 的 loading） */
    loading?: boolean
    /** 列定义（内置表格模式必传） */
    columns?: CrudColumn[]
    /** 是否显示多选列（内置表格模式） */
    selectable?: boolean
    /** 行 key 字段 */
    rowKey?: string
    /** 是否显示序号列（内置表格模式） */
    showIndex?: boolean
    /** 搜索区显隐（v-model:show-search，不传则由组件内部维护） */
    showSearch?: boolean
    /** 当前页（v-model:page） */
    page: number
    /** 每页条数（v-model:limit） */
    limit: number
    /** 总条数 */
    total: number
  }>(),
  {
    data: () => [],
    loading: false,
    selectable: false,
    rowKey: 'id',
    showIndex: true
  }
)

const emit = defineEmits<{
  (e: 'update:page', value: number): void
  (e: 'update:limit', value: number): void
  (e: 'update:showSearch', value: boolean): void
  (e: 'query'): void
  (e: 'selection-change', rows: T[]): void
}>()

const slots = useSlots()

const hasSearchSlot = computed(() => !!slots.search)
const hasTableSlot = computed(() => !!slots.table)

/* ---------- 搜索区显隐 ---------- */
const innerShowSearch = ref(true)

function onSearchVisibleChange(value: boolean): void {
  innerShowSearch.value = value
  if (props.showSearch !== undefined) emit('update:showSearch', value)
}

const searchVisible = computed<boolean>(() => props.showSearch ?? innerShowSearch.value)

/* ---------- 列设置（内置表格模式） ---------- */
/** 隐藏列 key 集合（列显隐状态由组件内部维护，父级零负担） */
const hiddenKeys = ref<Set<string>>(new Set())

const toolbarColumns = computed<ToolbarColumn[]>(() =>
  (props.columns ?? []).map((col) => ({
    key: col.key,
    label: col.label,
    visible: !hiddenKeys.value.has(col.key)
  }))
)

const visibleColumns = computed<CrudColumn[]>(() => {
  const columns = (props.columns ?? []).filter((col) => !hiddenKeys.value.has(col.key))
  // 操作列统一固定右侧：横向滚动时保持操作按钮可见，页面无需单独声明
  return columns.map((col) =>
    col.key === 'operation' ? { ...col, fixed: 'right' as const } : col
  )
})

function onToolbarColumnsChange(columns: ToolbarColumn[]): void {
  hiddenKeys.value = new Set(columns.filter((col) => !col.visible).map((col) => col.key))
}

/* ---------- 行选择（内置表格模式） ---------- */
const selectedKeys = ref<Array<string | number>>([])

const rowSelection = computed<TableRowSelection | undefined>(() => {
  if (!props.selectable) return undefined
  return {
    type: 'checkbox',
    showCheckedAll: true,
    selectedRowKeys: selectedKeys.value,
    width: 44
  }
})

function onSelectionChange(keys: Array<string | number>): void {
  selectedKeys.value = keys
  const rowKey = props.rowKey
  const rows = (props.data ?? []).filter((row) =>
    keys.some((key) => String(key) === String(row[rowKey]))
  )
  emit('selection-change', rows)
}

/** 数据刷新后清空勾选，避免残留脏选中 */
watch(
  () => props.data,
  () => {
    if (selectedKeys.value.length > 0) {
      selectedKeys.value = []
      emit('selection-change', [])
    }
  }
)

/* ---------- 分页透传 ---------- */
function onPageChange(value: number): void {
  emit('update:page', value)
}

function onLimitChange(value: number): void {
  emit('update:limit', value)
}
</script>

<style scoped>
/* 白卡容器：浮于浅灰内容底之上（与全局 a-card 覆写同观感；暗色主题变量自动翻转） */
.crud-table {
  background-color: var(--color-bg-2);
  border-radius: 8px;
  padding: 16px;
}

.crud-table__search {
  margin-bottom: 16px;
}

.crud-table__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
}

.crud-table__toolbar-main {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.crud-table__table {
  width: 100%;
}

.crud-table__footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}
</style>
