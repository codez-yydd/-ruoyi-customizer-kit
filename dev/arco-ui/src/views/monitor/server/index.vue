<template>
  <div class="server-page">
    <a-alert v-if="loadError" type="error" class="server-page__alert">{{ loadError }}</a-alert>

    <template v-if="info">
      <!-- CPU / 内存 -->
      <a-row :gutter="12">
        <a-col :xs="24" :md="12">
          <a-card :title="t('monitor.server.cpu')" class="app-page-card" :bordered="false">
            <template #extra>
              <a-button type="text" size="small" @click="loadData">
                <template #icon><IconRefresh /></template>
                {{ t('common.refresh') }}
              </a-button>
            </template>
            <div class="server-page__rings">
              <div v-for="item in cpuRings" :key="item.label" class="server-page__ring">
                <a-progress
                  type="circle"
                  size="mini"
                  :percent="toPercent(item.value)"
                  :color="ringColor(item.value)"
                />
                <div class="server-page__ring-label">{{ item.label }}</div>
                <div class="server-page__ring-value">{{ toFixed(item.value) }}%</div>
              </div>
            </div>
            <div class="server-page__foot">{{ t('monitor.server.coreCount', { count: info.cpu.cpuNum }) }}</div>
          </a-card>
        </a-col>
        <a-col :xs="24" :md="12">
          <a-card :title="t('monitor.server.mem')" class="app-page-card" :bordered="false">
            <div class="server-page__rings">
              <div class="server-page__ring">
                <a-progress
                  type="circle"
                  size="mini"
                  :percent="toPercent(info.mem.usage)"
                  :color="ringColor(info.mem.usage)"
                />
                <div class="server-page__ring-label">{{ t('monitor.server.usage') }}</div>
                <div class="server-page__ring-value">{{ toFixed(info.mem.usage) }}%</div>
              </div>
              <div class="server-page__mem-stats">
                <div class="server-page__mem-item">
                  <span class="server-page__mem-num">{{ toFixed(info.mem.total) }}G</span>
                  <span class="server-page__mem-label">{{ t('monitor.server.memTotal') }}</span>
                </div>
                <div class="server-page__mem-item">
                  <span class="server-page__mem-num">{{ toFixed(info.mem.used) }}G</span>
                  <span class="server-page__mem-label">{{ t('monitor.server.memUsed') }}</span>
                </div>
                <div class="server-page__mem-item">
                  <span class="server-page__mem-num">{{ toFixed(info.mem.free) }}G</span>
                  <span class="server-page__mem-label">{{ t('monitor.server.memFree') }}</span>
                </div>
              </div>
            </div>
          </a-card>
        </a-col>
      </a-row>

      <!-- 服务器信息 / JVM 信息 -->
      <a-row :gutter="12" class="server-page__row">
        <a-col :xs="24" :md="12">
          <a-card :title="t('monitor.server.serverInfo')" class="app-page-card" :bordered="false">
            <a-descriptions :column="1" size="medium">
              <a-descriptions-item :label="t('monitor.server.computerName')">{{ info.sys.computerName }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.osName')">{{ info.sys.osName }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.serverIp')">{{ info.sys.computerIp }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.osArch')">{{ info.sys.osArch }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.projectPath')">
                <span class="pre-wrap">{{ info.sys.userDir }}</span>
              </a-descriptions-item>
            </a-descriptions>
          </a-card>
        </a-col>
        <a-col :xs="24" :md="12">
          <a-card :title="t('monitor.server.jvmInfo')" class="app-page-card" :bordered="false">
            <div class="server-page__jvm-head">
              <a-progress
                type="circle"
                size="mini"
                :percent="toPercent(info.jvm.usage)"
                :color="ringColor(info.jvm.usage)"
              />
              <div>
                <div class="server-page__ring-label">{{ t('monitor.server.jvmUsage') }}</div>
                <div class="server-page__ring-value">{{ toFixed(info.jvm.usage) }}%</div>
              </div>
            </div>
            <a-descriptions :column="1" size="medium">
              <a-descriptions-item :label="t('monitor.server.javaVersion')">{{ info.jvm.version }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.startTime')">{{ info.jvm.startTime }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.runTime')">{{ info.jvm.runTime }}</a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.initMem')">
                {{ t('monitor.server.initMemValue', {
                  total: toFixed(info.jvm.total),
                  used: toFixed(info.jvm.used),
                  free: toFixed(info.jvm.free),
                  max: toFixed(info.jvm.max)
                }) }}
              </a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.installPath')">
                <span class="pre-wrap">{{ info.jvm.home }}</span>
              </a-descriptions-item>
              <a-descriptions-item :label="t('monitor.server.startArgs')">
                <a-collapse :bordered="false" class="server-page__args">
                  <a-collapse-item :header="truncate(info.jvm.inputArgs, 60)" key="args">
                    <pre class="args-pre">{{ info.jvm.inputArgs }}</pre>
                  </a-collapse-item>
                </a-collapse>
              </a-descriptions-item>
            </a-descriptions>
          </a-card>
        </a-col>
      </a-row>

      <!-- 磁盘状态 -->
      <a-card :title="t('monitor.server.diskStatus')" class="app-page-card server-page__row" :bordered="false">
        <a-table :data="info.sysFiles" :pagination="false" row-key="dirName">
          <template #columns>
            <a-table-column :title="t('monitor.server.dirName')" data-index="dirName" />
            <a-table-column :title="t('monitor.server.fsType')" data-index="sysTypeName" />
            <a-table-column :title="t('monitor.server.typeName')" data-index="typeName" />
            <a-table-column :title="t('monitor.server.diskTotal')" data-index="total" />
            <a-table-column :title="t('monitor.server.diskFree')" data-index="free" />
            <a-table-column :title="t('monitor.server.diskUsed')" data-index="used" />
            <a-table-column :title="t('monitor.server.diskUsedPercent')">
              <template #cell="{ record }">
                <a-space class="server-page__disk-usage">
                  <a-progress
                    :percent="toPercent((record as ServerFile).usage)"
                    :color="ringColor((record as ServerFile).usage)"
                    size="small"
                    :show-text="false"
                  />
                  <span>{{ toFixed((record as ServerFile).usage) }}%</span>
                </a-space>
              </template>
            </a-table-column>
          </template>
        </a-table>
      </a-card>
    </template>

    <a-card v-else-if="!loading" :title="t('monitor.server.serverMonitor')" class="app-page-card" :bordered="false">
      <a-empty :description="t('monitor.server.noData')" />
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { IconRefresh } from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import { getServerInfo } from '@/api/monitor/server'
import type { ServerFile, ServerInfo } from '@/api/monitor/server'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Server' })

const { t } = useI18n()

const info = ref<ServerInfo | null>(null)
const loading = ref(false)
const loadError = ref('')

async function loadData(): Promise<void> {
  loading.value = true
  loadError.value = ''
  try {
    info.value = await getServerInfo()
  } catch {
    // 错误提示已由响应拦截器统一弹出；页面给出兜底空态
    loadError.value = t('monitor.server.loadFailed')
  } finally {
    loading.value = false
  }
}

/** CPU 环形指标组（空闲与总使用率互补冗余，不再单独展示） */
const cpuRings = computed(() => {
  const cpu = info.value?.cpu
  if (!cpu) return []
  return [
    { label: t('monitor.server.totalUsage'), value: cpu.total },
    { label: t('monitor.server.user'), value: cpu.used },
    { label: t('monitor.server.sys'), value: cpu.sys },
    { label: t('monitor.server.wait'), value: cpu.wait }
  ]
})

/** 百分比 -> a-progress percent（0~1） */
function toPercent(value?: number): number {
  if (value == null || Number.isNaN(value)) return 0
  return Math.min(1, Math.max(0, value / 100))
}

function toFixed(value?: number): string {
  return value == null ? '-' : Number(value).toFixed(2)
}

/** 使用率超过阈值换色（<60 主色、<80 橙、>=80 红） */
function ringColor(value?: number): string {
  if (value == null) return 'blue'
  if (value >= 80) return 'rgb(var(--red-6))'
  if (value >= 60) return 'rgb(var(--orange-6))'
  return 'rgb(var(--primary-6))'
}

function truncate(text?: string, length = 60): string {
  if (!text) return '-'
  return text.length > length ? `${text.slice(0, length)}...` : text
}

onMounted(() => {
  void loadData()
})
</script>

<style scoped>
.server-page {
  display: flex;
  flex-direction: column;
}

.server-page__alert {
  margin-bottom: 12px;
}

.server-page__row {
  margin-top: 12px;
}

.server-page__rings {
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.server-page__ring {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.server-page__ring-label {
  font-size: 12px;
  color: var(--color-text-2);
}

.server-page__ring-value {
  font-size: 12px;
  color: var(--color-text-1);
}

.server-page__foot {
  margin-top: 12px;
  font-size: 13px;
  color: var(--color-text-2);
}

.server-page__mem-stats {
  flex: 1;
  display: flex;
  justify-content: space-around;
  min-width: 220px;
}

.server-page__mem-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.server-page__mem-num {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-1);
}

.server-page__mem-label {
  font-size: 12px;
  color: var(--color-text-3);
}

.server-page__jvm-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.server-page__args {
  margin-top: 4px;
}

.args-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 12px;
  color: var(--color-text-2);
}

.pre-wrap {
  white-space: pre-wrap;
  word-break: break-all;
}

.server-page__disk-usage {
  display: inline-flex;
  width: 100%;
}

.server-page__disk-usage > :first-child {
  flex: 1;
}
</style>
