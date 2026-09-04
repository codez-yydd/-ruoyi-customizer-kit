<template>
  <div class="dict-data">
    <!-- 字典信息头 -->
    <a-card :bordered="false" class="app-page-card dict-data__header">
      <div class="dict-data__header-main">
        <a-button @click="goBack">
          <template #icon><IconLeft /></template>
          {{ t('common.back') }}
        </a-button>
        <div class="dict-data__dict-info">
          <span class="dict-data__title">{{ t('system.dict.dictData') }}</span>
          <a-tag v-if="dictInfo" color="arcoblue" size="small">
            {{ dictInfo.dictName }}（{{ dictInfo.dictType }}）
          </a-tag>
        </div>
      </div>
    </a-card>

    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="dictCode"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="dictLabel" :label="t('system.dict.dictLabel')">
            <a-input
              v-model.trim="queryParams.dictLabel"
              :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictLabel') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="dictValue" :label="t('system.dict.dictValue')">
            <a-input
              v-model.trim="queryParams.dictValue"
              :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictValue') })"
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
      </template>

      <template #cell-dictLabel="{ record }">
        <DictTag :options="[asData(record)]" :value="asData(record).dictValue" />
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysNormalDisable" :value="asData(record).status" />
      </template>

      <template #cell-isDefault="{ record }">
        <DictTag :options="sysYesNo" :value="asData(record).isDefault" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['system:dict:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link
            v-hasPermi="['system:dict:remove']"
            status="danger"
            @click="handleDelete(asData(record).dictCode)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/编辑字典数据弹窗 -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="600"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-form-item field="dictLabel" :label="t('system.dict.dictLabel')">
          <a-input
            v-model.trim="dataForm.dictLabel"
            :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictLabel') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="dictValue" :label="t('system.dict.dictValue')">
          <a-input
            v-model.trim="dataForm.dictValue"
            :placeholder="t('common.pleaseEnter', { field: t('system.dict.dictValue') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="dictSort" :label="t('common.fields.displaySort')">
          <a-input-number v-model="dataForm.dictSort" :min="0" :placeholder="t('common.pleaseEnter', { field: t('common.fields.displaySort') })" />
        </a-form-item>
        <a-form-item field="listClass" :label="t('system.dict.listClass')">
          <a-select
            v-model="dataForm.listClass"
            :options="listClassOptions"
            :placeholder="t('common.pleaseSelect', { field: t('system.dict.listClass') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="cssClass" :label="t('system.dict.cssClass')">
          <a-input v-model.trim="dataForm.cssClass" :placeholder="t('system.dict.cssClassPlaceholder')" allow-clear />
        </a-form-item>
        <a-form-item field="isDefault" :label="t('common.fields.builtIn')">
          <a-radio-group v-model="dataForm.isDefault">
            <a-radio v-for="item in sysYesNo" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-radio-group v-model="dataForm.status">
            <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="dataForm.remark"
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
  IconEdit,
  IconLeft,
  IconPlus,
  IconRefresh,
  IconSearch
} from '@arco-design/web-vue/es/icon'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  addData,
  delData,
  getData,
  getType,
  listData,
  updateData
} from '@/api/system/dict'
import type { DictDataQuery, SysDictData, SysDictType } from '@/api/system/dict'
import { useCrud } from '@/hooks/useCrud'
import { useDict, clearDictCache } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'DictData' })

/** 弹窗表单类型 */
type DictDataForm = Partial<SysDictData>

/** 查询参数：dictValue 后端不支持条件查询，由前端过滤 */
type DictDataQueryExt = DictDataQuery & { dictValue?: string }

const route = useRoute()
const router = useRouter()
const { t } = useI18n()

/** 路由参数中的字典类型 id（/system/dict-data/:dictId） */
const dictId = Number(route.params.dictId)

