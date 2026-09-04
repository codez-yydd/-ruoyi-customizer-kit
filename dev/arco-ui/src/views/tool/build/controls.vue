<template>
  <a-input
    v-if="field.type === 'input'"
    :model-value="bindValue"
    :placeholder="field.placeholder"
    :disabled="disabled"
    allow-clear
    @update:model-value="emitValue"
  />
  <a-textarea
    v-else-if="field.type === 'textarea'"
    :model-value="bindValue"
    :placeholder="field.placeholder"
    :disabled="disabled"
    :auto-size="{ minRows: 2, maxRows: 4 }"
    @update:model-value="emitValue"
  />
  <a-select
    v-else-if="field.type === 'select'"
    :model-value="bindValue"
    :placeholder="field.placeholder"
    :disabled="disabled"
    allow-clear
    @update:model-value="emitValue"
  >
    <a-option v-for="option in field.options" :key="option" :value="option">
      {{ option }}
    </a-option>
  </a-select>
  <a-radio-group
    v-else-if="field.type === 'radio'"
    :model-value="bindValue"
    :disabled="disabled"
    @update:model-value="emitValue"
  >
    <a-radio v-for="option in field.options" :key="option" :value="option">{{ option }}</a-radio>
  </a-radio-group>
  <a-checkbox-group
    v-else-if="field.type === 'checkbox'"
    :model-value="checkboxValue"
    :disabled="disabled"
    @update:model-value="emitCheckbox"
  >
    <a-checkbox v-for="option in field.options" :key="option" :value="option">{{ option }}</a-checkbox>
  </a-checkbox-group>
  <a-switch
    v-else-if="field.type === 'switch'"
    :model-value="bindValue === 'true'"
    :disabled="disabled"
    @update:model-value="emitSwitch"
  />
  <a-date-picker
    v-else-if="field.type === 'date'"
    :model-value="bindValue"
    :placeholder="field.placeholder"
    :disabled="disabled"
    style="width: 100%"
    @update:model-value="emitValue"
  />
  <a-rate
    v-else
    :model-value="rateValue"
    :disabled="disabled"
    allow-half
    @update:model-value="emitRate"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { BuildField } from './controls'

/**
 * 表单构建控件渲染器：按 field.type 渲染对应 Arco 控件。
 * 值统一以 string 形态经 v-model 交换（checkbox 为逗号拼接、switch 为 'true'/'false'、
 * rate 为数字字符串），画布/预览共用，简化父级取值。
 */
const props = withDefaults(
  defineProps<{
    /** 表单项定义 */
    field: BuildField
    /** 当前值（string 形态） */
    modelValue?: string
    /** 画布场景禁用交互 */
    disabled?: boolean
  }>(),
  { modelValue: '', disabled: false }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const bindValue = computed<string>(() => props.modelValue ?? '')

const checkboxValue = computed<string[]>(() =>
  bindValue.value ? bindValue.value.split(',').filter((part) => part !== '') : []
)

const rateValue = computed<number>(() => {
  const value = Number(bindValue.value)
  return Number.isNaN(value) ? 0 : value
})

function emitValue(value: unknown): void {
  emit('update:modelValue', value == null ? '' : String(value))
}

function emitCheckbox(value: Array<string | number | boolean>): void {
  emit('update:modelValue', value.map(String).join(','))
}

function emitSwitch(value: string | number | boolean): void {
  emit('update:modelValue', value ? 'true' : 'false')
}

function emitRate(value: number): void {
  emit('update:modelValue', String(value ?? 0))
}
</script>
