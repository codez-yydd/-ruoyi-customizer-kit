<template>
  <a-pagination
    class="pagination"
    :current="page"
    :page-size="limit"
    :total="total"
    :page-size-options="pageSizes"
    size="medium"
    show-total
    show-page-size
    show-jumper
    @change="onPageChange"
    @page-size-change="onPageSizeChange"
  />
</template>

<script setup lang="ts">
import { onBeforeUnmount } from 'vue'

/**
 * 分页封装：page/pageSize 与后端 pageNum/pageSize 一一对应；
 * 页码或页大小变化都触发 change（同刻多次事件合并为一次，页大小变化时页码重置 1）。
 */
const props = withDefaults(
  defineProps<{
    /** 当前页（v-model:page） */
    page: number
    /** 每页条数（v-model:limit） */
    limit: number
    /** 总条数 */
    total: number
    /** 页大小可选项 */
    pageSizes?: number[]
  }>(),
  { pageSizes: () => [10, 20, 50, 100] }
)

const emit = defineEmits<{
  (e: 'update:page', value: number): void
  (e: 'update:limit', value: number): void
  (e: 'change'): void
}>()

/**
 * 同一刻的 change + pageSizeChange 合并为一次 change，
 * 避免 Arco 在页大小变化联动页码时重复触发父级查询
 */
let changeTimer: ReturnType<typeof setTimeout> | null = null

function fireChange(): void {
  if (changeTimer) clearTimeout(changeTimer)
  changeTimer = setTimeout(() => {
    changeTimer = null
    emit('change')
  }, 0)
}

function onPageChange(current: number): void {
  if (current !== props.page) emit('update:page', current)
  fireChange()
}

function onPageSizeChange(size: number): void {
  if (size !== props.limit) emit('update:limit', size)
  if (props.page !== 1) emit('update:page', 1)
  fireChange()
}

onBeforeUnmount(() => {
  if (changeTimer) clearTimeout(changeTimer)
})
</script>
