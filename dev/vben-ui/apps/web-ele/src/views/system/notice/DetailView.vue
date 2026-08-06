<script setup lang="ts">
/**
 * 公告详情抽屉
 * 对齐若依 HeaderNotice/DetailView：支持传入整行数据或仅 noticeId 拉取详情。
 */
import { computed, ref } from 'vue';

import { ElDrawer, ElIcon } from 'element-plus';
import { Bell, Message, Document, User, Clock } from '@element-plus/icons-vue';

import { getNotice, type SysNotice } from '#/api/system/notice';

const visible = ref(false);
const loading = ref(false);
const detail = ref<SysNotice | null>(null);

const isStatusNormal = computed(() => {
  const status = detail.value?.status;
  return status === '0' || (status as unknown) === 0;
});

const hasContent = computed(() => {
  const content = detail.value?.noticeContent;
  return content != null && String(content).trim() !== '';
});

const typeLabel = computed(() => {
  const type = detail.value?.noticeType;
  if (type === '1') return { text: '通知', cls: 'type-notify', icon: Bell };
  if (type === '2') return { text: '公告', cls: 'type-announce', icon: Message };
  return { text: '消息', cls: 'type-notify', icon: Document };
});

/**
 * 打开详情：可传公告对象（含内容则直接展示）或仅传 noticeId 拉取详情。
 */
async function open(payload: SysNotice | number | null | undefined) {
  let noticeId: number | null = null;
  let preset: SysNotice | null = null;

  if (payload != null && typeof payload === 'object') {
    noticeId = payload.noticeId;
    if (payload.noticeContent != null) {
      preset = payload;
    }
  } else if (payload != null) {
    noticeId = payload;
  }

  visible.value = true;

  if (preset) {
    detail.value = preset;
    return;
  }
  if (noticeId == null) {
    detail.value = null;
    return;
  }

  loading.value = true;
  detail.value = null;
  try {
    // 拦截器已解包 data，返回值即公告对象
    detail.value = await getNotice(noticeId);
  } catch {
    detail.value = null;
  } finally {
    loading.value = false;
  }
}

function handleClose() {
  visible.value = false;
  detail.value = null;
  loading.value = false;
}

defineExpose({ open });
</script>

<template>
  <ElDrawer
    v-model="visible"
    title="公告详情"
    direction="rtl"
    size="50%"
    append-to-body
    destroy-on-close
    class="notice-detail-drawer"
    @close="handleClose"
  >
    <div v-loading="loading" class="notice-detail-drawer__body">
      <div v-if="!detail" class="notice-empty">
        <ElIcon :size="28"><Document /></ElIcon>
        <span>暂无数据</span>
      </div>
      <div v-else class="notice-page">
        <div class="notice-type-wrap">
          <span :class="['notice-type-tag', typeLabel.cls]">
            <ElIcon><component :is="typeLabel.icon" /></ElIcon>
            {{ typeLabel.text }}
          </span>
        </div>

        <h1 class="notice-title">{{ detail.noticeTitle }}</h1>

        <div class="notice-meta">
          <span class="meta-item">
            <ElIcon><User /></ElIcon>
            <span>{{ detail.createBy || '—' }}</span>
          </span>
          <span class="meta-item">
            <ElIcon><Clock /></ElIcon>
            <span>{{ detail.createTime || '—' }}</span>
          </span>
          <span class="meta-item">
            <span :class="['status-dot', isStatusNormal ? 'status-ok' : 'status-off']" />
            <span>{{ isStatusNormal ? '正常' : '已关闭' }}</span>
          </span>
        </div>

        <div class="notice-divider">
          <span class="notice-divider-dot" />
          <span class="notice-divider-dot" />
          <span class="notice-divider-dot" />
        </div>

        <div class="notice-body">
          <div v-if="hasContent" class="notice-content" v-html="detail.noticeContent" />
          <div v-else class="notice-empty notice-empty--inner">
            <ElIcon :size="28"><Document /></ElIcon>
            暂无内容
          </div>
        </div>
      </div>
    </div>
  </ElDrawer>
</template>

<style scoped>
/* 颜色全部走 Element Plus 主题变量，跟随系统亮色/深色切换 */
.notice-page {
  max-width: 760px;
  margin: 0 auto;
  padding: 8px 8px 20px;
  animation: notice-fade-up 0.28s ease both;
}

@keyframes notice-fade-up {
  from {
    opacity: 0;
    transform: translateY(14px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.notice-type-tag {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 12px;
  border-radius: 2px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  margin-bottom: 14px;
}

.type-notify {
  background: var(--el-color-warning-light-9);
  color: var(--el-color-warning-dark-2);
  border-left: 3px solid var(--el-color-warning);
}

.type-announce {
  background: var(--el-color-success-light-9);
  color: var(--el-color-success-dark-2);
  border-left: 3px solid var(--el-color-success);
}

.notice-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--el-text-color-primary);
  line-height: 1.45;
  margin: 0 0 16px;
}

.notice-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
  padding: 12px 0;
  border-top: 1px solid var(--el-border-color-lighter);
  border-bottom: 1px solid var(--el-border-color-lighter);
  margin-bottom: 28px;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.status-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  margin-right: 4px;
}

.status-ok {
  background: var(--el-color-success);
}

.status-off {
  background: var(--el-color-danger);
}

.notice-divider {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.notice-divider::before,
.notice-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: linear-gradient(
    to right,
    transparent,
    var(--el-border-color),
    transparent
  );
}

.notice-divider-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--el-border-color);
}

.notice-body {
  background: var(--el-bg-color);
  border-radius: 6px;
  padding: 28px 32px;
  border: 1px solid var(--el-border-color-lighter);
  min-height: 120px;
}

.notice-content {
  font-size: 14px;
  line-height: 1.85;
  color: var(--el-text-color-regular);
  word-break: break-word;
}

.notice-content :deep(p) {
  margin: 0 0 1em;
}

.notice-content :deep(img) {
  max-width: 100%;
  border-radius: 4px;
  margin: 8px 0;
}

.notice-content :deep(a) {
  color: var(--el-color-primary);
  text-decoration: underline;
}

.notice-content :deep(h1),
.notice-content :deep(h2),
.notice-content :deep(h3) {
  color: var(--el-text-color-primary);
}

.notice-content :deep(blockquote) {
  border-left: 3px solid var(--el-border-color);
  margin: 1em 0;
  padding: 6px 16px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}

.notice-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}

.notice-content :deep(th),
.notice-content :deep(td) {
  border: 1px solid var(--el-border-color-lighter);
  padding: 7px 12px;
}

.notice-content :deep(th) {
  background: var(--el-fill-color-light);
}

.notice-empty {
  text-align: center;
  padding: 40px 0;
  color: var(--el-text-color-placeholder);
  font-size: 13px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.notice-empty--inner {
  padding: 32px 0;
}

.notice-detail-drawer__body {
  height: 100%;
  overflow: auto;
  padding: 10px 16px 22px;
  background: var(--el-bg-color-page);
}
</style>
