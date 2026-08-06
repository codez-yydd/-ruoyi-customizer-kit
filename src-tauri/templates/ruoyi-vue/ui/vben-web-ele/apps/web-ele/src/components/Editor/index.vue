<script setup lang="ts">
/**
 * 富文本编辑器（Quill）
 * 对齐若依原版 Editor：支持工具栏格式化、图片上传（/common/upload）。
 * 用于通知公告等内容字段，输出 HTML 字符串。
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';

import { ElMessage } from 'element-plus';
import Quill from 'quill';

import { requestClient } from '#/api/request';

import 'quill/dist/quill.core.css';
import 'quill/dist/quill.snow.css';

const props = withDefaults(
  defineProps<{
    /** 编辑器内容（HTML） */
    modelValue?: string;
    /** 固定高度 */
    height?: number;
    /** 最小高度 */
    minHeight?: number;
    /** 只读 */
    readOnly?: boolean;
    /** 上传图片大小限制（MB） */
    fileSize?: number;
  }>(),
  {
    modelValue: '',
    height: undefined,
    minHeight: 192,
    readOnly: false,
    fileSize: 5,
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const editorRef = ref<HTMLElement>();
const fileInputRef = ref<HTMLInputElement>();
let quillInstance: Quill | null = null;
/** 内部当前 HTML，用于避免与外部 v-model 互相覆盖 */
let currentHtml = '';

const editorStyles = computed(() => {
  const style: Record<string, string> = {};
  if (props.minHeight) style.minHeight = `${props.minHeight}px`;
  if (props.height) style.height = `${props.height}px`;
  return style;
});

function initEditor() {
  if (!editorRef.value) return;

  quillInstance = new Quill(editorRef.value, {
    theme: 'snow',
    bounds: document.body,
    debug: 'warn',
    readOnly: props.readOnly,
    placeholder: '请输入内容',
    modules: {
      toolbar: [
        ['bold', 'italic', 'underline', 'strike'],
        ['blockquote', 'code-block'],
        [{ list: 'ordered' }, { list: 'bullet' }],
        [{ indent: '-1' }, { indent: '+1' }],
        [{ size: ['small', false, 'large', 'huge'] }],
        [{ header: [1, 2, 3, 4, 5, 6, false] }],
        [{ color: [] }, { background: [] }],
        [{ align: [] }],
        ['clean'],
        ['link', 'image', 'video'],
      ],
    },
  });

  // 自定义图片按钮：触发隐藏 file input，统一走若依 /common/upload
  const toolbar = quillInstance.getModule('toolbar') as {
    addHandler: (name: string, fn: (value: boolean) => void) => void;
  };
  toolbar.addHandler('image', (value: boolean) => {
    if (value) {
      fileInputRef.value?.click();
    } else {
      quillInstance?.format('image', false);
    }
  });

  quillInstance.root.addEventListener('paste', handlePasteCapture, true);

  if (currentHtml) {
    quillInstance.clipboard.dangerouslyPasteHTML(currentHtml);
  }

  quillInstance.on('text-change', () => {
    const html = editorRef.value?.querySelector('.ql-editor')?.innerHTML ?? '';
    currentHtml = html;
    emit('update:modelValue', html);
  });
}

function beforeUpload(file: File) {
  const allowTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/svg+xml', 'image/svg'];
  if (!allowTypes.includes(file.type)) {
    ElMessage.error('图片格式错误，仅支持 jpg/png/svg');
    return false;
  }
  if (props.fileSize && file.size / 1024 / 1024 >= props.fileSize) {
    ElMessage.error(`上传文件大小不能超过 ${props.fileSize} MB`);
    return false;
  }
  return true;
}

/** 插入已上传图片到光标位置 */
function insertUploadedImage(fileName: string) {
  if (!quillInstance) return;
  const apiBase = import.meta.env.VITE_GLOB_API_URL || '';
  const range = quillInstance.getSelection(true);
  const index = range?.index ?? quillInstance.getLength();
  quillInstance.insertEmbed(index, 'image', `${apiBase}${fileName}`);
  quillInstance.setSelection(index + 1);
}

async function uploadImageFile(file: File) {
  try {
    const formData = new FormData();
    formData.append('file', file);
    // 上传接口无 data 字段，url/fileName 在响应顶层，需保留完整响应
    const res = (await requestClient.post('/common/upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
      rawResponse: true,
    })) as { code?: number; fileName?: string; url?: string; msg?: string };

    if (res?.code === 200 && res.fileName) {
      insertUploadedImage(res.fileName);
    } else {
      ElMessage.error(res?.msg || '图片插入失败');
    }
  } catch {
    ElMessage.error('图片插入失败');
  }
}

function handleFileChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  // 清空 value，允许连续选择同一文件
  input.value = '';
  if (!file) return;
  if (!beforeUpload(file)) return;
  uploadImageFile(file);
}

/** 粘贴图片时走上传，避免直接塞 base64 */
function handlePasteCapture(e: ClipboardEvent) {
  const clipboard = e.clipboardData;
  if (!clipboard?.items) return;
  for (let i = 0; i < clipboard.items.length; i++) {
    const item = clipboard.items[i];
    if (item?.type?.startsWith('image/')) {
      e.preventDefault();
      const file = item.getAsFile();
      if (file && beforeUpload(file)) {
        uploadImageFile(file);
      }
      break;
    }
  }
}

watch(
  () => props.modelValue,
  (val) => {
    const next = val ?? '';
    if (next !== currentHtml) {
      currentHtml = next;
      if (quillInstance) {
        const selection = quillInstance.getSelection();
        quillInstance.clipboard.dangerouslyPasteHTML(currentHtml);
        if (selection) quillInstance.setSelection(selection);
      }
    }
  },
  { immediate: true },
);

onMounted(initEditor);

onBeforeUnmount(() => {
  quillInstance?.root.removeEventListener('paste', handlePasteCapture, true);
  quillInstance = null;
});
</script>

<template>
  <div class="ruoyi-editor">
    <input
      ref="fileInputRef"
      type="file"
      accept="image/jpeg,image/jpg,image/png,image/svg+xml"
      class="ruoyi-editor__upload"
      @change="handleFileChange"
    />
    <div ref="editorRef" class="editor" :style="editorStyles" />
  </div>
</template>

<style>
.ruoyi-editor__upload {
  display: none;
}

.ruoyi-editor .editor,
.ruoyi-editor .ql-toolbar {
  white-space: pre-wrap !important;
  line-height: normal !important;
}

.ruoyi-editor .ql-snow .ql-tooltip[data-mode='link']::before {
  content: '请输入链接地址:';
}

.ruoyi-editor .ql-snow .ql-tooltip.ql-editing a.ql-action::after {
  border-right: 0;
  content: '保存';
  padding-right: 0;
}

.ruoyi-editor .ql-snow .ql-tooltip[data-mode='video']::before {
  content: '请输入视频地址:';
}

.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-label::before,
.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-item::before {
  content: '14px';
}

.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-label[data-value='small']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-item[data-value='small']::before {
  content: '10px';
}

.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-label[data-value='large']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-item[data-value='large']::before {
  content: '18px';
}

.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-label[data-value='huge']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-size .ql-picker-item[data-value='huge']::before {
  content: '32px';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item::before {
  content: '文本';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='1']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='1']::before {
  content: '标题1';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='2']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='2']::before {
  content: '标题2';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='3']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='3']::before {
  content: '标题3';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='4']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='4']::before {
  content: '标题4';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='5']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='5']::before {
  content: '标题5';
}

.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-label[data-value='6']::before,
.ruoyi-editor .ql-snow .ql-picker.ql-header .ql-picker-item[data-value='6']::before {
  content: '标题6';
}
</style>
