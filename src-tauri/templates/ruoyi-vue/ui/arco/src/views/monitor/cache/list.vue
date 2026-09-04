<template>
  <div class="cache-list-page">
    <a-row :gutter="12">
      <!-- 左：缓存名称 -->
      <a-col :xs="24" :md="7" :lg="6">
        <a-card :title="t('monitor.cache.list.cacheName')" class="app-page-card" :bordered="false">
          <template #extra>
            <a-button
              v-hasPermi="['monitor:cache:list']"
              type="text"
              size="small"
              status="danger"
              @click="handleClearAll"
            >
              {{ t('monitor.cache.list.clearAll') }}
            </a-button>
          </template>
          <a-spin :loading="namesLoading" style="display: block">
            <a-empty v-if="names.length === 0" :description="t('monitor.cache.list.noNames')" />
            <div v-else class="cache-list-page__names">
              <div
                v-for="item in names"
                :key="item.cacheName"
                class="cache-list-page__name-item"
                :class="{ 'cache-list-page__name-item--active': item.cacheName === activeName }"
                @click="selectName(item.cacheName)"
              >
                <span class="cache-list-page__name-text">{{ item.cacheName }}</span>
                <span class="cache-list-page__name-remark">{{ item.remark }}</span>
              </div>
            </div>
          </a-spin>
        </a-card>
      </a-col>

      <!-- 中：缓存键列表 -->
      <a-col :xs="24" :md="9" :lg="9">
        <a-card
          :title="activeName ? t('monitor.cache.list.keyListWithName', { name: activeName }) : t('monitor.cache.list.keyList')"
          class="app-page-card"
          :bordered="false"
        >
          <template #extra>
            <a-button
              v-hasPermi="['monitor:cache:list']"
              type="text"
              size="small"
              status="danger"
              :disabled="!activeName"
              @click="handleClearName"
            >
              {{ t('monitor.cache.list.clearName') }}
            </a-button>
          </template>
          <a-input
            v-model.trim="keyFilter"
            :placeholder="t('monitor.cache.list.filterPlaceholder')"
            allow-clear
            class="cache-list-page__filter"
          >
            <template #prefix><IconSearch /></template>
          </a-input>
          <a-spin :loading="keysLoading" style="display: block">
            <a-empty v-if="filteredKeys.length === 0" :description="t('monitor.cache.list.noKeys')" />
            <div v-else class="cache-list-page__keys">
              <div
                v-for="key in filteredKeys"
                :key="key"
                class="cache-list-page__key-item"
                :class="{ 'cache-list-page__key-item--active': key === activeKey }"
                :title="key"
                @click="selectKey(key)"
              >
                {{ key }}
              </div>
            </div>
          </a-spin>
        </a-card>
      </a-col>

      <!-- 右：缓存内容 -->
      <a-col :xs="24" :md="8" :lg="9">
        <a-card :title="t('monitor.cache.list.cacheValue')" class="app-page-card" :bordered="false">
          <template #extra>
            <a-button
              v-hasPermi="['monitor:cache:list']"
              type="text"
              size="small"
              status="danger"
              :disabled="!activeName && !activeKey"
              @click="handleClearKey"
            >
              {{ t('monitor.cache.list.clearKey') }}
            </a-button>
          </template>
          <a-empty v-if="!valueResult" :description="t('monitor.cache.list.selectKeyTip')" />
          <template v-else>
            <a-descriptions :column="1" size="medium" bordered>
              <a-descriptions-item :label="t('monitor.cache.list.cacheName')">{{ valueResult.cacheName }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.cache.list.cacheKey')">
                <span class="pre-wrap">{{ valueResult.cacheKey }}</span>
              </a-descriptions-item>
            </a-descriptions>
            <pre class="cache-list-page__value">{{ valueResult.cacheValue ?? t('monitor.cache.list.emptyValue') }}</pre>
          </template>
        </a-card>
      </a-col>
    </a-row>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Message, Modal } from '@arco-design/web-vue'
import { IconSearch } from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import {
  clearCacheAll,
  clearCacheKey,
  clearCacheName,
  getCacheKeys,
  getCacheNames,
  getCacheValue
} from '@/api/monitor/cache'
import type { CacheNameItem, CacheValueResult } from '@/api/monitor/cache'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'CacheList' })

const { t } = useI18n()

/* ---------- 左栏：缓存名称 ---------- */
const names = ref<CacheNameItem[]>([])
const namesLoading = ref(false)
const activeName = ref('')

