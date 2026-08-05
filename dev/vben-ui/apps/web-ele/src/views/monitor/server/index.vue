<script setup lang="ts">
import { onMounted, ref } from 'vue';

import {
  ElButton,
  ElCard,
  ElCol,
  ElDescriptions,
  ElDescriptionsItem,
  ElIcon,
  ElProgress,
  ElRow,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import {
  Cpu,
  Monitor,
  Refresh,
  Tickets,
  Coffee,
  DataAnalysis,
} from '@element-plus/icons-vue';

import { getServer, type ServerInfo } from '#/api/monitor/server';

defineOptions({ name: 'MonitorServer' });

const server = ref<ServerInfo | null>(null);
const loading = ref(false);

async function getList() {
  loading.value = true;
  try {
    const res = await getServer();
    server.value = res.data;
  } finally {
    loading.value = false;
  }
}

// 进度条颜色根据使用率变化
function progressColor(usage: number) {
  if (usage > 80) return '#f56c6c';
  if (usage > 60) return '#e6a23c';
  return '#67c23a';
}

onMounted(getList);
</script>

<template>
  <div v-loading="loading" class="server-page">
    <div class="server-header">
      <ElButton :icon="Refresh" size="small" @click="getList">刷新</ElButton>
    </div>

    <ElRow :gutter="16">
      <!-- CPU -->
      <ElCol :xs="24" :sm="12">
        <ElCard shadow="never" class="monitor-card">
          <template #header>
            <div class="card-header">
              <ElIcon class="header-icon"><Cpu /></ElIcon>
              <span>CPU</span>
            </div>
          </template>
          <div v-if="server?.cpu" class="progress-section">
            <div class="progress-item">
              <div class="progress-label">
                <span>使用率</span>
                <span :class="{ 'text-danger': server.cpu.used > 80 }">
                  {{ server.cpu.used }}%
                </span>
              </div>
              <ElProgress
                :percentage="Number(server.cpu.used || 0)"
                :color="progressColor(server.cpu.used)"
                :stroke-width="14"
              />
            </div>
            <ElDescriptions :column="2" border size="small" class="info-desc">
              <ElDescriptionsItem label="核心数">
                {{ server.cpu.cpuNum }}
              </ElDescriptionsItem>
              <ElDescriptionsItem label="系统使用率">
                {{ server.cpu.sys }}%
              </ElDescriptionsItem>
              <ElDescriptionsItem label="当前空闲率">
                {{ server.cpu.free }}%
              </ElDescriptionsItem>
              <ElDescriptionsItem label="等待率">
                {{ server.cpu.wait }}%
              </ElDescriptionsItem>
            </ElDescriptions>
          </div>
        </ElCard>
      </ElCol>

      <!-- 内存 -->
      <ElCol :xs="24" :sm="12">
        <ElCard shadow="never" class="monitor-card">
          <template #header>
            <div class="card-header">
              <ElIcon class="header-icon"><Tickets /></ElIcon>
              <span>内存</span>
            </div>
          </template>
          <div v-if="server?.mem" class="progress-section">
            <div class="progress-item">
              <div class="progress-label">
                <span>内存使用率</span>
                <span :class="{ 'text-danger': server.mem.usage > 80 }">
                  {{ server.mem.usage }}%
                </span>
              </div>
              <ElProgress
                :percentage="Number(server.mem.usage || 0)"
                :color="progressColor(server.mem.usage)"
                :stroke-width="14"
              />
            </div>
            <ElDescriptions :column="2" border size="small" class="info-desc">
              <ElDescriptionsItem label="总内存">
                {{ server.mem.total }} G
              </ElDescriptionsItem>
              <ElDescriptionsItem label="已用内存">
                {{ server.mem.used }} G
              </ElDescriptionsItem>
              <ElDescriptionsItem label="剩余内存">
                {{ server.mem.free }} G
              </ElDescriptionsItem>
              <ElDescriptionsItem label="使用率">
                {{ server.mem.usage }}%
              </ElDescriptionsItem>
            </ElDescriptions>
          </div>
        </ElCard>
      </ElCol>

      <!-- JVM -->
      <ElCol :xs="24" :sm="12">
        <ElCard shadow="never" class="monitor-card">
          <template #header>
            <div class="card-header">
              <ElIcon class="header-icon"><Coffee /></ElIcon>
              <span>JVM</span>
            </div>
          </template>
          <div v-if="server?.jvm" class="progress-section">
            <div class="progress-item">
              <div class="progress-label">
                <span>JVM 使用率</span>
                <span :class="{ 'text-danger': server.jvm.usage > 80 }">
                  {{ server.jvm.usage }}%
                </span>
              </div>
              <ElProgress
                :percentage="Number(server.jvm.usage || 0)"
                :color="progressColor(server.jvm.usage)"
                :stroke-width="14"
              />
            </div>
            <ElDescriptions :column="2" border size="small" class="info-desc">
              <ElDescriptionsItem label="总内存">
                {{ server.jvm.total }} M
              </ElDescriptionsItem>
              <ElDescriptionsItem label="已用内存">
                {{ server.jvm.used }} M
              </ElDescriptionsItem>
              <ElDescriptionsItem label="剩余内存">
                {{ server.jvm.free }} M
              </ElDescriptionsItem>
              <ElDescriptionsItem label="使用率">
                {{ server.jvm.usage }}%
              </ElDescriptionsItem>
            </ElDescriptions>
          </div>
        </ElCard>
      </ElCol>

      <!-- 磁盘概览 -->
      <ElCol :xs="24" :sm="12">
        <ElCard shadow="never" class="monitor-card">
          <template #header>
            <div class="card-header">
              <ElIcon class="header-icon"><DataAnalysis /></ElIcon>
              <span>磁盘概览</span>
            </div>
          </template>
          <div class="disk-overview">
            <div
              v-for="(sysFile, index) in server?.sysFiles ?? []"
              :key="index"
              class="disk-item"
            >
              <div class="progress-label">
                <span>{{ sysFile.dirName }}（{{ sysFile.typeName }}）</span>
                <span :class="{ 'text-danger': sysFile.usage > 80 }">
                  {{ sysFile.usage }}%
                </span>
              </div>
              <ElProgress
                :percentage="Number(sysFile.usage || 0)"
                :color="progressColor(sysFile.usage)"
                :stroke-width="12"
              />
              <div class="disk-detail">
                总 {{ sysFile.total }} · 已用 {{ sysFile.used }} · 可用
                {{ sysFile.free }}
              </div>
            </div>
          </div>
        </ElCard>
      </ElCol>
    </ElRow>

    <!-- 服务器信息 -->
    <ElCard shadow="never" class="info-card">
      <template #header>
        <div class="card-header">
          <ElIcon class="header-icon"><Monitor /></ElIcon>
          <span>服务器信息</span>
        </div>
      </template>
      <ElDescriptions :column="2" border size="default" v-if="server?.sys">
        <ElDescriptionsItem label="服务器名称">
          {{ server.sys.computerName }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="操作系统">
          {{ server.sys.osName }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="服务器IP">
          {{ server.sys.computerIp }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="系统架构">
          {{ server.sys.osArch }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="项目路径" :span="2">
          {{ server.sys.userDir }}
        </ElDescriptionsItem>
      </ElDescriptions>
    </ElCard>

    <!-- Java 虚拟机信息 -->
    <ElCard shadow="never" class="info-card">
      <template #header>
        <div class="card-header">
          <ElIcon class="header-icon"><Coffee /></ElIcon>
          <span>Java 虚拟机信息</span>
        </div>
      </template>
      <ElDescriptions :column="2" border size="default" v-if="server?.jvm">
        <ElDescriptionsItem label="Java 名称">
          {{ server.jvm.name }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="Java 版本">
          {{ server.jvm.version }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="启动时间">
          {{ server.jvm.startTime }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="运行时长">
          {{ server.jvm.runTime }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="安装路径" :span="2">
          {{ server.jvm.home }}
        </ElDescriptionsItem>
        <ElDescriptionsItem label="运行参数" :span="2">
          <span class="mono">{{ server.jvm.inputArgs }}</span>
        </ElDescriptionsItem>
      </ElDescriptions>
    </ElCard>

    <!-- 磁盘状态详细表格 -->
    <ElCard shadow="never" class="info-card">
      <template #header>
        <div class="card-header">
          <ElIcon class="header-icon"><DataAnalysis /></ElIcon>
          <span>磁盘状态</span>
        </div>
      </template>
      <ElTable :data="server?.sysFiles ?? []" border size="small">
        <ElTableColumn label="盘符路径" align="center" prop="dirName" />
        <ElTableColumn label="文件系统" align="center" prop="sysTypeName" />
        <ElTableColumn label="盘符类型" align="center" prop="typeName" />
        <ElTableColumn label="总大小" align="center" prop="total" />
        <ElTableColumn label="可用大小" align="center" prop="free" />
        <ElTableColumn label="已用大小" align="center" prop="used" />
        <ElTableColumn label="已用百分比" align="center" prop="usage">
          <template #default="{ row }">
            <span :class="{ 'text-danger': row.usage > 80 }">
              {{ row.usage }}%
            </span>
          </template>
        </ElTableColumn>
      </ElTable>
    </ElCard>
  </div>
</template>

<style scoped>
.server-page {
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 16px;
  height: 100%;
  overflow: auto;
}

.server-header {
  display: flex;
  justify-content: flex-end;
}

.monitor-card,
.info-card {
  margin-bottom: 0;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 14px;
}

.header-icon {
  font-size: 16px;
  color: var(--el-color-primary);
}

.progress-section {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.progress-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.progress-label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  font-weight: 500;
}

.info-desc {
  margin-top: 4px;
}

.disk-overview {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.disk-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.disk-detail {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.text-danger {
  color: var(--el-color-danger);
  font-weight: 600;
}

.mono {
  font-family: 'Consolas', 'Menlo', monospace;
  font-size: 12px;
  word-break: break-all;
}
</style>
