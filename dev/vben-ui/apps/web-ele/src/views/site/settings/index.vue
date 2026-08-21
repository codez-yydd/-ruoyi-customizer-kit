<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElCard,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElUpload,
} from 'element-plus';
import { UploadFilled } from '@element-plus/icons-vue';
import { updatePreferences } from '@vben/preferences';

import {
  getSiteSettings,
  updateSiteSettings,
  uploadLogoApi,
  type SiteSettings,
} from '#/api/site/settings';

defineOptions({ name: 'SiteSettings' });

const loading = ref(false);
const uploading = ref(false);
const formRef = ref();
const form = reactive<SiteSettings>({ icp: '', logo: '', title: '' });

const rules = {
  title: [{ max: 50, message: '标题不能超过 50 个字符', trigger: 'blur' }],
  icp: [{ max: 100, message: '备案号不能超过 100 个字符', trigger: 'blur' }],
};

/** Logo 预览地址：/profile/upload/... 需带接口前缀访问后端静态资源 */
const apiBaseUrl = import.meta.env.VITE_GLOB_API_URL || '/prod-api';
const logoPreview = ref('');

function applyForm(data?: SiteSettings) {
  form.title = data?.title || '';
  form.logo = data?.logo || '';
  form.icp = data?.icp || '';
  logoPreview.value = form.logo ? `${apiBaseUrl}${form.logo}` : '';
}

async function load() {
  const res = await getSiteSettings();
  applyForm(res.data);
}

/** 站点标题/Logo 保存后即时生效（侧边栏、登录页、页脚同步更新） */
function applyRuntime(data?: SiteSettings) {
  const title = data?.title || import.meta.env.VITE_APP_TITLE || '';
  updatePreferences({
    app: { name: title },
    logo: data?.logo ? { source: `${apiBaseUrl}${data.logo}` } : {},
  });
}

async function handleSave() {
  await formRef.value?.validate();
  loading.value = true;
  try {
    const res = await updateSiteSettings({ ...form });
    const data = (res as unknown as SiteSettings) ?? {};
    applyForm(data);
    applyRuntime(data);
    ElMessage.success('保存成功');
  } finally {
    loading.value = false;
  }
}

function handleReset() {
  load();
}

/** ElUpload 自定义上传：走若依 /common/upload，仅保留返回的相对路径 */
async function requestUpload(options: { file: File }) {
  uploading.value = true;
  try {
    const fileName = await uploadLogoApi(options.file);
    form.logo = fileName;
    logoPreview.value = `${apiBaseUrl}${fileName}`;
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : 'Logo 上传失败');
  } finally {
    uploading.value = false;
  }
}

function handleRemoveLogo() {
  form.logo = '';
  logoPreview.value = '';
}

onMounted(load);
</script>

<template>
  <div class="app-container">
    <ElCard shadow="never">
      <template #header><span>站点设置</span></template>
      <ElForm
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="120px"
        style="max-width: 640px"
      >
        <ElFormItem label="站点标题" prop="title">
          <ElInput
            v-model="form.title"
            maxlength="50"
            placeholder="留空使用打包时的默认标题"
            show-word-limit
          />
          <div class="site-tip">侧边栏、登录页、浏览器标签页与页脚版权中的系统名称</div>
        </ElFormItem>
        <ElFormItem label="后台Logo" prop="logo">
          <div class="logo-row">
            <ElUpload
              :http-request="requestUpload"
              :show-file-list="false"
              accept="image/png,image/jpeg"
              class="logo-uploader"
            >
              <img v-if="logoPreview" :src="logoPreview" class="logo-preview" alt="logo" />
              <ElButton v-else :loading="uploading" :icon="UploadFilled">
                {{ uploading ? '上传中...' : '上传 Logo' }}
              </ElButton>
            </ElUpload>
            <ElButton v-if="form.logo" link type="danger" @click="handleRemoveLogo">
              移除
            </ElButton>
          </div>
          <div class="site-tip">建议 PNG 格式；留空使用默认（仅文字标题），保存后立即生效</div>
        </ElFormItem>
        <ElFormItem label="ICP备案号" prop="icp">
          <ElInput
            v-model="form.icp"
            maxlength="100"
            placeholder="如 浙ICP备2026000000号-1，留空则不显示"
          />
          <div class="site-tip">
            备案通过后在此填写即可，页脚同步显示（优先级高于 application.yaml 的
            ruoyi.icp）
          </div>
        </ElFormItem>
        <ElFormItem>
          <ElButton :loading="loading" type="primary" @click="handleSave">保 存</ElButton>
          <ElButton @click="handleReset">重 置</ElButton>
        </ElFormItem>
      </ElForm>
    </ElCard>
  </div>
</template>

<style scoped>
.site-tip {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.8;
}

.logo-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo-preview {
  width: 64px;
  height: 64px;
  object-fit: contain;
  border: 1px dashed var(--el-border-color);
  border-radius: 6px;
  padding: 4px;
}
</style>
