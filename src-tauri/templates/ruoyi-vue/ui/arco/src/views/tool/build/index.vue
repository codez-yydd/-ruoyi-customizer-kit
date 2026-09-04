<template>
  <div class="build-page">
    <div class="build-page__body app-page-card">
      <!-- 左：组件面板 -->
      <div class="build-page__panel">
        <div class="build-page__panel-title">{{ t('tool.build.palette') }}</div>
        <div class="build-page__palette">
          <button
            v-for="item in palette"
            :key="item.type"
            type="button"
            class="build-page__palette-item"
            @click="addField(item.type, item.label)"
          >
            <component :is="item.icon" />
            <span>{{ item.label }}</span>
          </button>
        </div>
      </div>

      <!-- 中：画布 -->
      <div class="build-page__canvas-wrap">
        <a-space class="build-page__canvas-actions">
          <a-button type="primary" :disabled="fields.length === 0" @click="openPreview">
            <template #icon><IconEye /></template>
            {{ t('tool.build.preview') }}
          </a-button>
          <a-button :disabled="fields.length === 0" @click="handleClear">
            <template #icon><IconDelete /></template>
            {{ t('common.clean') }}
          </a-button>
          <a-button :disabled="fields.length === 0" @click="openCode">
            <template #icon><IconCode /></template>
            {{ t('tool.build.genCode') }}
          </a-button>
        </a-space>
        <a-empty
          v-if="fields.length === 0"
          :description="t('tool.build.canvasEmpty')"
          class="build-page__empty"
        />
        <div v-else class="build-page__canvas">
          <div
            v-for="(field, index) in fields"
            :key="field.field"
            class="build-page__item"
            :class="{ 'build-page__item--active': field.field === selectedField }"
            @click="selectedField = field.field"
          >
            <a-form-item :label="field.label" class="build-page__item-form">
              <BuildControl
                :field="field"
                :model-value="previewValues[field.field]"
                :disabled="true"
                @update:model-value="(value: string) => onPreviewChange(field.field, value)"
              />
            </a-form-item>
            <a-space :size="2" class="build-page__item-actions" @click.stop>
              <a-button type="text" size="mini" :disabled="index === 0" @click="moveField(index, -1)">
                <template #icon><IconUp /></template>
              </a-button>
              <a-button
                type="text"
                size="mini"
                :disabled="index === fields.length - 1"
                @click="moveField(index, 1)"
              >
                <template #icon><IconDown /></template>
              </a-button>
              <a-button type="text" size="mini" status="danger" @click="removeField(field.field)">
                <template #icon><IconDelete /></template>
              </a-button>
            </a-space>
          </div>
        </div>
      </div>

      <!-- 右：属性面板 -->
      <div class="build-page__panel">
        <div class="build-page__panel-title">{{ t('tool.build.props') }}</div>
        <a-empty v-if="!selected" :description="t('tool.build.selectFieldFirst')" />
        <a-form v-else :model="selected" layout="vertical" size="small">
          <a-form-item :label="t('tool.build.labelName')">
            <a-input
              v-model.trim="selected.label"
              :placeholder="t('tool.build.labelPlaceholder')"
              allow-clear
            />
          </a-form-item>
          <a-form-item :label="t('tool.build.fieldName')">
            <a-input
              v-model.trim="selected.field"
              :placeholder="t('tool.build.fieldPlaceholder')"
              allow-clear
              @update:model-value="(value: string) => onFieldRename(value)"
            />
          </a-form-item>
          <a-form-item :label="t('tool.build.placeholderLabel')">
            <a-input
              v-model.trim="selected.placeholder"
              :placeholder="t('tool.build.placeholderPlaceholder')"
              allow-clear
            />
          </a-form-item>
          <a-form-item :label="t('tool.build.requiredLabel')">
            <a-switch
              v-model="selected.required"
              :checked-text="t('common.yes')"
              :unchecked-text="t('common.no')"
            />
          </a-form-item>
          <a-form-item v-if="hasOptions(selected.type)" :label="t('tool.build.optionsLabel')">
            <a-textarea
              :model-value="selected.options.join('\n')"
              :placeholder="t('tool.build.optionsPlaceholder')"
              :auto-size="{ minRows: 3, maxRows: 6 }"
              @update:model-value="(value: string) => selected && onOptionsChange(selected, value)"
            />
          </a-form-item>
        </a-form>
      </div>
    </div>

    <!-- 预览弹窗 -->
    <a-modal
      :visible="preview.open"
      :title="t('tool.build.previewTitle')"
      :width="560"
      :footer="false"
      @cancel="preview.open = false"
      @close="preview.open = false"
    >
      <a-form :model="previewValues" auto-label-width @submit-success="onPreviewSubmit">
        <a-form-item
          v-for="field in fields"
          :key="field.field"
          :field="field.field"
          :label="field.label"
          :rules="field.required ? [{ required: true, message: t('tool.build.fieldRequired', { field: field.label }) }] : []"
        >
          <BuildControl
            :field="field"
            :model-value="previewValues[field.field]"
            @update:model-value="(value: string) => onPreviewChange(field.field, value)"
          />
        </a-form-item>
        <a-form-item>
          <a-space>
            <a-button type="primary" html-type="submit">{{ t('common.submit') }}</a-button>
            <a-button @click="resetPreviewValues">{{ t('common.reset') }}</a-button>
          </a-space>
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 生成代码弹窗 -->
    <a-modal
      :visible="code.open"
      :title="t('tool.build.codeTitle')"
      :width="720"
      :footer="false"
      @cancel="code.open = false"
      @close="code.open = false"
    >
      <div class="build-page__code-actions">
        <a-button type="primary" size="small" @click="copyCode">
          <template #icon><IconCopy /></template>
          {{ t('common.copyCode') }}
        </a-button>
      </div>
      <pre class="build-page__code">{{ code.content }}</pre>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { Component } from 'vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconCalendar,
  IconCheckCircle,
  IconCheckSquare,
  IconCode,
  IconCopy,
  IconDelete,
  IconDown,
  IconEdit,
  IconEye,
  IconList,
  IconMenu,
  IconNav,
  IconStar
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import type { BuildField, FieldType } from './controls'
import { hasOptions } from './controls'
import BuildControl from './controls.vue'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Build' })

