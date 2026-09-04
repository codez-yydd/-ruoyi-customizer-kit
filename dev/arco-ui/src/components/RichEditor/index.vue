<template>
  <div class="rich-editor" :style="{ '--rich-editor-height': height }">
    <Toolbar
      class="rich-editor__toolbar"
      :editor="editorRef"
      :default-config="toolbarConfig"
      :mode="mode"
    />
    <Editor
      class="rich-editor__content"
      :default-config="editorConfig"
      :mode="mode"
      :model-value="modelValue"
      @update:model-value="onUpdateValue"
      @on-created="handleCreated"
    />
  </div>
</template>

<script setup lang="ts">
import { shallowRef, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import type { IDomEditor, IEditorConfig, IToolbarConfig } from '@wangeditor/editor'
import { Editor, Toolbar } from '@wangeditor/editor-for-vue'
import '@wangeditor/editor/dist/css/style.css'

/**
 * wangEditor 5 富文本封装：
 * - v-model 绑定 HTML 字符串（编辑中由 Editor 组件回传，父级直接 v-model）
 * - onCreated 拿编辑器实例（shallowRef 避免深度代理破坏内部状态）
 * - 销毁前必须 editor.destroy() 释放 DOM/事件
 */
const props = withDefaults(
  defineProps<{
    /** 富文本 HTML 内容（v-model） */
    modelValue?: string
    /** 占位提示（不传时用当前语言的默认提示） */
    placeholder?: string
    /** 编辑区高度（CSS 值，默认 300px） */
    height?: string
    /** 编辑器模式（default 完整 / simple 精简） */
    mode?: string
  }>(),
  {
    modelValue: '',
    placeholder: '',
    height: '300px',
    mode: 'default'
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const { t } = useI18n()

/** 编辑器实例（shallowRef：IDomEditor 含循环引用结构，不能深度响应式） */
// 官方推荐 shallowRef 保存实例，类型为 IDomEditor | undefined
const editorRef = shallowRef<IDomEditor>()

const toolbarConfig: Partial<IToolbarConfig> = {
  excludeKeys: ['group-video']
}

// default-config 仅在编辑器创建时读取一次，placeholder 取创建时的当前语言
const editorConfig: Partial<IEditorConfig> = {
  placeholder: props.placeholder || t('components.richTextPlaceholder'),
  MENU_CONF: {}
}

function handleCreated(editor: IDomEditor): void {
  editorRef.value = editor
}

function onUpdateValue(value: string | undefined): void {
  emit('update:modelValue', value ?? '')
}

onBeforeUnmount(() => {
  editorRef.value?.destroy()
})
</script>

<style scoped>
.rich-editor {
  width: 100%;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  z-index: 100;
}

.rich-editor__toolbar {
  border-bottom: 1px solid var(--color-border-2);
}

.rich-editor__content {
  height: var(--rich-editor-height);
  overflow-y: hidden;
}
</style>
