<template>
  <div class="config-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="configId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="configName" :label="t('system.config.configName')">
            <a-input
              v-model.trim="queryParams.configName"
              :placeholder="t('common.pleaseEnter', { field: t('system.config.configName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="configKey" :label="t('system.config.configKey')">
            <a-input
              v-model.trim="queryParams.configKey"
              :placeholder="t('common.pleaseEnter', { field: t('system.config.configKey') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="configType" :label="t('common.fields.builtIn')">
            <a-select
              v-model="queryParams.configType"
              :options="yesNoOptions"
              :placeholder="t('common.fields.builtIn')"
              allow-clear
              style="width: 140px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item :label="t('common.fields.createTime')">
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
        <a-button v-hasPermi="['system:config:add']" type="primary" @click="handleAdd">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button v-hasPermi="['system:config:edit']" :disabled="single" @click="handleUpdateSelection">
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:config:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['system:config:edit']" @click="handleRefreshCache">
          <template #icon><IconRefresh /></template>
          {{ t('common.refreshCache') }}
        </a-button>
        <a-button
          v-hasPermi="['system:config:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-configKey="{ record }">
        <a-space :size="4">
          <span>{{ asConfig(record).configKey }}</span>
          <a-link @click="copyConfigKey(asConfig(record).configKey)">
            <IconCopy :size="14" />
          </a-link>
        </a-space>
      </template>

      <template #cell-configType="{ record }">
        <DictTag :options="sysYesNo" :value="asConfig(record).configType" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['system:config:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link
            v-hasPermi="['system:config:remove']"
            status="danger"
            @click="handleDelete(asConfig(record).configId, asConfig(record).configName)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/编辑参数弹窗 -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="560"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-form-item field="configName" :label="t('system.config.configName')">
          <a-input
            v-model.trim="configForm.configName"
            :placeholder="t('common.pleaseEnter', { field: t('system.config.configName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="configKey" :label="t('system.config.configKey')">
          <a-input
            v-model.trim="configForm.configKey"
            :placeholder="t('common.pleaseEnter', { field: t('system.config.configKey') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="configValue" :label="t('system.config.configValue')">
          <a-input
            v-model.trim="configForm.configValue"
            :placeholder="t('common.pleaseEnter', { field: t('system.config.configValue') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="configType" :label="t('common.fields.builtIn')">
          <a-radio-group v-model="configForm.configType">
            <a-radio v-for="item in sysYesNo" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="configForm.remark"
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
import { Message } from '@arco-design/web-vue'
import {
  IconCopy,
  IconDelete,
  IconDownload,
  IconEdit,
  IconPlus,
  IconRefresh,
  IconSearch
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  addConfig,
  delConfig,
  exportConfig,
  listConfig,
  refreshConfigCache,
  updateConfig
} from '@/api/system/config'
import type { ConfigQuery, SysConfig } from '@/api/system/config'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Config' })

/** 弹窗表单类型 */
type ConfigForm = Partial<SysConfig>

const { t } = useI18n()
const dicts = useDict('sys_normal_disable', 'sys_yes_no')
const sysYesNo = dicts['sys_yes_no']

const yesNoOptions = computed(() =>
  sysYesNo.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'configId', label: t('system.config.configId'), width: 90 },
  { key: 'configName', label: t('system.config.configName'), width: 180, ellipsis: true, tooltip: true },
  { key: 'configKey', label: t('system.config.configKey'), minWidth: 220 },
  { key: 'configValue', label: t('system.config.configValue'), minWidth: 160, ellipsis: true, tooltip: true },
  { key: 'configType', label: t('common.fields.builtIn'), width: 100 },
  { key: 'remark', label: t('common.fields.remark'), minWidth: 140, ellipsis: true, tooltip: true },
  { key: 'operation', label: t('common.fields.operation'), width: 140 }
])

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: ConfigQuery): ConfigQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  configName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.config.configName') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  configKey: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.config.configKey') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  configValue: [{ required: true, message: t('common.pleaseEnter', { field: t('system.config.configValue') }) }]
}))

const crud = useCrud<SysConfig, ConfigQuery>({
  listApi: (query) => listConfig(mergeDateRange(query)),
  addApi: addConfig,
  updateApi: updateConfig,
  deleteApi: delConfig,
  pkField: 'configId',
  formFactory: () => ({ configType: 'N' })
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

function asConfig(record: TableData): SysConfig {
  return record as SysConfig
}

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const configForm = computed(() => modal.form as ConfigForm)

const submitting = ref(false)

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleUpdateRow(record: TableData): Promise<void> {
  crud.handleUpdate(asConfig(record))
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
    if (configForm.value.configId != null) {
      await updateConfig(configForm.value)
      Message.success(t('common.updateSuccess'))
    } else {
      await addConfig(configForm.value)
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

async function handleRefreshCache(): Promise<void> {
  try {
    await refreshConfigCache()
    Message.success(t('common.cacheRefreshed'))
  } catch {
    // 失败提示已由响应拦截器统一弹出
  }
}

/** 复制参数键名（剪贴板不可用时降级提示） */
async function copyConfigKey(configKey: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(configKey)
    Message.success(t('system.config.copied', { key: configKey }))
  } catch {
    Message.warning(t('system.config.copyUnsupported'))
  }
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportConfig(
      mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value })
    )
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/* ---------- 初始化 ---------- */
void getList()
</script>