const { t } = useI18n()

/** 组件面板项 */
interface PaletteItem {
  type: FieldType
  label: string
  icon: Component
}

/** 组件面板（computed：随语言切换联动面板项文案） */
const palette = computed<PaletteItem[]>(() => [
  { type: 'input', label: t('tool.build.ctlInput'), icon: IconEdit },
  { type: 'textarea', label: t('tool.build.ctlTextarea'), icon: IconMenu },
  { type: 'select', label: t('tool.build.ctlSelect'), icon: IconList },
  { type: 'radio', label: t('tool.build.ctlRadio'), icon: IconCheckCircle },
  { type: 'checkbox', label: t('tool.build.ctlCheckbox'), icon: IconCheckSquare },
  { type: 'switch', label: t('tool.build.ctlSwitch'), icon: IconNav },
  { type: 'date', label: t('tool.build.ctlDate'), icon: IconCalendar },
  { type: 'rate', label: t('tool.build.ctlRate'), icon: IconStar }
])

/** 自增字段名序号（保证画布内字段名唯一） */
let fieldSeq = 0

const fields = ref<BuildField[]>([])
const selectedField = ref('')

const selected = computed<BuildField | null>(
  () => fields.value.find((item) => item.field === selectedField.value) ?? null
)

/** 新增表单项：field 生成规则 field{序号}，可后续在属性面板改标签 */
function addField(type: FieldType, label: string): void {
  fieldSeq += 1
  const field = `field${fieldSeq}`
  fields.value.push({
    field,
    label: `${label}${fieldSeq}`,
    type,
    placeholder: t('common.pleaseEnter', { field: `${label}${fieldSeq}` }),
    required: false,
    options: hasOptions(type)
      ? [
          t('tool.build.defaultOption', { index: 1 }),
          t('tool.build.defaultOption', { index: 2 }),
          t('tool.build.defaultOption', { index: 3 })
        ]
      : []
  })
  selectedField.value = field
  syncPreviewKeys()
}

