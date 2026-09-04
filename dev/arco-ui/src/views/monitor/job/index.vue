<template>
  <div class="job-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="jobId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="jobName" :label="t('monitor.job.jobName')">
            <a-input
              v-model.trim="queryParams.jobName"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.job.jobName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="jobGroup" :label="t('monitor.job.jobGroup')">
            <a-select
              v-model="queryParams.jobGroup"
              :options="jobGroupOptions"
              :placeholder="t('monitor.job.jobGroup')"
              allow-clear
              style="width: 140px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item field="status" :label="t('common.fields.status')">
            <a-select
              v-model="queryParams.status"
              :options="jobStatusOptions"
              :placeholder="t('monitor.job.statusPlaceholder')"
              allow-clear
              style="width: 120px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="resetQuery">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #toolbar>
        <a-button v-hasPermi="['monitor:job:add']" type="primary" @click="handleAdd">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button v-hasPermi="['monitor:job:edit']" :disabled="single" @click="handleUpdateSelection">
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['monitor:job:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button
          v-hasPermi="['monitor:job:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
        <a-button v-hasPermi="['monitor:job:query']" @click="goJobLog">
          <template #icon><IconHistory /></template>
          {{ t('monitor.job.jobLogEntry') }}
        </a-button>
      </template>

      <template #cell-jobGroup="{ record }">
        <DictTag :options="sysJobGroup" :value="asJob(record).jobGroup" />
      </template>

      <template #cell-invokeTarget="{ record }">
        <span :title="asJob(record).invokeTarget">{{ asJob(record).invokeTarget }}</span>
      </template>

      <template #cell-status="{ record }">
        <a-switch
          :model-value="asJob(record).status === '0'"
          :disabled="!checkPermi(['monitor:job:changeStatus'])"
          @change="(value) => onStatusChange(asJob(record), value as unknown as boolean)"
        >
          <template #checked>{{ t('monitor.job.enable') }}</template>
          <template #unchecked>{{ t('monitor.job.pause') }}</template>
        </a-switch>
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['monitor:job:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link
            v-hasPermi="['monitor:job:remove']"
            status="danger"
            @click="handleDelete(asJob(record).jobId, asJob(record).jobName)"
          >
            {{ t('common.delete') }}
          </a-link>
          <a-dropdown v-hasPermi="['monitor:job:changeStatus']" trigger="hover">
            <a-link>
              {{ t('common.more') }}
              <IconDown :size="12" />
            </a-link>
            <template #content>
              <a-doption @click="handleRunOnce(asJob(record))">
                <template #icon><IconThunderbolt /></template>
                {{ t('monitor.job.runOnce') }}
              </a-doption>
              <a-doption @click="goJobLog">
                <template #icon><IconHistory /></template>
                {{ t('monitor.job.jobLogEntry') }}
              </a-doption>
            </template>
          </a-dropdown>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/修改任务弹窗 -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="640"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-row :gutter="8">
          <a-col :span="12">
            <a-form-item field="jobName" :label="t('monitor.job.jobName')">
              <a-input
                v-model.trim="jobForm.jobName"
                :placeholder="t('common.pleaseEnter', { field: t('monitor.job.jobName') })"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="jobGroup" :label="t('monitor.job.jobGroup')">
              <a-radio-group v-model="jobForm.jobGroup">
                <a-radio v-for="item in sysJobGroup" :key="item.dictValue" :value="item.dictValue">
                  {{ item.dictLabel }}
                </a-radio>
              </a-radio-group>
            </a-form-item>
          </a-col>
        </a-row>
        <a-form-item field="invokeTarget" :label="t('monitor.job.invokeTarget')">
          <a-input
            v-model.trim="jobForm.invokeTarget"
            :placeholder="t('monitor.job.invokeTargetPlaceholder')"
            allow-clear
          />
        </a-form-item>
        <a-row :gutter="8">
          <a-col :span="14">
            <a-form-item field="cronExpression" :label="t('monitor.job.cronExpression')">
              <a-input
                v-model.trim="jobForm.cronExpression"
                :placeholder="t('monitor.job.cronPlaceholder')"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="10">
            <a-form-item :label="undefined">
              <a-dropdown trigger="hover">
                <a-button type="outline" long>
                  {{ t('monitor.job.commonPresets') }}
                  <IconDown :size="12" />
                </a-button>
                <template #content>
                  <a-doption
                    v-for="item in cronPresets"
                    :key="item.expression"
                    @click="jobForm.cronExpression = item.expression"
                  >
                    {{ t('monitor.job.presetWithExpression', { label: item.label, expression: item.expression }) }}
                  </a-doption>
                </template>
              </a-dropdown>
            </a-form-item>
          </a-col>
        </a-row>
        <a-form-item field="misfirePolicy" :label="t('monitor.job.misfirePolicy')">
          <a-radio-group v-model="jobForm.misfirePolicy">
            <a-radio v-for="item in misfireOptions" :key="item.value" :value="item.value">
              {{ item.label }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="concurrent" :label="t('monitor.job.concurrent')">
          <a-radio-group v-model="jobForm.concurrent">
            <a-radio value="0">{{ t('monitor.job.allow') }}</a-radio>
            <a-radio value="1">{{ t('monitor.job.forbid') }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-radio-group v-model="jobForm.status">
            <a-radio v-for="item in sysJobStatus" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="jobForm.remark"
            :placeholder="t('common.inputContent')"
            :max-length="500"
            show-word-limit
            :auto-size="{ minRows: 2, maxRows: 4 }"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { FieldRule, TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconDelete,
  IconDown,
  IconDownload,
  IconEdit,
  IconHistory,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconThunderbolt
} from '@arco-design/web-vue/es/icon'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  addJob,
  changeJobStatus,
  delJob,
  exportJob,
  getJob,
  listJob,
  runJobOnce,
  updateJob
} from '@/api/monitor/job'
import type { JobQuery, SysJob } from '@/api/monitor/job'
import { checkPermi } from '@/utils/permission'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Job' })

const router = useRouter()
const { t } = useI18n()

const dicts = useDict('sys_job_group', 'sys_job_status')
const sysJobGroup = dicts['sys_job_group']
const sysJobStatus = dicts['sys_job_status']

const jobGroupOptions = computed(() =>
  sysJobGroup.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

const jobStatusOptions = computed(() =>
  sysJobStatus.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'jobId', label: t('monitor.job.jobId'), width: 90 },
  { key: 'jobName', label: t('monitor.job.jobName'), minWidth: 140, ellipsis: true, tooltip: true },
  { key: 'jobGroup', label: t('monitor.job.jobGroup'), width: 100 },
  { key: 'invokeTarget', label: t('monitor.job.invokeTargetLabel'), minWidth: 200, ellipsis: true, tooltip: true },
  { key: 'cronExpression', label: t('monitor.job.cronExpression'), width: 140, ellipsis: true, tooltip: true },
  { key: 'status', label: t('common.fields.status'), width: 90 },
  { key: 'operation', label: t('common.fields.operation'), width: 190 }
])

/** 常用 cron 表达式快捷项（computed：随语言切换联动） */
const cronPresets = computed(() => [
  { label: t('monitor.job.presetEvery10s'), expression: '0/10 * * * * ?' },
  { label: t('monitor.job.presetEvery30s'), expression: '0/30 * * * * ?' },
  { label: t('monitor.job.presetEveryMinute'), expression: '0 * * * * ?' },
  { label: t('monitor.job.presetEveryHour'), expression: '0 0 * * * ?' },
  { label: t('monitor.job.presetDailyAt'), expression: '0 30 0 * * ?' },
  { label: t('monitor.job.presetWeeklyAt'), expression: '0 0 9 ? * MON' },
  { label: t('monitor.job.presetMonthlyAt'), expression: '0 0 0 1 * ?' }
])

/** 执行策略选项（1 立即执行 2 执行一次 3 放弃执行） */
const misfireOptions = computed(() => [
  { value: '1', label: t('monitor.job.misfireImmediately') },
  { value: '2', label: t('monitor.job.runOnce') },
  { value: '3', label: t('monitor.job.misfireAbandon') }
])

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  jobName: [{ required: true, message: t('common.pleaseEnter', { field: t('monitor.job.jobName') }) }],
  jobGroup: [{ required: true, message: t('common.pleaseSelect', { field: t('monitor.job.jobGroup') }) }],
  invokeTarget: [{ required: true, message: t('monitor.job.invokeTargetRule') }],
  cronExpression: [{ required: true, message: t('monitor.job.cronPlaceholder') }]
}))

