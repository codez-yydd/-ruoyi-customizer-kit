<template>
  <div class="operlog-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="operId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="title" :label="t('monitor.operlog.title')">
            <a-input
              v-model.trim="queryParams.title"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.operlog.title') })"
              allow-clear
              style="width: 150px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="operName" :label="t('monitor.operlog.operator')">
            <a-input
              v-model.trim="queryParams.operName"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.operlog.operator') })"
              allow-clear
              style="width: 150px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="businessType" :label="t('monitor.operlog.typeLabel')">
            <a-select
              v-model="queryParams.businessType"
              :options="operTypeOptions"
              :placeholder="t('monitor.operlog.businessType')"
              allow-clear
              style="width: 130px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item field="status" :label="t('common.fields.status')">
            <a-select
              v-model="queryParams.status"
              :options="statusOptions"
              :placeholder="t('monitor.operlog.operStatus')"
              allow-clear
              style="width: 120px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item :label="t('monitor.operlog.operTime')">
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
          v-hasPermi="['monitor:operlog:remove', 'system:operlog:remove']"
          :disabled="multiple"
          status="danger"
          @click="handleDelete()"
        >
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['monitor:operlog:remove', 'system:operlog:remove']" status="danger" @click="handleClean">
          <template #icon><IconDelete /></template>
          {{ t('common.clean') }}
        </a-button>
        <a-button
          v-hasPermi="['monitor:operlog:export', 'system:operlog:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-title="{ record }">
        {{ asLog(record).title }}
        <a-tag v-if="asLog(record).businessType != null" size="small" color="arcoblue">
          {{ dictLabel(sysOperType, asLog(record).businessType) }}
        </a-tag>
      </template>

      <template #cell-businessType="{ record }">
        <DictTag :options="sysOperType" :value="asLog(record).businessType" />
      </template>

      <template #cell-requestMethod="{ record }">
        <a-tag size="small">{{ asLog(record).requestMethod || '-' }}</a-tag>
      </template>

      <template #cell-operUrl="{ record }">
        <span :title="asLog(record).operUrl">{{ truncate(asLog(record).operUrl, 30) }}</span>
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysCommonStatus" :value="asLog(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-link v-hasPermi="['monitor:operlog:query', 'system:operlog:query']" @click="openDetail(asLog(record))">
          {{ t('common.detail') }}
        </a-link>
      </template>
    </CrudTable>

    <!-- 操作日志详情弹窗（后端无详情接口，使用列表行数据） -->
    <a-modal
      :visible="detail.open"
      :title="t('monitor.operlog.detailTitle')"
      :width="760"
      :footer="false"
      @cancel="detail.open = false"
      @close="detail.open = false"
    >
      <a-descriptions :column="2" bordered size="medium" class="operlog-detail">
        <a-descriptions-item :label="t('monitor.operlog.detailOperId')" :span="1">{{ detail.row?.operId }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.operTime')" :span="1">{{ detail.row?.operTime }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.title')" :span="1">{{ detail.row?.title }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.businessType')" :span="1">
          <DictTag :options="sysOperType" :value="detail.row?.businessType" />
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.operator')" :span="1">{{ detail.row?.operName }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.detailDept')" :span="1">{{ detail.row?.deptName }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.requestMethod')" :span="1">{{ detail.row?.requestMethod }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.operStatus')" :span="1">
          <DictTag :options="sysCommonStatus" :value="detail.row?.status" />
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.operUrl')" :span="2">
          <span class="pre-wrap">{{ detail.row?.operUrl }}</span>
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.detailHost')" :span="1">{{ detail.row?.operIp }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.operLocation')" :span="1">{{ detail.row?.operLocation }}</a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.costTimeLabel')" :span="1">
          {{ detail.row?.costTime != null ? `${detail.row.costTime} ms` : '-' }}
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.invokeMethod')" :span="1">
          <span class="pre-wrap">{{ detail.row?.method }}</span>
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.requestParams')" :span="2">
          <pre class="log-pre">{{ formatJson(detail.row?.operParam) }}</pre>
        </a-descriptions-item>
        <a-descriptions-item :label="t('monitor.operlog.responseParams')" :span="2">
          <pre class="log-pre">{{ formatJson(detail.row?.jsonResult) }}</pre>
        </a-descriptions-item>
        <a-descriptions-item v-if="detail.row?.errorMsg" :label="t('monitor.operlog.errorMsg')" :span="2">
          <pre class="log-pre log-pre--error">{{ detail.row.errorMsg }}</pre>
        </a-descriptions-item>
      </a-descriptions>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import { IconDelete, IconDownload, IconRefresh, IconSearch } from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  cleanOperlog,
  delOperlog,
  exportOperlog,
  listOperlog
} from '@/api/monitor/operlog'
import type { OperLogQuery, SysOperLog } from '@/api/monitor/operlog'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Operlog' })