function removeField(field: string): void {
  fields.value = fields.value.filter((item) => item.field !== field)
  if (selectedField.value === field) selectedField.value = ''
  syncPreviewKeys()
}

function moveField(index: number, offset: -1 | 1): void {
  const target = index + offset
  if (target < 0 || target >= fields.value.length) return
  const next = [...fields.value]
  const [moved] = next.splice(index, 1)
  if (moved) next.splice(target, 0, moved)
  fields.value = next
}

function handleClear(): void {
  Modal.confirm({
    title: t('common.cleanConfirm'),
    content: t('tool.build.clearCanvasConfirm'),
    hideCancel: false,
    onOk: () => {
      fields.value = []
      selectedField.value = ''
      previewValues.value = {}
    }
  })
}

/** 选项文本（每行一个）回写 */
function onOptionsChange(target: BuildField, value: string): void {
  target.options = value
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
}

/** 字段名重命名时同步画布选中态（保持唯一性：重名自动追加序号） */
function onFieldRename(value: string): void {
  const target = selected.value
  if (!target) return
  const renamed = value.trim() || target.field
  if (renamed !== target.field && fields.value.some((item) => item.field === renamed)) {
    let seq = 2
    let candidate = `${renamed}${seq}`
    while (fields.value.some((item) => item.field === candidate)) {
      seq += 1
      candidate = `${renamed}${seq}`
    }
    target.field = candidate
    Message.warning(t('tool.build.fieldNameDup', { field: candidate }))
  }
  selectedField.value = target.field
  syncPreviewKeys()
}

/* ---------- 预览 ---------- */
const preview = reactive({ open: false })

const previewValues = ref<Record<string, string>>({})

function syncPreviewKeys(): void {
  const next: Record<string, string> = {}
  for (const field of fields.value) {
    next[field.field] = previewValues.value[field.field] ?? ''
  }
  previewValues.value = next
}

function onPreviewChange(field: string, value: string): void {
  previewValues.value = { ...previewValues.value, [field]: value }
}

function resetPreviewValues(): void {
  previewValues.value = {}
}

function openPreview(): void {
  resetPreviewValues()
  preview.open = true
}

function onPreviewSubmit(): void {
  Message.success(t('tool.build.previewSubmitSuccess'))
}

/* ---------- 生成代码 ---------- */
const code = reactive({ open: false, content: '' })