const dicts = useDict('sys_normal_disable', 'sys_yes_no')
const sysNormalDisable = dicts['sys_normal_disable']
const sysYesNo = dicts['sys_yes_no']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 回显样式色板（若依 listClass 六项；default 不着色；computed：随语言切换联动选项名） */
const LIST_CLASS_OPTIONS = computed(() => [
  { label: t('system.dict.listClassOptions.default'), value: 'default' },
  { label: t('system.dict.listClassOptions.primary'), value: 'primary' },
  { label: t('system.dict.listClassOptions.success'), value: 'success' },
  { label: t('system.dict.listClassOptions.info'), value: 'info' },
  { label: t('system.dict.listClassOptions.warning'), value: 'warning' },
  { label: t('system.dict.listClassOptions.danger'), value: 'danger' }
])

const listClassOptions = computed(() => {
  // 编辑回显时后端 listClass 可能为空串，追加"无样式"占位保证受控值可显示
  const current = dataForm.value.listClass
  if (current && !LIST_CLASS_OPTIONS.value.some((item) => item.value === current)) {
    return [
      ...LIST_CLASS_OPTIONS.value,
      { label: t('system.dict.listClassOptions.none', { value: current }), value: current }
    ]
  }
  return LIST_CLASS_OPTIONS.value
})

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'dictCode', label: t('system.dict.dictCode'), width: 100 },
  { key: 'dictLabel', label: t('system.dict.dictLabel'), width: 130 },
  { key: 'dictValue', label: t('system.dict.dictValue'), width: 140 },
  { key: 'dictSort', label: t('common.fields.sort'), width: 80, align: 'center' },
  { key: 'status', label: t('common.fields.status'), width: 90 },
  { key: 'isDefault', label: t('common.fields.builtIn'), width: 100 },
  { key: 'remark', label: t('common.fields.remark'), minWidth: 150, ellipsis: true, tooltip: true },
  { key: 'operation', label: t('common.fields.operation'), width: 140 }
])

/* ---------- 字典类型信息 ---------- */
const dictInfo = ref<SysDictType>()
/** 当前字典类型字符串（getType 成功后回填，listApi 读取） */
const currentDictType = ref('')

void getType(dictId)
  .then((info) => {
    dictInfo.value = info
    currentDictType.value = info.dictType
  })
  .then(() => getList())
  .catch(() => {
    // 类型详情加载失败时仍按空类型查询（错误提示已由拦截器弹出）
    void getList()
  })

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  dictLabel: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.dict.dictLabel') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  dictValue: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.dict.dictValue') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  dictSort: [{ required: true, message: t('common.pleaseEnter', { field: t('common.fields.displaySort') }) }]
}))

const crud = useCrud<SysDictData, DictDataQueryExt>({
  listApi: async (query) => {
    const { dictValue, ...rest } = query
    const result = await listData({ ...rest, dictType: currentDictType.value })
    if (dictValue) {
      result.rows = result.rows.filter((row) => row.dictValue.includes(dictValue))
    }
    return result
  },
  addApi: addData,
  updateApi: updateData,
  deleteApi: delData,
  pkField: 'dictCode',
  formFactory: () => ({ dictSort: 0, isDefault: 'N', status: '0' })
})

const {
  loading,
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

function asData(record: TableData): SysDictData {
  return record as SysDictData
}

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const dataForm = computed(() => modal.form as DictDataForm)

const submitting = ref(false)

function goBack(): void {
  void router.push('/system/dict')
}

async function handleUpdateRow(record: TableData): Promise<void> {
  crud.handleUpdate(asData(record))
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
  const data: DictDataForm = { ...dataForm.value, dictType: currentDictType.value }
  try {
    if (data.dictCode != null) {
      await updateData(data)
      Message.success(t('common.updateSuccess'))
    } else {
      await addData(data)
      Message.success(t('common.addSuccess'))
    }
    modal.open = false
    // 字典数据变更影响下拉缓存，成功后清空前端字典缓存
    clearDictCache()
    await getList()
  } catch {
    // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.dict-data__header {
  margin-bottom: 12px;
}

.dict-data__header-main {
  display: flex;
  align-items: center;
  gap: 12px;
}

.dict-data__dict-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dict-data__title {
  font-size: 16px;
  font-weight: 500;
}
</style>
