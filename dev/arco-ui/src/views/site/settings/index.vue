<template>
  <div class="site-settings-page">
    <a-card :bordered="false" class="app-page-card" :title="t('site.settings.pageTitle')">
      <a-form
        ref="formRef"
        :model="form"
        :rules="formRules"
        auto-label-width
        class="site-settings-page__form"
      >
        <a-form-item field="title" :label="t('site.settings.title')">
          <a-input
            v-model.trim="form.title"
            :placeholder="t('site.settings.titlePlaceholder')"
            :max-length="50"
            show-word-limit
            allow-clear
          />
          <div class="site-settings-page__tip">{{ t('site.settings.titleTip') }}</div>
        </a-form-item>
        <a-form-item field="logo" :label="t('site.settings.logo')">
          <ImageUpload v-model="form.logo" :limit="1" />
          <div class="site-settings-page__tip">{{ t('site.settings.logoTip') }}</div>
        </a-form-item>
        <a-form-item field="icp" :label="t('site.settings.icp')">
          <a-input
            v-model.trim="form.icp"
            :placeholder="t('site.settings.icpPlaceholder')"
            :max-length="100"
            show-word-limit
            allow-clear
          />
          <div class="site-settings-page__tip">{{ t('site.settings.icpTip') }}</div>
        </a-form-item>
        <a-form-item>
          <a-space>
            <a-button
              v-hasPermi="['site:settings:edit']"
              type="primary"
              :loading="loading"
              @click="handleSave"
            >
              {{ t('common.save') }}
            </a-button>
            <a-button @click="handleReset">{{ t('common.reset') }}</a-button>
          </a-space>
        </a-form-item>
      </a-form>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { FieldRule, FormInstance } from '@arco-design/web-vue'
import { Message } from '@arco-design/web-vue'
import ImageUpload from '@/components/ImageUpload/index.vue'
import { getSiteSettings, updateSiteSettings, type SiteSettings } from '@/api/site/settings'
import { useAppStore } from '@/stores/app'

defineOptions({ name: 'SiteSettings' })

/** 站点设置：标题 / 后台 Logo / ICP，保存后全站即时生效 */
const { t } = useI18n()
const appStore = useAppStore()

const formRef = ref<FormInstance>()
const loading = ref(false)
const form = reactive<SiteSettings>({ title: '', logo: '', icp: '' })

const formRules = computed<Record<string, FieldRule[]>>(() => ({
  title: [{ maxLength: 50, message: t('common.maxLengthTip', { max: 50 }) }],
  icp: [{ maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }]
}))

function applyForm(data?: SiteSettings): void {
  form.title = data?.title || ''
  form.logo = data?.logo || ''
  form.icp = data?.icp || ''
}

async function load(): Promise<void> {
  const data = await getSiteSettings()
  applyForm(data)
}

async function handleSave(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }
  loading.value = true
  try {
    const data = await updateSiteSettings({ ...form })
    applyForm(data)
    appStore.setSite({ title: data?.title, logo: data?.logo, icp: data?.icp })
    Message.success(t('site.settings.saveSuccess'))
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    loading.value = false
  }
}

function handleReset(): void {
  void load()
}

onMounted(() => {
  void load()
})
</script>

<style scoped>
.site-settings-page__form {
  max-width: 640px;
}

.site-settings-page__tip {
  margin-top: 4px;
  font-size: 12px;
  line-height: 1.8;
  color: var(--color-text-3);
}
</style>