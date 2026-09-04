<template>
  <div class="cache-page">
    <a-alert v-if="loadError" type="error" class="cache-page__alert">{{ loadError }}</a-alert>

    <!-- 统计卡：Redis 版本 / key 数量 / 命令统计条数 -->
    <a-row v-loading="loading" :gutter="12">
      <a-col :xs="24" :sm="8">
        <a-card class="app-page-card" :bordered="false">
          <a-statistic :title="t('monitor.cache.redisVersion')">
            <template #value>{{ infoField('redis_version') || '-' }}</template>
          </a-statistic>
        </a-card>
      </a-col>
      <a-col :xs="24" :sm="8">
        <a-card class="app-page-card" :bordered="false">
          <a-statistic :title="t('monitor.cache.keyCount')" :value="dbSize" />
        </a-card>
      </a-col>
      <a-col :xs="24" :sm="8">
        <a-card class="app-page-card" :bordered="false">
          <a-statistic :title="t('monitor.cache.commandTotal')" :value="commandTotal" />
        </a-card>
      </a-col>
    </a-row>

    <!-- 命令统计横向条形排行 -->
    <a-card :title="t('monitor.cache.commandRanking')" class="app-page-card cache-page__section" :bordered="false">
      <template #extra>
        <a-button type="text" size="small" :loading="loading" @click="loadData">
          <template #icon><IconRefresh /></template>
          {{ t('common.refresh') }}
        </a-button>
      </template>
      <a-empty v-if="commandBars.length === 0" :description="t('monitor.cache.noCommandData')" />
      <div v-else class="cache-page__bars">
        <div v-for="item in commandBars" :key="item.name" class="cache-page__bar-row">
          <span class="cache-page__bar-name">{{ item.name }}</span>
          <a-progress
            class="cache-page__bar-track"
            :percent="item.percent"
            :show-text="false"
            size="small"
            color="rgb(var(--primary-6))"
          />
          <span class="cache-page__bar-value">{{ item.count }}</span>
        </div>
      </div>
    </a-card>

    <!-- Redis 信息 -->
    <a-card :title="t('monitor.cache.redisInfo')" class="app-page-card cache-page__section" :bordered="false">
      <a-empty v-if="infoEntries.length === 0" :description="t('monitor.cache.noInfoData')" />
      <a-descriptions v-else :column="{ xs: 1, md: 2, lg: 3 }" bordered size="medium">
        <a-descriptions-item v-for="entry in infoEntries" :key="entry.key" :label="entry.key">
          {{ entry.value }}
        </a-descriptions-item>
      </a-descriptions>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { IconRefresh } from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import { getCacheInfo } from '@/api/monitor/cache'
import type { CacheInfoResult } from '@/api/monitor/cache'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Cache' })

const { t } = useI18n()

const data = ref<CacheInfoResult | null>(null)
const loading = ref(false)
const loadError = ref('')

async function loadData(): Promise<void> {
  loading.value = true
  loadError.value = ''
  try {
    data.value = await getCacheInfo()
  } catch {
    // 错误提示已由响应拦截器统一弹出；页面给出兜底空态
    loadError.value = t('monitor.cache.loadFailed')
  } finally {
    loading.value = false
  }
}

const dbSize = computed<number>(() => data.value?.dbSize ?? 0)

/** 命令调用次数总计（value 为字符串形式的数字） */
const commandTotal = computed<number>(() =>
  (data.value?.commandStats ?? []).reduce((sum, item) => sum + toCount(item.value), 0)
)

/** 条形排行：按次数倒序取前 15，percent 按最大值归一化 */
const commandBars = computed(() => {
  const stats = [...(data.value?.commandStats ?? [])]
    .map((item) => ({ name: item.name, count: toCount(item.value) }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 15)
  const max = stats.reduce((current, item) => Math.max(current, item.count), 0)
  return stats.map((item) => ({
    ...item,
    percent: max > 0 ? item.count / max : 0
  }))
})

/** Redis INFO 关注键（按存在与否渲染，键序即展示序） */
const INFO_KEYS = [
  'redis_version',
  'redis_mode',
  'os',
  'run_id',
  'tcp_port',
  'uptime_in_days',
  'connected_clients',
  'used_memory_human',
  'used_memory_peak_human',
  'maxmemory_human',
  'mem_allocator',
  'total_connections_received',
  'total_commands_processed',
  'instantaneous_ops_per_sec',
  'expired_keys',
  'evicted_keys',
  'keyspace_hits',
  'keyspace_misses',
  'aof_enabled',
  'rdb_last_save_time'
]

const infoEntries = computed(() => {
  const info = data.value?.info ?? {}
  const entries = INFO_KEYS.filter((key) => info[key] !== undefined).map((key) => ({
    key,
    value: info[key]
  }))
  // INFO_KEYS 未命中时兜底展示前 12 个键，保证页面不空白
  if (entries.length === 0) {
    return Object.keys(info)
      .slice(0, 12)
      .map((key) => ({ key, value: info[key] }))
  }
  return entries
})

/** 关注键之外的其余键折叠展示 */
function infoField(key: string): string {
  return data.value?.info?.[key] ?? ''
}

function toCount(value: string | number): number {
  const count = Number(value)
  return Number.isNaN(count) ? 0 : count
}

onMounted(() => {
  void loadData()
})
</script>

<style scoped>
.cache-page__alert {
  margin-bottom: 12px;
}

.cache-page__section {
  margin-top: 12px;
}

.cache-page__bars {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.cache-page__bar-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.cache-page__bar-name {
  width: 160px;
  text-align: right;
  font-size: 13px;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.cache-page__bar-track {
  flex: 1;
}

.cache-page__bar-value {
  width: 72px;
  font-size: 13px;
  color: var(--color-text-2);
  text-align: left;
  flex-shrink: 0;
}
</style>
