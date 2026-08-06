<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue';

import {
  ElButton,
  ElCard,
  ElCol,
  ElEmpty,
  ElRow,
} from 'element-plus';
import { Refresh } from '@element-plus/icons-vue';
import {
  EchartsUI,
  type EchartsUIType,
  useEcharts,
} from '@vben/plugins/echarts';

import { getCache, type CacheInfo } from '#/api/monitor/cache';

defineOptions({ name: 'MonitorCache' });

const info = ref<CacheInfo | null>(null);
const loading = ref(false);

const commandStatsRef = ref<EchartsUIType>();
const usedMemoryRef = ref<EchartsUIType>();
const { renderEcharts: renderCommandStats } = useEcharts(commandStatsRef);
const { renderEcharts: renderUsedMemory } = useEcharts(usedMemoryRef);

/**
 * 拉取 Redis 监控数据并绘制图表。
 * 须先关闭 loading 再渲染：v-loading 遮罩期间容器尺寸可能为 0，仪表盘会画不出。
 */
async function getList() {
  loading.value = true;
  try {
    info.value = await getCache();
  } finally {
    loading.value = false;
  }
  await nextTick();
  renderCharts();
}

/** 渲染命令统计饼图与内存仪表盘 */
function renderCharts() {
  const cache = info.value;
  if (!cache?.info) {
    return;
  }

  // 后端 commandStats.value 为字符串（calls 次数），饼图需数值才能正确计算占比
  const commandStatsData = (cache.commandStats ?? []).map((item) => ({
    name: item.name,
    value: Number(item.value) || 0,
  }));

  renderCommandStats({
    tooltip: {
      trigger: 'item',
      formatter: '{a} <br/>{b} : {c} ({d}%)',
    },
    series: [
      {
        name: '命令',
        type: 'pie',
        roseType: 'radius',
        radius: [15, 95],
        center: ['50%', '38%'],
        data: commandStatsData,
        animationEasing: 'cubicInOut',
        animationDuration: 1000,
      },
    ],
  });

  const usedMemoryHuman = cache.info.used_memory_human ?? '0';
  renderUsedMemory({
    tooltip: {
      formatter: `{b} <br/>{a} : ${usedMemoryHuman}`,
    },
    series: [
      {
        name: '峰值',
        type: 'gauge',
        min: 0,
        max: 1000,
        detail: {
          formatter: usedMemoryHuman,
        },
        data: [
          {
            value: Number.parseFloat(usedMemoryHuman) || 0,
            name: '内存消耗',
          },
        ],
      },
    ],
  });
}

onMounted(getList);
onUnmounted(() => {
  info.value = null;
});
</script>

<template>
  <div v-loading="loading" class="cache-monitor-page">
    <div class="page-header">
      <ElButton :icon="Refresh" size="small" @click="getList">刷新</ElButton>
    </div>

    <ElRow :gutter="10">
      <ElCol :span="24">
        <ElCard shadow="never" class="info-card">
          <template #header>
            <span class="card-title">基本信息</span>
          </template>
          <div v-if="info?.info" class="info-table-wrap">
            <table class="info-table">
              <tbody>
                <tr>
                  <td class="label">Redis版本</td>
                  <td>{{ info.info.redis_version }}</td>
                  <td class="label">运行模式</td>
                  <td>
                    {{ info.info.redis_mode === 'standalone' ? '单机' : '集群' }}
                  </td>
                  <td class="label">端口</td>
                  <td>{{ info.info.tcp_port }}</td>
                  <td class="label">客户端数</td>
                  <td>{{ info.info.connected_clients }}</td>
                </tr>
                <tr>
                  <td class="label">运行时间(天)</td>
                  <td>{{ info.info.uptime_in_days }}</td>
                  <td class="label">使用内存</td>
                  <td>{{ info.info.used_memory_human }}</td>
                  <td class="label">使用CPU</td>
                  <td>
                    {{
                      Number.parseFloat(info.info.used_cpu_user_children || 0).toFixed(2)
                    }}
                  </td>
                  <td class="label">内存配置</td>
                  <td>{{ info.info.maxmemory_human }}</td>
                </tr>
                <tr>
                  <td class="label">AOF是否开启</td>
                  <td>{{ info.info.aof_enabled === '0' ? '否' : '是' }}</td>
                  <td class="label">RDB是否成功</td>
                  <td>{{ info.info.rdb_last_bgsave_status }}</td>
                  <td class="label">Key数量</td>
                  <td>{{ info.dbSize }}</td>
                  <td class="label">网络入口/出口</td>
                  <td>
                    {{ info.info.instantaneous_input_kbps }}kps /
                    {{ info.info.instantaneous_output_kbps }}kps
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <ElEmpty v-else description="暂无数据" />
        </ElCard>
      </ElCol>

      <ElCol :xs="24" :md="12">
        <ElCard shadow="never" class="chart-card">
          <template #header>
            <span class="card-title">命令统计</span>
          </template>
          <EchartsUI ref="commandStatsRef" height="420px" />
        </ElCard>
      </ElCol>

      <ElCol :xs="24" :md="12">
        <ElCard shadow="never" class="chart-card">
          <template #header>
            <span class="card-title">内存信息</span>
          </template>
          <EchartsUI ref="usedMemoryRef" height="420px" />
        </ElCard>
      </ElCol>
    </ElRow>
  </div>
</template>

<style scoped>
.cache-monitor-page {
  padding: 12px;
}

.page-header {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 10px;
}

.info-card,
.chart-card {
  margin-bottom: 10px;
}

.card-title {
  font-weight: 600;
}

.info-table-wrap {
  overflow-x: auto;
}

.info-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.info-table td {
  padding: 10px 12px;
  border: 1px solid var(--el-border-color-lighter);
}

.info-table .label {
  width: 110px;
  background: var(--el-fill-color-light);
  color: var(--el-text-color-secondary);
  white-space: nowrap;
}
</style>
