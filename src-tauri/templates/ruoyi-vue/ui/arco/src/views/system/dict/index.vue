<template>
  <div class="dict-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="dictId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="dictName" :label="t('system.dict.dictName')">
            <a-input
              v-model.trim="queryParams.dictName"
              :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="dictType" :label="t('system.dict.dictType')">
            <a-input
              v-model.trim="queryParams.dictType"
              :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictType') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="status" :label="t('common.fields.status')">
            <a-select
              v-model="queryParams.status"
              :options="statusOptions"
              :placeholder="t('system.dict.statusPlaceholder')"
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
        <a-button v-hasPermi="['system:dict:add']" type="primary" @click="handleAdd">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button v-hasPermi="['system:dict:edit']" :disabled="single" @click="handleUpdateSelection">
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:dict:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['system:dict:edit']" @click="handleRefreshCache">
          <template #icon><IconRefresh /></template>
          {{ t('common.refreshCache') }}
        </a-button>
        <a-button
          v-hasPermi="['system:dict:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-dictType="{ record }">
        <a-link @click="goDictData(asDictType(record))">{{ asDictType(record).dictType }}</a-link>
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysNormalDisable" :value="asDictType(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['system:dict:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link @click="goDictData(asDictType(record))">{{ t('system.dict.dictData') }}</a-link>
          <a-link
            v-hasPermi="['system:dict:remove']"
            status="danger"
            @click="handleDelete(asDictType(record).dictId, asDictType(record).dictName)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/编辑字典类型弹窗 -->
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
        <a-form-item field="dictName" :label="t('system.dict.dictName')">
          <a-input
            v-model.trim="dictForm.dictName"
            :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="dictType" :label="t('system.dict.dictType')">
          <a-input
            v-model.trim="dictForm.dictType"
            :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictType') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-radio-group v-model="dictForm.status">
            <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="dictForm.remark"
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
  IconDelete,
  IconDownload,
  IconEdit,
  IconPlus,
  IconRefresh,
  IconSearch
} from '@arco-design/web-vue/es/icon'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  addType,
  delType,
  listType,
  refreshCache,
  updateType
} from '@/api/system/dict'
import type { DictTypeQuery, SysDictType } from '@/api/system/dict'
import { useCrud } from '@/hooks/useCrud'
import { useDict, clearDictCache } from '@/hooks/useDict'
import { exportRequest } from '@/utils/download'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Dict' })

/** 弹窗表单类型 */
type DictTypeForm = Partial<SysDictType>

const router = useRouter()
const { t } = useI18n()
const dicts = useDict('sys_normal_disable')
const sysNormalDisable = dicts['sys_normal_disable']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'dictId', label: t('system.dict.dictId'), width: 90 },
  { key: 'dictName', label: t('system.dict.dictName'), width: 160 },
  { key: 'dictType', label: t('system.dict.dictType'), minWidth: 220 },
  { key: 'status', label: t('common.fields.status'), width: 90 },
  { key: 'remark', label: t('common.fields.remark'), minWidth: 160, ellipsis: true, tooltip: true },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 200 }
])

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: DictTypeQuery): DictTypeQuery {
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
  dictName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.dict.dictName') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  dictType: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.dict.dictType') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ]
}))

const crud = useCrud<SysDictType, DictTypeQuery>({
  listApi: (query) => listType(mergeDateRange(query)),
  addApi: addType,
  updateApi: updateType,
  deleteApi: delType,
  pkField: 'dictId',
  formFactory: () => ({ status: '0' })
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

function asDictType(record: TableData): SysDictType {
  return record as SysDictType
}

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const dictForm = computed(() => modal.form as DictTypeForm)

const submitting = ref(false)

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleUpdateRow(record: TableData): Promise<void> {
  crud.handleUpdate(asDictType(record))
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
    if (dictForm.value.dictId != null) {
      await updateType(dictForm.value)
      Message.success(t('common.updateSuccess'))
    } else {
      await addType(dictForm.value)
      Message.success(t('common.addSuccess'))
    }
    modal.open = false
    // 字典类型变更影响下拉缓存，成功后清空前端字典缓存
    clearDictCache()
    await getList()
  } catch {
    // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
  } finally {
    submitting.value = false
  }
}

async function handleRefreshCache(): Promise<void> {
  try {
    await refreshCache()
    clearDictCache()
    Message.success(t('common.cacheRefreshed'))
  } catch {
    // 失败提示已由响应拦截器统一弹出
  }
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportRequest(
      '/system/dict/type/export',
      mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value }),
      `${t('system.dict.exportFileName')}.xlsx`
    )
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/* ---------- 字典数据页跳转（隐藏路由 DictData） ---------- */
function goDictData(row: SysDictType): void {
  void router.push(`/system/dict-data/${row.dictId}`)
}

/* ---------- 初始化 ---------- */
void getList()
</script>
