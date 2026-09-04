<template>
  <div class="file-upload">
    <a-upload
      v-model:file-list="fileList"
      :accept="accept"
      :multiple="limit > 1"
      :limit="limit"
      :custom-request="customRequest"
      :before-upload="beforeUpload"
      @exceed-limit="onExceedLimit"
    >
      <template #upload-button>
        <a-button type="primary">
          <template #icon><IconUpload /></template>
          {{ t('components.uploadFile') }}
        </a-button>
      </template>
      <template #upload-item="{ fileItem }">
        <div class="file-upload__item" :class="{ 'file-upload__item--error': fileItem.status === 'error' }">
          <IconLoading v-if="fileItem.status === 'uploading'" class="file-upload__status-icon" />
          <IconFile v-else class="file-upload__file-icon" />
          <span class="file-upload__name" :title="displayName(fileItem)" @click="downloadItem(fileItem)">
            {{ displayName(fileItem) }}
          </span>
          <span class="file-upload__size">{{ formatFileSize(fileItem.file?.size) }}</span>
          <a-tooltip :content="t('components.download')">
            <a-button
              type="text"
              size="mini"
              class="file-upload__action"
              :disabled="!getFileName(fileItem)"
              @click="downloadItem(fileItem)"
            >
              <template #icon><IconDownload /></template>
            </a-button>
          </a-tooltip>
          <a-tooltip :content="t('common.delete')">
            <a-button type="text" size="mini" class="file-upload__action" @click="removeItem(fileItem)">
              <template #icon><IconDelete /></template>
            </a-button>
          </a-tooltip>
        </div>
      </template>
    </a-upload>
    <div class="file-upload__tip">{{ t('components.fileSizeTip', { size: fileSize }) }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Message } from '@arco-design/web-vue'
import type { FileItem, RequestOption, UploadRequest } from '@arco-design/web-vue'
import { IconDelete, IconDownload, IconFile, IconLoading, IconUpload } from '@arco-design/web-vue/es/icon'
import { uploadFile } from '@/api/common'
import { formatFileSize, getFileBasename, resolveFileUrl } from '@/utils/file'

/**
 * 文件上传（列表形态）：
 * - 值为后端 fileName（/profile 相对路径）逗号拼接；展示名优先 originalFilename
 * - 上传经 /common/upload（FormData 字段 file），不用 a-upload 自带 action
 * - 自定义列表项：文件名（点击下载）+ 大小 + 下载/删除
 */
