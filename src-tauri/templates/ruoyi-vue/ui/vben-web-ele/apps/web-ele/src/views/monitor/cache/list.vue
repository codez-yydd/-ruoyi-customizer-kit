<script setup lang="ts">
import { onMounted, ref } from 'vue';

import {
  ElButton,
  ElCard,
  ElCol,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElRow,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Collection, Delete, Document, Key, RefreshRight } from '@element-plus/icons-vue';

import {
  clearCacheAll,
  clearCacheKey,
  clearCacheName,
  getCacheValue,
  listCacheKey,
  listCacheName,
  type SysCache,
} from '#/api/monitor/cache';

defineOptions({ name: 'MonitorCacheList' });

/** 表格行：保留完整 Redis 键，避免 ElTable 对原始字符串数组支持不佳 */
interface CacheKeyRow {
  fullKey: string;
}

const cacheNames = ref<SysCache[]>([]);
const cacheKeys = ref<CacheKeyRow[]>([]);
const cacheForm = ref<Partial<SysCache>>({});
const loading = ref(false);
const subLoading = ref(false);
/** 当前选中的缓存名称（含冒号后缀，与 Redis key 前缀一致） */
const nowCacheName = ref('');

/** 查询缓存名称列表 */
async function getCacheNames() {
  loading.value = true;
  try {
    cacheNames.value = (await listCacheName()) ?? [];
  } finally {
    loading.value = false;
  }
}

/** 刷新缓存名称列表 */
async function refreshCacheNames() {
  await getCacheNames();
  ElMessage.success('刷新缓存列表成功');
}

/**
 * 查询指定缓存名称下的键列表。
 * row 有值时来自行点击；无值时用当前已选名称刷新（清理后回刷）。
 */
async function getCacheKeys(row?: SysCache) {
  const cacheName = row?.cacheName ?? nowCacheName.value;
  if (!cacheName) {
    return;
  }
  subLoading.value = true;
  try {
    const keys = (await listCacheKey(cacheName)) ?? [];
    cacheKeys.value = keys.map((fullKey) => ({ fullKey }));
    nowCacheName.value = cacheName;
    // 切换名称时清空右侧详情，避免展示过期内容
    if (row) {
      cacheForm.value = {};
    }
  } finally {
    subLoading.value = false;
  }
}

/** 刷新键名列表 */
async function refreshCacheKeys() {
  await getCacheKeys();
  ElMessage.success('刷新键名列表成功');
}

/** 列表展示时去掉缓存名称末尾冒号，便于阅读 */
function formatCacheName(row: SysCache) {
  return (row.cacheName ?? '').replace(':', '');
}

/** 键名展示时去掉当前缓存名称前缀 */
function formatCacheKey(fullKey: string) {
  return (fullKey ?? '').replace(nowCacheName.value, '');
}

/** 查询缓存内容详细 */
async function handleCacheValue(row: CacheKeyRow) {
  cacheForm.value =
    (await getCacheValue(nowCacheName.value, row.fullKey)) ?? {};
}

/** 清理指定名称下全部缓存 */
async function handleClearCacheName(row: SysCache) {
  try {
    await ElMessageBox.confirm(
      `确认要清空「${formatCacheName(row)}」的缓存吗？`,
      '提示',
      { type: 'warning' },
    );
    await clearCacheName(row.cacheName);
    ElMessage.success(`清理缓存名称[${row.cacheName}]成功`);
    nowCacheName.value = row.cacheName;
    cacheForm.value = {};
    await getCacheKeys();
  } catch {
    /* 用户取消 */
  }
}

/** 清理单个缓存键（接口需要完整 Redis 键） */
async function handleClearCacheKey(row: CacheKeyRow) {
  try {
    await ElMessageBox.confirm(
      `确认要清理键「${formatCacheKey(row.fullKey)}」吗？`,
      '提示',
      { type: 'warning' },
    );
    await clearCacheKey(row.fullKey);
    ElMessage.success(`清理缓存键名[${row.fullKey}]成功`);
    cacheForm.value = {};
    await getCacheKeys();
  } catch {
    /* 用户取消 */
  }
}

