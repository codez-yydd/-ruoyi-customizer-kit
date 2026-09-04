<template>
  <div class="error-page">
    <a-result status="404" :subtitle="t('error.notFound')">
      <template #extra>
        <a-space :size="12">
          <a-button type="primary" @click="goHome">{{ t('error.backHome') }}</a-button>
          <a-button @click="goBack">{{ t('error.goBack') }}</a-button>
        </a-space>
      </template>
    </a-result>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { usePermissionStore } from '@/stores/permission'

/** 404 页面不存在（a-result 风格） */
const { t } = useI18n()
const router = useRouter()
const permissionStore = usePermissionStore()

function goHome(): void {
  router.push(permissionStore.firstMenuPath)
}

function goBack(): void {
  router.back()
}
</script>

<style scoped>
.error-page {
  height: calc(100vh - var(--header-height) - var(--tabs-height));
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