const props = withDefaults(
  defineProps<{
    /** 上传值：fileName 逗号拼接字符串或字符串数组（v-model:model-value） */
    modelValue?: string | string[] | null
    /** 最大数量 */
    limit?: number
    /** 单文件大小上限（MB） */
    fileSize?: number
    /** 允许的扩展名（不含点），不传不限制格式 */
    fileType?: string[]
  }>(),
  {
    modelValue: '',
    limit: 1,
    fileSize: 5,
    fileType: () => []
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const { t } = useI18n()

const fileList = ref<FileItem[]>([])

/** 防止 modelValue <-> fileList 双向同步成环：记录最近一次对外发出的值 */
let lastEmitted: string | null = null

const accept = computed(() => props.fileType.map((type) => `.${type}`).join(','))

/** modelValue -> fileName 数组 */
function parseModelValue(value: string | string[] | null | undefined): string[] {
  if (!value) return []
  const raw = Array.isArray(value) ? value.join(',') : value
  return raw
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

/** 从 FileItem.response（UploadResult）中取后端 fileName */
function getFileName(item: FileItem): string {
  const response: unknown = item.response
  if (typeof response === 'object' && response !== null && 'fileName' in response) {
    const fileName: unknown = (response as { fileName?: unknown }).fileName
    return typeof fileName === 'string' ? fileName : ''
  }
  return ''
}

/** 从 FileItem.response 中取原始文件名（无则截取 fileName） */
function displayName(item: FileItem): string {
  const response: unknown = item.response
  if (typeof response === 'object' && response !== null && 'originalFilename' in response) {
    const name: unknown = (response as { originalFilename?: unknown }).originalFilename
    if (typeof name === 'string' && name) return name
  }
  const fileName = getFileName(item)
  return fileName ? getFileBasename(fileName) : (item.name ?? '')
}

/** 当前 fileList 对应的值（仅统计 done 项） */
function joinedValue(): string {
  return fileList.value
    .filter((item) => item.status === 'done')
    .map((item) => getFileName(item))
    .filter(Boolean)
    .join(',')
}

/** 外部值 -> fileList（与当前值一致时跳过，避免回显循环） */
watch(
  () => props.modelValue,
  (value) => {
    const names = parseModelValue(value)
    const external = names.join(',')
    if (external === joinedValue()) return
    lastEmitted = external
    fileList.value = names.map((fileName) => ({
      uid: `echo-${fileName}-${Math.random().toString(36).slice(2, 8)}`,
      status: 'done',
      name: getFileBasename(fileName),
      response: { fileName }
    }))
  },
  { immediate: true }
)

/** fileList 变化 -> 对外发值（上传成功/删除统一收敛到此处） */
watch(
  fileList,
  () => {
    const value = joinedValue()
    if (value !== lastEmitted) {
      lastEmitted = value
      emit('update:modelValue', value)
    }
  },
  { deep: true }
)

/** 扩展名（小写，不含点） */
function getFileExt(name: string): string {
  const index = name.lastIndexOf('.')
  return index >= 0 ? name.slice(index + 1).toLowerCase() : ''
}

/** 上传前校验：格式（配置了才校验）、大小 */
function beforeUpload(file: File): boolean {
  if (props.fileType.length > 0 && !props.fileType.includes(getFileExt(file.name))) {
    Message.error(t('components.unsupportedFormat', { types: props.fileType.join(' / ') }))
    return false
  }
  if (file.size > props.fileSize * 1024 * 1024) {
    Message.error(t('components.fileSizeExceeded', { size: props.fileSize }))
    return false
  }
  return true
}

/** 超出数量上限（a-upload 主动丢弃多余文件时触发） */
function onExceedLimit(): void {
  Message.warning(t('components.exceedUploadLimit', { limit: props.limit }))
}

/** 自定义上传：FormData 字段 file 调 /common/upload */
function customRequest(option: RequestOption): UploadRequest {
  const file = option.fileItem.file
  if (!file) {
    option.onError()
    return {}
  }
  uploadFile(file)
    .then((result) => {
      option.onSuccess(result)
    })
    .catch(() => {
      option.onError()
    })
  return {}
}

/** 删除列表项（受控 fileList，直接移除） */
function removeItem(item: FileItem): void {
  fileList.value = fileList.value.filter((entry) => entry.uid !== item.uid)
}

/** 下载：a 标签 + download 属性（/profile 资源后端放行，经代理可访问） */
function downloadItem(item: FileItem): void {
  const fileName = getFileName(item)
  if (!fileName) return
  const link = document.createElement('a')
  link.href = resolveFileUrl(fileName)
  link.download = displayName(item)
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}
</script>

<style scoped>
.file-upload__item {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 260px;
  max-width: 380px;
  padding: 6px 8px;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
  background-color: var(--color-fill-1);
}

.file-upload__item--error {
  border-color: rgb(var(--danger-6));
}

.file-upload__status-icon {
  flex-shrink: 0;
  color: rgb(var(--primary-6));
}

.file-upload__file-icon {
  flex-shrink: 0;
  color: var(--color-text-2);
}

.file-upload__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-1);
  cursor: pointer;
}

.file-upload__name:hover {
  color: rgb(var(--primary-6));
}

.file-upload__size {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--color-text-3);
}

.file-upload__action {
  flex-shrink: 0;
  color: var(--color-text-2);
}

.file-upload__tip {
  margin-top: 4px;
  font-size: 12px;
  line-height: 18px;
  color: var(--color-text-3);
}
</style>
