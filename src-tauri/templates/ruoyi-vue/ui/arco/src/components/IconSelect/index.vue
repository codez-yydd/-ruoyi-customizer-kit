<template>
  <a-popover
    v-model:popup-visible="popoverVisible"
    trigger="click"
    position="bl"
    content-class="icon-select__popover"
    :content-style="{ padding: '12px' }"
  >
    <a-input
      :model-value="modelValue ?? ''"
      readonly
      :placeholder="t('components.selectMenuIcon')"
      allow-clear
      style="width: 200px"
      @clear="onClear"
    >
      <template #prefix>
        <AppIcon :name="modelValue" />
      </template>
      <template #suffix>
        <IconSearch />
      </template>
    </a-input>
    <template #content>
      <a-input
        v-model="keyword"
        :placeholder="t('components.searchIconPlaceholder')"
        allow-clear
        size="small"
      >
        <template #prefix><IconSearch /></template>
      </a-input>
      <div class="icon-select__grid">
        <a-tooltip v-for="item in filteredIcons" :key="item.name" :content="item.label">
          <div
            class="icon-select__item"
            :class="{ 'icon-select__item--active': item.name === modelValue }"
            @click="onPick(item.name)"
          >
            <AppIcon :name="item.name" />
          </div>
        </a-tooltip>
        <a-empty v-if="filteredIcons.length === 0" :style="{ padding: '24px 0' }" />
      </div>
    </template>
  </a-popover>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { IconSearch } from '@arco-design/web-vue/es/icon'
import AppIcon from '@/components/AppIcon/index.vue'
import { APP_ICON_NAMES } from '@/components/AppIcon/icons'

/**
 * 菜单图标选择器（若依交互：弹出图标网格 + 搜索过滤，点选回填）：
 * - 选项来自 AppIcon 共享映射表（若依短名 + Arco 图标名）
 * - v-model 绑定图标名（可为空，空表示不设置图标）
 */
defineProps<{
  /** 当前图标名（AppIcon 可渲染的短名/Arco 图标名） */
  modelValue?: string | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | undefined): void
}>()

const { t } = useI18n()

const popoverVisible = ref(false)
const keyword = ref('')

/** 可选项（label 用于搜索与悬浮提示：短名即提示文本） */
const iconOptions = computed(() =>
  APP_ICON_NAMES.map((name) => ({ name, label: name }))
)

/** 按关键字过滤（不区分大小写的包含匹配） */
const filteredIcons = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return iconOptions.value
  return iconOptions.value.filter((item) => item.label.toLowerCase().includes(kw))
})

function onPick(name: string): void {
  emit('update:modelValue', name)
  popoverVisible.value = false
}

function onClear(): void {
  emit('update:modelValue', undefined)
}
</script>

<style scoped>
.icon-select__grid {
  display: grid;
  grid-template-columns: repeat(8, 32px);
  gap: 4px;
  max-height: 240px;
  margin-top: 8px;
  padding-right: 4px;
  overflow-y: auto;
}

.icon-select__item {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  font-size: 16px;
  color: var(--color-text-1);
  cursor: pointer;
  border: 1px solid transparent;
  border-radius: 4px;
  transition: all 0.15s;
}

.icon-select__item:hover {
  background-color: var(--color-fill-2);
}

.icon-select__item--active {
  color: rgb(var(--primary-6));
  background-color: var(--color-primary-light-1);
  border-color: rgb(var(--primary-6));
}
</style>
