<template>
  <div class="job-log-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="jobLogId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="jobName" :label="t('monitor.jobLog.jobName')">
            <a-input
              v-model.trim="queryParams.jobName"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.jobLog.jobName') })"
              allow-clear
              style="width: 150px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="jobGroup" :label="t('monitor.jobLog.jobGroup')">
            <a-select
              v-model="queryParams.jobGroup"
              :options="jobGroupOptions"
              :placeholder="t('monitor.jobLog.jobGroup')"
              allow-clear
              style="width: 140px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item field="status" :label="t('monitor.jobLog.status')">
            <a-select
              v-model="queryParams.status"
              :options="statusOptions"
              :placeholder="t('monitor.jobLog.status')"
              allow-clear
              style="width: 130px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item :label="t('monitor.jobLog.execTime')">
            <a-range-picker v-model="dateRange" style="width: 240px" />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="handleReset">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #toolbar>
        <a-button
          v-hasPermi="['monitor:job:remove']"
          :disabled="multiple"
          status="danger"
          @click="handleDelete()"
        >
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['monitor:job:remove']" status="danger" @click="handleClean">
          <template #icon><IconDelete /></template>
          {{ t('common.clean') }}
        </a-button>
        <a-button
          v-hasPermi="['monitor:job:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
        <a-button v-hasPermi="['monitor:job:list']" @click="backToJob">
          <template #icon><IconUndo /></template>
          {{ t('monitor.jobLog.backToJob') }}
        </a-button>
      </template>

      <template #cell-jobGroup="{ record }">
        <DictTag :options="sysJobGroup" :value="asLog(record).jobGroup" />
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysCommonStatus" :value="asLog(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['monitor:job:query']" @click="openDetail(asLog(record))">{{ t('common.detail') }}</a-link>
          <a-link
            v-hasPermi="['monitor:job:remove']"
            status="danger"
            @click="handleDelete(asLog(record).jobLogId)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 调度日志详情弹窗 -->
    <a-modal
      :visible="detail.open"
      :title="t('monitor.jobLog.detailTitle')"
      :width="680"
      :footer="false"
      @cancel="detail.open = false"
      @close="detail.open = false"
    >
      <a-descriptions :column="2" bordered size="medium">
        <a-descriptions-item :label="t('monitor.jobLog.jobLogId')" :span="1">{{ detail.row?.jobLogId }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.jobName')" :span="1">{{ detail.row?.jobName }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.jobGroup')" :span="1">
          <DictTag :options="sysJobGroup" :value="detail.row?.jobGroup" />
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.status')" :span="1">
          <DictTag :options="sysCommonStatus" :value="detail.row?.status" />
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.invokeTargetLabel')" :span="2">
          <span class="pre-wrap">{{ detail.row?.invokeTarget }}</span>
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.execTime')" :span="1">{{ detail.row?.createTime }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.jobLog.jobMessage')" :span="1">
          <span class="pre-wrap">{{ detail.row?.jobMessage }}</span>
        </a-descriptions-item>
        <a-descriptions-item v-if="detail.row?.jobException" :label="t('monitor.jobLog.jobException')" :span="2">
          <pre class="log-pre log-pre--error">{{ detail.row.jobException }}</pre>
        </a-descriptions-item>
      </a-descriptions>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconDelete,
  IconDownload,
  IconRefresh,
  IconSearch,
  IconUndo
} from '@arco-design/web-vue/es/icon'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import { cleanJobLog, delJobLog, exportJobLog, listJobLog } from '@/api/monitor/job'
import type { JobLogQuery, SysJobLog } from '@/api/monitor/job'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'JobLog' })

const router = useRouter()
const { t } = useI18n()

const dicts = useDict('sys_job_group', 'sys_common_status')
const sysJobGroup = dicts['sys_job_group']
const sysCommonStatus = dicts['sys_common_status']

const jobGroupOptions = computed(() =>
  sysJobGroup.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

const statusOptions = computed(() =>
  sysCommonStatus.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'jobLogId', label: t('monitor.jobLog.jobLogId'), width: 90 },
  { key: 'jobName', label: t('monitor.jobLog.jobName'), minWidth: 130, ellipsis: true, tooltip: true },
  { key: 'jobGroup', label: t('monitor.jobLog.jobGroup'), width: 100 },
  { key: 'invokeTarget', label: t('monitor.jobLog.invokeTargetLabel'), minWidth: 180, ellipsis: true, tooltip: true },
  { key: 'jobMessage', label: t('monitor.jobLog.jobMessage'), minWidth: 160, ellipsis: true, tooltip: true },
  { key: 'status', label: t('monitor.jobLog.status'), width: 90 },
  { key: 'createTime', label: t('monitor.jobLog.execTime'), width: 165 },
  { key: 'operation', label: t('common.fields.operation'), width: 110 }
])

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: JobLogQuery): JobLogQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

const crud = useCrud<SysJobLog, JobLogQuery>({
  listApi: (query) => listJobLog(mergeDateRange(query)),
  deleteApi: delJobLog,
  exportUrl: '/monitor/jobLog/export',
  exportName: `${t('monitor.jobLog.exportFileName')}.xlsx`,
  pkField: 'jobLogId'
})

const {
  loading,
  exportLoading,
  list,
  total,
  page,
  limit,
  getList,
  handleQuery,
  resetQuery,
  setSelection,
  multiple,
  handleDelete
} = crud

const queryParams = crud.queryParams

function asLog(record: TableData): SysJobLog {
  return record as SysJobLog
}

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportJobLog(mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value }))
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/** 清空全部调度日志（单独确认文案，防误操作） */
function handleClean(): void {
  Modal.confirm({
    title: t('common.cleanConfirm'),
    content: t('common.cleanAllConfirm', { field: t('monitor.jobLog.name') }),
    hideCancel: false,
    onOk: async () => {
      try {
        await cleanJobLog()
        Message.success(t('common.cleanSuccess'))
        await getList()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/** 返回定时任务列表页 */
function backToJob(): void {
  void router.push('/monitor/job')
}

/* ---------- 详情弹窗 ---------- */
const detail = reactive<{ open: boolean; row: SysJobLog | null }>({
  open: false,
  row: null
})

function openDetail(row: SysJobLog): void {
  detail.row = row
  detail.open = true
}

/* ---------- 初始化 ---------- */
void getList()
</script>

<style scoped>
.log-pre {
  margin: 0;
  max-height: 200px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 12px;
  line-height: 1.6;
  background-color: var(--color-fill-2);
  padding: 8px;
  border-radius: 4px;
}

.log-pre--error {
  color: rgb(var(--red-6));
}

.pre-wrap {
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
