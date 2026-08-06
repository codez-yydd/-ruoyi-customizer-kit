<script lang="ts" setup>
import { computed, reactive, ref } from 'vue';

import { useUserStore } from '@vben/stores';

import {
  ElButton,
  ElCol,
  ElDialog,
  ElMessage,
  ElRow,
  ElUpload,
} from 'element-plus';
import {
  Plus,
  Minus,
  RefreshLeft,
  RefreshRight,
  Upload,
} from '@element-plus/icons-vue';
import { VueCropper } from 'vue-cropper/dist/vue-cropper.es.js';
import type { VueCropperInstance } from 'vue-cropper/dist/vue-cropper.es.js';
import 'vue-cropper/dist/index.css';

import { uploadAvatarApi } from '#/api/system/profile';

defineOptions({ name: 'UserAvatar' });

const props = defineProps<{
  /** 当前头像地址（已拼好 API 前缀的完整 URL，或相对路径） */
  avatar?: string;
}>();

const emit = defineEmits<{
  /** 头像更新成功后回传相对路径 imgUrl，供父组件同步展示 */
  (e: 'success', imgUrl: string): void;
}>();

const userStore = useUserStore();

const open = ref(false);
/** 对话框完全打开后再挂载 cropper，避免尺寸计算错误 */
const cropperVisible = ref(false);
const cropperRef = ref<VueCropperInstance>();
const submitting = ref(false);

/** 将相对路径拼成可访问 URL */
function resolveAvatarUrl(path?: string) {
  const a = path ?? '';
  if (!a) return '';
  return /^https?:\/\//i.test(a)
    ? a
    : `${import.meta.env.VITE_GLOB_API_URL}${a}`;
}

const displayAvatar = computed(() => {
  return (
    resolveAvatarUrl(props.avatar) ||
    userStore.userInfo?.avatar ||
    ''
  );
});

const options = reactive({
  img: '',
  autoCrop: true,
  autoCropWidth: 200,
  autoCropHeight: 200,
  fixedBox: true,
  outputType: 'png' as const,
  filename: 'avatar.png',
  previews: {} as { url?: string; img?: Record<string, string> },
});

/** 打开裁剪弹窗 */
function openCropper() {
  options.img = displayAvatar.value;
  open.value = true;
}

function onDialogOpened() {
  cropperVisible.value = true;
}

function onDialogClosed() {
  cropperVisible.value = false;
  options.img = displayAvatar.value;
  options.previews = {};
}

function requestUpload() {
  // 覆盖 el-upload 默认上传，仅本地选图
  return Promise.resolve();
}

function beforeUpload(file: File) {
  if (!file.type.startsWith('image/')) {
    ElMessage.error('文件格式错误，请上传图片类型，如：JPG、PNG 后缀的文件');
    return false;
  }
  const reader = new FileReader();
  reader.readAsDataURL(file);
  reader.onload = () => {
    options.img = String(reader.result || '');
    options.filename = file.name || 'avatar.png';
  };
  return false;
}

function changeScale(num: number) {
  cropperRef.value?.changeScale(num);
}

function rotateLeft() {
  cropperRef.value?.rotateLeft();
}

function rotateRight() {
  cropperRef.value?.rotateRight();
}

function onRealTime(data: { url?: string; img?: Record<string, string> }) {
  options.previews = data;
}

