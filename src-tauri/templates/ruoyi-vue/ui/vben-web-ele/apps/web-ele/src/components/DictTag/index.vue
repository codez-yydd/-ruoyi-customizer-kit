<script setup lang="ts">
import type { DictData } from '#/api/system/dict';

import { computed } from 'vue';

import { ElTag } from 'element-plus';

/**
 * DictTag：根据字典值渲染标签（移植自若依，改用 Element Plus + Vue3）
 *
 * 用法：<DictTag :options="sys_user_sex" :value="row.sex" />
 * - options：该字典类型的 DictData[]（由 useDict 获取）
 * - value：当前数据的字典值（dictValue）
 */
const props = defineProps<{
  options?: DictData[];
  value?: string | number | string[] | number[];
}>();

const tagInfo = computed(() => {
  if (props.value === undefined || props.value === null || props.value === '') {
    return { label: '', type: 'info' as const, show: false };
  }
  // 支持单个值或多值（逗号分隔字符串/数组）
  const values = Array.isArray(props.value)
    ? props.value.map(String)
    : String(props.value).split(',');

  // 若依 listClass → el-tag type 映射
  const map = (item?: DictData) => {
    if (!item) return null;
    const typeMap: Record<string, 'primary' | 'success' | 'warning' | 'danger' | 'info' | ''> = {
      default: '',
      primary: 'primary',
      success: 'success',
      info: 'info',
      warning: 'warning',
      danger: 'danger',
    };
    return {
      label: item.dictLabel,
      type: typeMap[item.listClass ?? ''] ?? 'info',
    };
  };

  const tags = values
    .map((v) => map(props.options?.find((o) => o.dictValue === v)))
    .filter(Boolean) as { label: string; type: string }[];

  return { tags, show: tags.length > 0 };
});
</script>

<template>
  <template v-if="tagInfo.show">
    <ElTag
      v-for="(tag, i) in (tagInfo as any).tags"
      :key="i"
      :type="(tag.type as any) || undefined"
      size="small"
      style="margin-right: 4px"
    >
      {{ tag.label }}
    </ElTag>
  </template>
  <span v-else class="muted">-</span>
</template>
