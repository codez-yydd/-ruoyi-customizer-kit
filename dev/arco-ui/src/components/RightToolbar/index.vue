<template>
  <div class="right-toolbar">
    <a-tooltip v-if="showSearch !== undefined" :content="showSearch ? t('components.hideSearch') : t('components.showSearch')">
      <a-button type="text" size="small" @click="toggleSearch">
        <template #icon><IconSearch /></template>
      </a-button>
    </a-tooltip>

    <a-tooltip :content="t('common.refresh')">
      <a-button type="text" size="small" @click="emit('refresh')">
        <template #icon><IconRefresh /></template>
      </a-button>
    </a-tooltip>

    <a-popover v-if="columns" trigger="click" position="br" content-class="right-toolbar-popover">
      <a-tooltip :content="t('components.columnSettings')">
        <a-button type="text" size="small">
          <template #icon><IconSettings /></template>
        </a-button>
      </a-tooltip>
      <template #content>
        <a-checkbox-group
          class="right-toolbar__columns"
          :model-value="checkedKeys"
          direction="vertical"
          @change="onColumnsChange"
        >
          <a-checkbox v-for="col in columns" :key="col.key" :value="col.key">
            {{ col.label }}
          </a-checkbox>
        </a-checkbox-group>
        <div class="right-toolbar__reset">
          <a-link :disabled="checkedKeys.length === (columns ?? []).length" @click="onResetColumns">
            {{ t('common.reset') }}
          </a-link>
        </div>
      </template>
    </a-popover>
  </div>
</template>

<script lang="ts">
/** 列设置项（与表格列对应，visible 控制显隐） */
export interface ToolbarColumn {
  /** 列字段名（与表格 data-index 对应） */
  key: string
  /** 列标题 */
  label: string
  /** 是否显示 */
  visible: boolean
}
</script>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { IconRefresh, IconSearch, IconSettings } from '@arco-design/web-vue/es/icon'

/**
 * 表格右上工具栏：搜索显隐（传入 showSearch 才显示切换按钮）、刷新、列设置（传入 columns 才显示）。
 */
const { t } = useI18n()

const props = defineProps<{
  /** 搜索区显隐（v-model:show-search） */
  showSearch?: boolean
  /** 列设置（v-model:columns） */
  columns?: ToolbarColumn[]
}>()

const emit = defineEmits<{
  (e: 'update:showSearch', value: boolean): void
  (e: 'update:columns', value: ToolbarColumn[]): void
  (e: 'refresh'): void
}>()

function toggleSearch(): void {
  emit('update:showSearch', !props.showSearch)
}

/** 当前勾选项 = visible 的列 key */
const checkedKeys = computed<string[]>(() =>
  (props.columns ?? []).filter((col) => col.visible).map((col) => col.key)
)

function onColumnsChange(keys: Array<string | number | boolean>): void {
  emit(
    'update:columns',
    (props.columns ?? []).map((col) => ({ ...col, visible: keys.includes(col.key) }))
  )
}

/** 重置列显隐：恢复全部列可见 */
function onResetColumns(): void {
  emit(
    'update:columns',
    (props.columns ?? []).map((col) => ({ ...col, visible: true }))
  )
}
</script>

<style scoped>
.right-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
}

.right-toolbar__columns {
  display: flex;
  max-height: 260px;
  overflow-y: auto;
}

.right-toolbar__reset {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--color-border);
  text-align: center;
}
</style>
