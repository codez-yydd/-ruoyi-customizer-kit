<script setup lang="ts">
import { onMounted, ref } from 'vue';

import {
  ElButton,
  ElCard,
  ElDescriptions,
  ElDescriptionsItem,
  ElEmpty,
  ElMessage,
  ElMessageBox,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Delete, Refresh } from '@element-plus/icons-vue';

import {
  clearCacheAll,
  clearCacheKey,
  clearCacheName,
  getCache,
  getCacheValue,
  listCacheKey,
  listCacheName,
  type CacheInfo,
} from '#/api/monitor/cache';

defineOptions({ name: 'MonitorCache' });

const info = ref<CacheInfo | null>(null);
const loading = ref(false);

const cacheNames = ref<{ name: string; keySize: number }[]>([]);
const cacheKeys = ref<string[]>([]);
const cacheValue = ref<any>(null);
const currentName = ref('');
const currentKey = ref('');

async function getInfo() {
  loading.value = true;
  try {
    info.value = await getCache();
  } finally {
    loading.value = false;
  }
}

async function loadNames() {
  cacheNames.value = (await listCacheName()) ?? [];
}

async function loadKeys(name: string) {
  currentName.value = name;
  cacheKeys.value = (await listCacheKey(name)) ?? [];
  currentKey.value = '';
  cacheValue.value = null;
}

async function loadValue(key: string) {
  currentKey.value = key;
  cacheValue.value = await getCacheValue(currentName.value, key);
}

async function handleClearName(name: string) {
  try {
    await ElMessageBox.confirm(`确认要清空"${name}"的缓存吗？`, '提示', { type: 'warning' });
    await clearCacheName(name);
    ElMessage.success('清理成功');
    loadNames();
  } catch {
    /* 取消 */
  }
}

async function handleClearKey(key: string) {
  try {
    await ElMessageBox.confirm(`确认要清理键"${key}"吗？`, '提示', { type: 'warning' });
    await clearCacheKey(key);
    ElMessage.success('清理成功');
    loadKeys(currentName.value);
  } catch {
    /* 取消 */
  }
}

async function handleClearAll() {
  try {
    await ElMessageBox.confirm('确认要清空所有缓存吗？此操作不可逆', '提示', { type: 'warning' });
    await clearCacheAll();
    ElMessage.success('清理成功');
    getInfo();
    loadNames();
  } catch {
    /* 取消 */
  }
}

function refreshAll() {
  getInfo();
  loadNames();
}

/** Redis 信息分组展示的关键字段（预留，按需使用） */
// const redisInfoKeys = ['redis_version', 'redis_mode', 'used_memory_human'];

onMounted(() => {
  getInfo();
  loadNames();
});
</script>

<template>
  <div class="cache-page">
    <div class="cache-toolbar">
      <ElButton :icon="Refresh" @click="refreshAll">刷新</ElButton>
      <ElButton type="danger" :icon="Delete" @click="handleClearAll">清空全部缓存</ElButton>
    </div>

    <!-- Redis 概览 -->
    <ElCard class="info-card" shadow="never">
      <template #header><span>Redis 基本信息</span></template>
      <ElDescriptions v-if="info" :column="3" border size="small">
        <ElDescriptionsItem label="Redis 版本">{{ info.info?.redis_version }}</ElDescriptionsItem>
        <ElDescriptionsItem label="运行模式">{{ info.info?.redis_mode }}</ElDescriptionsItem>
        <ElDescriptionsItem label="端口">{{ info.info?.tcp_port }}</ElDescriptionsItem>
        <ElDescriptionsItem label="已运行天数">{{ info.info?.uptime_in_days }}</ElDescriptionsItem>
        <ElDescriptionsItem label="连接客户端数">{{ info.info?.connected_clients }}</ElDescriptionsItem>
        <ElDescriptionsItem label="已用内存">{{ info.info?.used_memory_human }}</ElDescriptionsItem>
        <ElDescriptionsItem label="总系统内存">{{ info.info?.total_system_memory_human }}</ElDescriptionsItem>
        <ElDescriptionsItem label="最大内存配置">{{ info.info?.maxmemory_human }}</ElDescriptionsItem>
        <ElDescriptionsItem label="key 总数">{{ info.dbSize }}</ElDescriptionsItem>
      </ElDescriptions>
      <ElEmpty v-else description="加载中..." />
    </ElCard>

    <!-- 缓存名称列表 -->
    <ElCard class="info-card" shadow="never">
      <template #header><span>缓存分类</span></template>
      <ElTable :data="cacheNames" border size="small">
        <ElTableColumn label="序号" type="index" width="60" align="center" />
        <ElTableColumn label="缓存名称" prop="name" />
        <ElTableColumn label="键数量" prop="keySize" width="100" align="center" />
        <ElTableColumn label="操作" width="200" align="center">
          <template #default="{ row }">
            <ElButton link type="primary" size="small" @click="loadKeys(row.name)">查看键</ElButton>
            <ElButton link type="danger" size="small" @click="handleClearName(row.name)">清空</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 缓存键列表 -->
    <ElCard v-if="currentName" class="info-card" shadow="never">
      <template #header><span>「{{ currentName }}」的缓存键</span></template>
      <ElTable :data="cacheKeys.map((k) => ({ key: k }))" border size="small" style="width: 50%">
        <ElTableColumn label="缓存键" prop="key" show-overflow-tooltip />
        <ElTableColumn label="操作" width="180" align="center">
          <template #default="{ row }">
            <ElButton link type="primary" size="small" @click="loadValue(row.key)">查看值</ElButton>
            <ElButton link type="danger" size="small" @click="handleClearKey(row.key)">删除</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>

    <!-- 缓存值详情 -->
    <ElCard v-if="cacheValue !== null" class="info-card" shadow="never">
      <template #header><span>「{{ currentKey }}」的缓存值</span></template>
      <pre class="cache-value">{{ typeof cacheValue === 'string' ? cacheValue : JSON.stringify(cacheValue, null, 2) }}</pre>
    </ElCard>
  </div>
</template>

<style scoped>
.cache-page {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cache-toolbar {
  display: flex;
  gap: 8px;
}
.info-card :deep(.el-card__header) {
  font-weight: 600;
}
.cache-value {
  background: var(--el-fill-color-light);
  padding: 12px;
  border-radius: 4px;
  font-size: 12px;
  max-height: 300px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