const crud = useCrud<SysJob, JobQuery>({
  listApi: listJob,
  addApi: addJob,
  updateApi: updateJob,
  deleteApi: delJob,
  pkField: 'jobId',
  formFactory: () => ({
    jobGroup: 'DEFAULT',
    misfirePolicy: '1',
    concurrent: '1',
    status: '0'
  })
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
  single,
  multiple,
  modal,
  formRef,
  handleAdd,
  handleDelete,
  cancel
} = crud

const queryParams = crud.queryParams

function asJob(record: TableData): SysJob {
  return record as SysJob
}

/** 模板中对 modal.form 使用带类型视图 */
const jobForm = computed(() => modal.form as Partial<SysJob>)

const submitting = ref(false)

async function handleUpdateRow(record: TableData): Promise<void> {
  // 详情接口回填（列表行不含全部表单字段时保持一致）
  const row = asJob(record)
  crud.handleUpdate(row)
  try {
    const detail = await getJob(row.jobId)
    if (detail) {
      crud.handleUpdate({ ...row, ...detail })
    }
  } catch {
    // 回填失败时保留列表行数据
  }
}

function handleUpdateSelection(): void {
  const first = crud.selection.value[0]
  if (first) void handleUpdateRow(first)
}

async function onSubmit(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    // 校验失败：错误信息已由表单展示
    return
  }
  submitting.value = true
  try {
    if (jobForm.value.jobId != null) {
      await updateJob(jobForm.value)
      Message.success(t('common.updateSuccess'))
    } else {
      await addJob(jobForm.value)
      Message.success(t('common.addSuccess'))
    }
    modal.open = false
    await getList()
  } catch {
    // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
  } finally {
    submitting.value = false
  }
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportJob({ ...queryParams, pageNum: page.value, pageSize: limit.value })
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/** 状态开关：确认后调用 changeStatus，取消时回滚开关值 */
function onStatusChange(row: SysJob, checked: boolean): void {
  const nextStatus = checked ? '0' : '1'
  const action = checked ? t('monitor.job.enable') : t('monitor.job.pause')
  Modal.confirm({
    title: t('monitor.job.statusConfirmTitle'),
    content: t('monitor.job.statusChangeConfirm', { action, name: row.jobName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await changeJobStatus(row.jobId, nextStatus)
        row.status = nextStatus
        Message.success(t('monitor.job.actionSuccess', { action }))
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/** 立即执行一次 */
function handleRunOnce(row: SysJob): void {
  Modal.confirm({
    title: t('monitor.job.runOnceConfirmTitle'),
    content: t('monitor.job.runOnceConfirm', { name: row.jobName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await runJobOnce(row.jobId, row.jobGroup)
        Message.success(t('monitor.job.runOnceSuccess'))
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/** 跳转调度日志（内置隐藏路由 /monitor/job-log，高亮 /monitor/job） */
function goJobLog(): void {
  void router.push('/monitor/job-log')
}

/* ---------- 初始化 ---------- */
void getList()
</script>