const { t } = useI18n()

const dicts = useDict('sys_oper_type', 'sys_common_status')
const sysOperType = dicts['sys_oper_type']
const sysCommonStatus = dicts['sys_common_status']

const operTypeOptions = computed(() =>
  sysOperType.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

const statusOptions = computed(() =>
  sysCommonStatus.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'operId', label: t('monitor.operlog.operId'), width: 90 },
  { key: 'title', label: t('monitor.operlog.title'), minWidth: 160 },
  { key: 'businessType', label: t('monitor.operlog.businessType'), width: 100 },
  { key: 'requestMethod', label: t('monitor.operlog.requestMethod'), width: 90 },
  { key: 'operName', label: t('monitor.operlog.operator'), width: 110 },
  { key: 'deptName', label: t('monitor.operlog.dept'), width: 120, ellipsis: true, tooltip: true },
  { key: 'operUrl', label: t('monitor.operlog.operUrl'), minWidth: 180, ellipsis: true },
  { key: 'operIp', label: t('monitor.operlog.operIp'), width: 120 },
  { key: 'operLocation', label: t('monitor.operlog.operLocation'), width: 110 },
  { key: 'status', label: t('monitor.operlog.operStatus'), width: 90 },
  { key: 'costTime', label: t('monitor.operlog.costTime'), width: 90 },
  { key: 'operTime', label: t('monitor.operlog.operTime'), width: 165 },
  { key: 'operation', label: t('common.fields.operation'), width: 80 }
])

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: OperLogQuery): OperLogQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

const crud = useCrud<SysOperLog, OperLogQuery>({
  listApi: (query) => listOperlog(mergeDateRange(query)),
  deleteApi: delOperlog,
  exportUrl: '/monitor/operlog/export',
  exportName: `${t('monitor.operlog.exportFileName')}.xlsx`,
  pkField: 'operId'
})

const { loading, exportLoading, list, total, page, limit, getList, handleQuery, resetQuery, setSelection, multiple, handleDelete } =
  crud

const queryParams = crud.queryParams

function asLog(record: TableData): SysOperLog {
  return record as SysOperLog
}

/** 字典值转标签文案（搜索选项/标题列内联展示用） */
function dictLabel(options: { dictLabel: string; dictValue: string }[], value?: number | string | null): string {
  const raw = value == null ? '' : String(value)
  return options.find((item) => item.dictValue === raw)?.dictLabel ?? raw
}

function truncate(text?: string, length = 30): string {
  if (!text) return '-'
  return text.length > length ? `${text.slice(0, length)}...` : text
}

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportOperlog(mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value }))
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/** 清空全部操作日志（单独确认文案，防误操作） */
function handleClean(): void {
  Modal.confirm({
    title: t('common.cleanConfirm'),
    content: t('common.cleanAllConfirm', { field: t('monitor.operlog.name') }),
    hideCancel: false,
    onOk: async () => {
      try {
        await cleanOperlog()
        Message.success(t('common.cleanSuccess'))
        await getList()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/* ---------- 详情弹窗 ---------- */
const detail = reactive<{ open: boolean; row: SysOperLog | null }>({
  open: false,
  row: null
})

function openDetail(row: SysOperLog): void {
  detail.row = row
  detail.open = true
}

/** JSON 字符串格式化展示（解析失败原样返回） */
function formatJson(text?: string): string {
  if (!text) return '-'
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
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