/** 依据画布字段生成 Arco 表单的 Vue 片段 */
function generateCode(): string {
  const lines: string[] = ['<template>', '  <a-form :model="form" auto-label-width>']
  for (const field of fields.value) {
    const rules = field.required
      ? ' :rules="[{ required: true, message: \'' +
        t('tool.build.fieldRequired', { field: field.label }) +
        '\' }]"'
      : ''
    lines.push(`    <a-form-item label="${field.label}" field="${field.field}"${rules}>`)
    switch (field.type) {
      case 'input':
        lines.push(`      <a-input v-model="form.${field.field}" placeholder="${field.placeholder}" allow-clear />`)
        break
      case 'textarea':
        lines.push(`      <a-textarea v-model="form.${field.field}" placeholder="${field.placeholder}" allow-clear />`)
        break
      case 'select':
        lines.push(`      <a-select v-model="form.${field.field}" placeholder="${field.placeholder}" allow-clear>`)
        for (const option of field.options) {
          lines.push(`        <a-option value="${option}">${option}</a-option>`)
        }
        lines.push('      </a-select>')
        break
      case 'radio':
        lines.push('      <a-radio-group v-model="form.' + field.field + '">')
        for (const option of field.options) {
          lines.push(`        <a-radio value="${option}">${option}</a-radio>`)
        }
        lines.push('      </a-radio-group>')
        break
      case 'checkbox':
        lines.push('      <a-checkbox-group v-model="form.' + field.field + '">')
        for (const option of field.options) {
          lines.push(`        <a-checkbox value="${option}">${option}</a-checkbox>`)
        }
        lines.push('      </a-checkbox-group>')
        break
      case 'switch':
        lines.push(`      <a-switch v-model="form.${field.field}" />`)
        break
      case 'date':
        lines.push(`      <a-date-picker v-model="form.${field.field}" style="width: 100%" />`)
        break
      case 'rate':
        lines.push(`      <a-rate v-model="form.${field.field}" allow-half />`)
        break
    }
    lines.push('    </a-form-item>')
  }
    lines.push('    <a-form-item>')
    lines.push('      <a-space>')
    lines.push(`        <a-button type="primary" @click="onSubmit">${t('common.submit')}</a-button>`)
    lines.push(`        <a-button @click="onReset">${t('common.reset')}</a-button>`)
    lines.push('      </a-space>')
    lines.push('    </a-form-item>')
  lines.push('  </a-form>')
  lines.push('</template>')
  lines.push('')
  lines.push('<script setup lang="ts">')
  lines.push("import { reactive } from 'vue'")
  lines.push('')
  lines.push('const form = reactive({')
  for (const field of fields.value) {
    const initial =
      field.type === 'checkbox' ? '[]' : field.type === 'switch' ? 'false' : field.type === 'rate' ? '0' : "''"
    lines.push(`  ${field.field}: ${initial},`)
  }
  lines.push('})')
  lines.push('')
  lines.push('function onSubmit(): void {')
  lines.push('  console.log(\'submit\', form)')
  lines.push('}')
  lines.push('')
  lines.push('function onReset(): void {')
  lines.push('  Object.assign(form, {')
  for (const field of fields.value) {
    const initial = field.type === 'checkbox' ? '[]' : field.type === 'switch' ? 'false' : field.type === 'rate' ? '0' : "''"
    lines.push(`    ${field.field}: ${initial},`)
  }
  lines.push('  })')
  lines.push('}')
  lines.push('<\/script>')
  return lines.join('\n')
}

function openCode(): void {
  code.content = generateCode()
  code.open = true
}

async function copyCode(): Promise<void> {
  try {
    await navigator.clipboard.writeText(code.content)
    Message.success(t('common.codeCopied'))
  } catch {
    Message.warning(t('common.copyUnsupported'))
  }
}
</script>

<style scoped>
.build-page {
  height: 100%;
}

.build-page__body {
  display: flex;
  gap: 16px;
  align-items: stretch;
  min-height: 480px;
}

.build-page__panel {
  width: 240px;
  flex-shrink: 0;
}

.build-page__panel-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-1);
  margin-bottom: 12px;
}

.build-page__palette {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.build-page__palette-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 4px;
  border: 1px dashed var(--color-border-2);
  border-radius: 4px;
  background-color: var(--color-fill-1);
  cursor: pointer;
  font-size: 12px;
  color: var(--color-text-2);
  transition: all 0.2s;
}

.build-page__palette-item:hover {
  border-color: rgb(var(--arcoblue-6));
  color: rgb(var(--arcoblue-6));
}

.build-page__canvas-wrap {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.build-page__canvas-actions {
  margin-bottom: 12px;
}

.build-page__empty {
  margin-top: 120px;
}

.build-page__canvas {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.build-page__item {
  position: relative;
  padding: 12px 44px 2px 12px;
  border: 1px dashed var(--color-border-2);
  border-radius: 4px;
  cursor: pointer;
  transition: border-color 0.2s;
}

.build-page__item:hover {
  border-color: rgb(var(--arcoblue-5));
}

.build-page__item--active {
  border: 1px solid rgb(var(--arcoblue-6));
  background-color: var(--color-primary-light-1);
}

.build-page__item-form {
  margin-bottom: 0;
}

.build-page__item-actions {
  position: absolute;
  top: 8px;
  right: 8px;
}

.build-page__code-actions {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}

.build-page__code {
  margin: 0;
  max-height: 420px;
  overflow: auto;
  padding: 12px;
  font-size: 12px;
  line-height: 1.6;
  background-color: var(--color-fill-2);
  border-radius: 4px;
  white-space: pre;
}
</style>