/** 清理全部缓存 */
async function handleClearCacheAll() {
  try {
    await ElMessageBox.confirm('确认要清空所有缓存吗？此操作不可逆', '提示', {
      type: 'warning',
    });
    await clearCacheAll();
    ElMessage.success('清理全部缓存成功');
    cacheKeys.value = [];
    cacheForm.value = {};
    nowCacheName.value = '';
  } catch {
    /* 用户取消 */
  }
}

onMounted(getCacheNames);
</script>

<template>
  <div class="cache-list-page">
    <ElRow :gutter="10">
      <!-- 缓存名称列表 -->
      <ElCol :xs="24" :sm="24" :md="8">
        <ElCard shadow="never" class="panel-card">
          <template #header>
            <div class="card-header">
              <span class="card-title">
                <Collection class="title-icon" />
                缓存列表
              </span>
              <ElButton link type="primary" :icon="RefreshRight" @click="refreshCacheNames" />
            </div>
          </template>
          <ElTable
            v-loading="loading"
            :data="cacheNames"
            highlight-current-row
            height="calc(100vh - 200px)"
            style="width: 100%"
            @row-click="getCacheKeys"
          >
            <ElTableColumn label="序号" type="index" width="60" align="center" />
            <ElTableColumn label="缓存名称" align="center" show-overflow-tooltip>
              <template #default="{ row }">
                {{ formatCacheName(row) }}
              </template>
            </ElTableColumn>
            <ElTableColumn
              label="备注"
              align="center"
              prop="remark"
              show-overflow-tooltip
            />
            <ElTableColumn label="操作" width="60" align="center">
              <template #default="{ row }">
                <ElButton
                  link
                  type="danger"
                  :icon="Delete"
                  @click.stop="handleClearCacheName(row)"
                />
              </template>
            </ElTableColumn>
          </ElTable>
        </ElCard>
      </ElCol>

      <!-- 键名列表 -->
      <ElCol :xs="24" :sm="24" :md="8">
        <ElCard shadow="never" class="panel-card">
          <template #header>
            <div class="card-header">
              <span class="card-title">
                <Key class="title-icon" />
                键名列表
              </span>
              <ElButton link type="primary" :icon="RefreshRight" @click="refreshCacheKeys" />
            </div>
          </template>
          <ElTable
            v-loading="subLoading"
            :data="cacheKeys"
            highlight-current-row
            height="calc(100vh - 200px)"
            style="width: 100%"
            @row-click="handleCacheValue"
          >
            <ElTableColumn label="序号" type="index" width="60" align="center" />
            <ElTableColumn label="缓存键名" align="center" show-overflow-tooltip>
              <template #default="{ row }">
                {{ formatCacheKey(row.fullKey) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="操作" width="60" align="center">
              <template #default="{ row }">
                <ElButton
                  link
                  type="danger"
                  :icon="Delete"
                  @click.stop="handleClearCacheKey(row)"
                />
              </template>
            </ElTableColumn>
          </ElTable>
        </ElCard>
      </ElCol>

      <!-- 缓存内容 -->
      <ElCol :xs="24" :sm="24" :md="8">
        <ElCard shadow="never" class="panel-card">
          <template #header>
            <div class="card-header">
              <span class="card-title">
                <Document class="title-icon" />
                缓存内容
              </span>
              <ElButton link type="danger" @click="handleClearCacheAll">清理全部</ElButton>
            </div>
          </template>
          <ElForm :model="cacheForm" label-width="90px" class="cache-form">
            <ElFormItem label="缓存名称:">
              <ElInput :model-value="cacheForm.cacheName" readonly />
            </ElFormItem>
            <ElFormItem label="缓存键名:">
              <ElInput :model-value="cacheForm.cacheKey" readonly />
            </ElFormItem>
            <ElFormItem label="缓存内容:">
              <ElInput
                :model-value="cacheForm.cacheValue"
                type="textarea"
                :rows="12"
                readonly
              />
            </ElFormItem>
          </ElForm>
        </ElCard>
      </ElCol>
    </ElRow>
  </div>
</template>

<style scoped>
.cache-list-page {
  padding: 12px;
}

.panel-card {
  margin-bottom: 10px;
}

.panel-card :deep(.el-card__body) {
  padding: 10px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
}

.title-icon {
  width: 1em;
  height: 1em;
}

.cache-form {
  padding: 8px 12px 0;
}

.cache-form :deep(.el-textarea__inner) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
}
</style>