async function loadNames(): Promise<void> {
  namesLoading.value = true
  try {
    names.value = await getCacheNames()
  } finally {
    namesLoading.value = false
  }
}

function selectName(cacheName: string): void {
  if (cacheName === activeName.value) return
  activeName.value = cacheName
  activeKey.value = ''
  valueResult.value = null
  keyFilter.value = ''
  void loadKeys()
}

/* ---------- 中栏：缓存键 ---------- */
const keys = ref<string[]>([])
const keysLoading = ref(false)
const keyFilter = ref('')
const activeKey = ref('')

async function loadKeys(): Promise<void> {
  if (!activeName.value) {
    keys.value = []
    return
  }
  keysLoading.value = true
  try {
    keys.value = await getCacheKeys(activeName.value)
  } finally {
    keysLoading.value = false
  }
}

const filteredKeys = computed(() => {
  const keyword = keyFilter.value.toLowerCase()
  if (!keyword) return keys.value
  return keys.value.filter((key) => key.toLowerCase().includes(keyword))
})

function selectKey(cacheKey: string): void {
  activeKey.value = cacheKey
  void loadValue()
}

/* ---------- 右栏：缓存内容 ---------- */
const valueResult = ref<CacheValueResult | null>(null)

async function loadValue(): Promise<void> {
  if (!activeName.value || !activeKey.value) return
  valueResult.value = await getCacheValue(activeName.value, activeKey.value)
}

/* ---------- 清理操作 ---------- */
/** 清理指定缓存名称（含尾冒号前缀，如 sys_dict:） */
function handleClearName(): void {
  if (!activeName.value) {
    Message.warning(t('monitor.cache.list.selectNameFirst'))
    return
  }
  Modal.confirm({
    title: t('monitor.cache.list.cleanConfirmTitle'),
    content: t('monitor.cache.list.clearNameConfirm', { name: activeName.value }),
    hideCancel: false,
    onOk: async () => {
      try {
        await clearCacheName(activeName.value)
        Message.success(t('monitor.cache.list.cleanNameSuccess'))
        activeKey.value = ''
        valueResult.value = null
        await loadKeys()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/** 清理当前选中缓存键 */
function handleClearKey(): void {
  if (!activeName.value || !activeKey.value) {
    Message.warning(t('monitor.cache.list.selectKeyFirst'))
    return
  }
  Modal.confirm({
    title: t('monitor.cache.list.cleanConfirmTitle'),
    content: t('monitor.cache.list.clearKeyConfirm', { key: activeKey.value }),
    hideCancel: false,
    onOk: async () => {
      try {
        await clearCacheKey(activeKey.value)
        Message.success(t('monitor.cache.list.cleanKeySuccess'))
        activeKey.value = ''
        valueResult.value = null
        await loadKeys()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/** 清理全部缓存（高危操作，单独确认文案） */
function handleClearAll(): void {
  Modal.confirm({
    title: t('common.cleanConfirm'),
    content: t('monitor.cache.list.clearAllConfirm'),
    hideCancel: false,
    onOk: async () => {
      try {
        await clearCacheAll()
        Message.success(t('monitor.cache.list.clearAllSuccess'))
        activeName.value = ''
        activeKey.value = ''
        valueResult.value = null
        keys.value = []
        await loadNames()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/* ---------- 初始化 ---------- */
void loadNames()
</script>

<style scoped>
.cache-list-page__names {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cache-list-page__name-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.cache-list-page__name-item:hover {
  background-color: var(--color-fill-2);
}

.cache-list-page__name-item--active {
  background-color: var(--color-primary-light-1);
}

.cache-list-page__name-text {
  font-size: 13px;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cache-list-page__name-remark {
  font-size: 12px;
  color: var(--color-text-3);
  flex-shrink: 0;
}

.cache-list-page__filter {
  margin-bottom: 10px;
}

.cache-list-page__keys {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 480px;
  overflow: auto;
}

.cache-list-page__key-item {
  padding: 7px 10px;
  border-radius: 4px;
  font-size: 13px;
  color: var(--color-text-1);
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: background-color 0.2s;
}

.cache-list-page__key-item:hover {
  background-color: var(--color-fill-2);
}

.cache-list-page__key-item--active {
  background-color: var(--color-primary-light-1);
}

.cache-list-page__value {
  margin: 12px 0 0;
  padding: 10px;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 12px;
  line-height: 1.6;
  background-color: var(--color-fill-2);
  border-radius: 4px;
}

.pre-wrap {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