/** 裁剪并上传到若依 /system/user/profile/avatar */
async function submitCrop() {
  if (!cropperRef.value) return;
  submitting.value = true;
  try {
    await new Promise<void>((resolve, reject) => {
      cropperRef.value!.getCropBlob(async (blob: Blob) => {
        try {
          const result = await uploadAvatarApi(blob, options.filename);
          const imgUrl = result?.imgUrl;
          if (!imgUrl) {
            reject(new Error('上传失败'));
            return;
          }
          const fullUrl = resolveAvatarUrl(imgUrl);
          // 同步顶栏头像
          if (userStore.userInfo) {
            userStore.userInfo.avatar = fullUrl;
          }
          emit('success', imgUrl);
          ElMessage.success('修改成功');
          open.value = false;
          resolve();
        } catch (error) {
          reject(error);
        }
      });
    });
  } catch {
    ElMessage.error('头像上传失败');
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div>
    <div class="user-info-head" @click="openCropper">
      <img
        v-if="displayAvatar"
        :src="displayAvatar"
        title="点击上传头像"
        class="img-circle"
        alt="头像"
      />
      <div v-else class="img-circle img-placeholder" title="点击上传头像">
        {{ userStore.userInfo?.realName?.charAt(0)?.toUpperCase() || 'U' }}
      </div>
    </div>

    <ElDialog
      v-model="open"
      title="修改头像"
      width="800px"
      append-to-body
      destroy-on-close
      @opened="onDialogOpened"
      @closed="onDialogClosed"
    >
      <ElRow>
        <ElCol :xs="24" :md="12" class="cropper-box">
          <VueCropper
            v-if="cropperVisible"
            ref="cropperRef"
            :img="options.img"
            :info="true"
            :auto-crop="options.autoCrop"
            :auto-crop-width="options.autoCropWidth"
            :auto-crop-height="options.autoCropHeight"
            :fixed-box="options.fixedBox"
            :output-type="options.outputType"
            @real-time="onRealTime"
          />
        </ElCol>
        <ElCol :xs="24" :md="12" class="cropper-box">
          <div class="avatar-upload-preview">
            <img
              v-if="options.previews.url"
              :src="options.previews.url"
              :style="options.previews.img"
              alt="预览"
            />
          </div>
        </ElCol>
      </ElRow>
      <ElRow class="mt-4" align="middle">
        <ElCol :lg="2" :sm="3" :xs="4">
          <ElUpload
            action="#"
            :http-request="requestUpload"
            :show-file-list="false"
            :before-upload="beforeUpload"
          >
            <ElButton size="small">
              选择
              <Upload class="ml-1 size-3.5" />
            </ElButton>
          </ElUpload>
        </ElCol>
        <ElCol :lg="{ span: 1, offset: 2 }" :sm="2" :xs="4">
          <ElButton size="small" :icon="Plus" @click="changeScale(1)" />
        </ElCol>
        <ElCol :lg="{ span: 1, offset: 1 }" :sm="2" :xs="4">
          <ElButton size="small" :icon="Minus" @click="changeScale(-1)" />
        </ElCol>
        <ElCol :lg="{ span: 1, offset: 1 }" :sm="2" :xs="4">
          <ElButton size="small" :icon="RefreshLeft" @click="rotateLeft" />
        </ElCol>
        <ElCol :lg="{ span: 1, offset: 1 }" :sm="2" :xs="4">
          <ElButton size="small" :icon="RefreshRight" @click="rotateRight" />
        </ElCol>
        <ElCol :lg="{ span: 2, offset: 6 }" :sm="4" :xs="6">
          <ElButton
            type="primary"
            size="small"
            :loading="submitting"
            @click="submitCrop"
          >
            提 交
          </ElButton>
        </ElCol>
      </ElRow>
    </ElDialog>
  </div>
</template>

<style scoped>
.user-info-head {
  position: relative;
  display: inline-block;
  height: 120px;
  cursor: pointer;
}

.img-circle {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  object-fit: cover;
  border: 1px solid var(--el-border-color);
}

.img-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}

.user-info-head:hover::after {
  content: '+';
  position: absolute;
  inset: 0;
  color: #eee;
  background: rgb(0 0 0 / 50%);
  font-size: 24px;
  font-style: normal;
  line-height: 120px;
  border-radius: 50%;
  text-align: center;
}

.cropper-box {
  position: relative;
  height: 350px;
}

.avatar-upload-preview {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 200px;
  height: 200px;
  border-radius: 50%;
  box-shadow: 0 0 4px #ccc;
  overflow: hidden;
}

.mt-4 {
  margin-top: 16px;
}

.ml-1 {
  margin-left: 4px;
}

.size-3\.5 {
  width: 14px;
  height: 14px;
}
</style>
