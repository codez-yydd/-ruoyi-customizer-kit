<template>
  <div class="image-upload">
    <a-upload
      v-model:file-list="fileList"
      list-type="picture-card"
      :accept="accept"
      :multiple="limit > 1"
      :limit="limit"
      :custom-request="customRequest"
      :before-upload="beforeUpload"
      @preview="onPreview"
      @exceed-limit="onExceedLimit"
    />
    <a-image-preview v-model:visible="previewVisible" :src="previewUrl" />
    <div class="image-upload__tip">
      {{ t('components.imageFormatTip', { types: fileType.join(' / '), size: fileSize }) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Message } from '@arco-design/web-vue'
import type { FileItem, RequestOption, UploadRequest } from '@arco-design/web-vue'
import { uploadFile } from '@/api/common'
import { resolveFileUrl } from '@/utils/file'

/**
 * 图片上传：
 * - 值为后端 fileName（/profile 相对路径）逗号拼接；回显拼 VITE_APP_BASE_API 前缀
 * - 上传经 /common/upload（FormData 字段 file），不用 a-upload 自带 action
 * - 支持多图、格式/大小校验、删除、大图预览；上传中/失败状态由 a-upload 呈现
 */
const props = withDefaults(
  defineProps<{
    /** 上传值：fileName 逗号拼接字符串或字符串数组（v-model:model-value） */
    modelValue?: string | string[] | null
    /** 最大数量 */
    limit?: number
    /** 单文件大小上限（MB） */
    fileSize?: number
    /** 允许的扩展名（不含点） */
    fileType?: string[]
  }>(),
  {
    modelValue: '',
    limit: 1,
    fileSize: 5,
    fileType: () => ['png', 'jpg', 'jpeg']
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const { t } = useI18n()

const fileList = ref<FileItem[]>([])
const previewVisible = ref(false)
const previewUrl = ref('')

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
      name: fileName.split('/').pop() || fileName,
      url: resolveFileUrl(fileName),
      // 回显项补 response，使 joinedValue 与新上传项走同一取值路径
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

/** 上传前校验：格式、大小、数量 */
function beforeUpload(file: File): boolean {
  if (!props.fileType.includes(getFileExt(file.name))) {
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
      // 预览地址用 fileName 拼 API 前缀（后端 url 域名与 dev 环境不一致）
      option.fileItem.url = resolveFileUrl(result.fileName)
    })
    .catch(() => {
      option.onError()
    })
  return {}
}

/** 点击预览：大图预览 */
function onPreview(item: FileItem): void {
  previewUrl.value = item.url ?? ''
  previewVisible.value = true
}
</script>

<style scoped>
.image-upload__tip {
  font-size: 12px;
  line-height: 18px;
  color: var(--color-text-3);
}
</style>
