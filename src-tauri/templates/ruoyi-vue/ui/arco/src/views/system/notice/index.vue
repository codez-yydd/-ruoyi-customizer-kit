<template>
  <div class="notice-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="noticeId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="noticeTitle" :label="t('system.notice.noticeTitle')">
            <a-input
              v-model.trim="queryParams.noticeTitle"
              :placeholder="t('common.pleaseEnter', { field: t('system.notice.noticeTitle') })"
              allow-clear
              style="width: 180px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="createBy" :label="t('system.notice.operator')">
            <a-input
              v-model.trim="queryParams.createBy"
              :placeholder="t('common.pleaseEnter', { field: t('system.notice.operator') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="noticeType" :label="t('system.notice.typeColumn')">
            <a-select
              v-model="queryParams.noticeType"
              :options="typeOptions"
              :placeholder="t('system.notice.noticeType')"
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
        <a-button v-hasPermi="['system:notice:add']" type="primary" @click="handleAddClick">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button v-hasPermi="['system:notice:edit']" :disabled="single" @click="handleUpdateSelection">
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:notice:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
      </template>

      <template #cell-noticeTitle="{ record }">
        <a-link @click="handleView(asNotice(record))">{{ asNotice(record).noticeTitle }}</a-link>
      </template>

      <template #cell-noticeType="{ record }">
        <DictTag :options="sysNoticeType" :value="asNotice(record).noticeType" />
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysNoticeStatus" :value="asNotice(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link @click="handleView(asNotice(record))">{{ t('common.view') }}</a-link>
          <a-link v-hasPermi="['system:notice:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link
            v-hasPermi="['system:notice:remove']"
            status="danger"
            @click="handleDelete(asNotice(record).noticeId, asNotice(record).noticeTitle)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/编辑公告弹窗（大号：容纳富文本编辑器） -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="800"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-form-item field="noticeTitle" :label="t('system.notice.noticeTitle')">
          <a-input
            v-model.trim="noticeForm.noticeTitle"
            :placeholder="t('common.pleaseEnter', { field: t('system.notice.noticeTitle') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="noticeType" :label="t('system.notice.noticeType')">
          <a-radio-group v-model="noticeForm.noticeType">
            <a-radio v-for="item in sysNoticeType" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-radio-group v-model="noticeForm.status">
            <a-radio v-for="item in sysNoticeStatus" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item :label="t('common.fields.content')">
          <RichEditor v-model="noticeForm.noticeContent" height="280px" />
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="noticeForm.remark"
            :placeholder="t('common.inputContent')"
            :max-length="500"
            show-word-limit
            :auto-size="{ minRows: 2, maxRows: 4 }"
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 公告详情弹窗（渲染后 HTML） -->
    <a-modal
      :visible="viewOpen"
      :title="viewNotice?.noticeTitle ?? t('system.notice.detailTitle')"
      :width="720"
      :footer="false"
      @cancel="viewOpen = false"
      @close="viewOpen = false"
    >
      <a-space :size="16" class="notice-page__view-meta">
        <a-space :size="4">
          {{ t('system.notice.typeLabel') }}<DictTag :options="sysNoticeType" :value="viewNotice?.noticeType" />
        </a-space>
        <a-space :size="4">
          {{ t('system.notice.statusLabel') }}<DictTag :options="sysNoticeStatus" :value="viewNotice?.status" />
        </a-space>
        <span v-if="viewNotice?.createBy">{{ t('system.notice.publisher', { name: viewNotice.createBy }) }}</span>
        <span v-if="viewNotice?.createTime">{{ t('system.notice.publishTime', { time: viewNotice.createTime }) }}</span>
      </a-space>
      <a-divider :margin="12" />
      <!-- 内容来自后端已存储的公告富文本，渲染前经 DOMPurify 净化，消除存储型 XSS -->
      <div class="notice-page__view-content" v-html="sanitizedContent"></div>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { FieldRule, TableData } from '@arco-design/web-vue'
import { Message } from '@arco-design/web-vue'
import DOMPurify from 'dompurify'
import {
  IconDelete,
  IconEdit,
  IconPlus,
  IconRefresh,
  IconSearch
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import RichEditor from '@/components/RichEditor/index.vue'
import { addNotice, delNotice, getNotice, listNotice, updateNotice } from '@/api/system/notice'
import type { NoticeQuery, SysNotice } from '@/api/system/notice'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Notice' })

/** 弹窗表单类型 */
type NoticeForm = Partial<SysNotice>

const { t } = useI18n()
const dicts = useDict('sys_notice_type', 'sys_notice_status')
const sysNoticeType = dicts['sys_notice_type']
const sysNoticeStatus = dicts['sys_notice_status']

const typeOptions = computed(() =>
  sysNoticeType.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'noticeId', label: t('system.notice.noticeId'), width: 90 },
  { key: 'noticeTitle', label: t('system.notice.noticeTitle'), minWidth: 220, ellipsis: true, tooltip: true },
  { key: 'noticeType', label: t('system.notice.typeColumn'), width: 100 },
  { key: 'createBy', label: t('system.notice.createBy'), width: 120, ellipsis: true, tooltip: true },
  { key: 'status', label: t('common.fields.status'), width: 90 },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 170 }
])

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  noticeTitle: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.notice.noticeTitle') }) },
    { maxLength: 50, message: t('common.maxLengthTip', { max: 50 }) }
  ],
  noticeType: [{ required: true, message: t('common.pleaseSelect', { field: t('system.notice.noticeType') }) }]
}))

const crud = useCrud<SysNotice, NoticeQuery>({
  listApi: listNotice,
  addApi: addNotice,
  updateApi: updateNotice,
  deleteApi: delNotice,
  pkField: 'noticeId',
  formFactory: () => ({ noticeType: '1', status: '0', noticeContent: '' })
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

function asNotice(record: TableData): SysNotice {
  return record as SysNotice
}

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const noticeForm = computed(() => modal.form as NoticeForm)

const submitting = ref(false)

/** 打开新增弹窗 */
function handleAddClick(): void {
  handleAdd()
}

/** 修改前拉详情（列表行不含富文本内容） */
async function handleUpdateRow(record: TableData): Promise<void> {
  try {
    const detail = await getNotice(asNotice(record).noticeId)
    crud.handleUpdate({ ...detail })
  } catch {
    // 详情加载失败：错误已由拦截器提示
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
    if (noticeForm.value.noticeId != null) {
      await updateNotice(noticeForm.value)
      Message.success(t('common.updateSuccess'))
    } else {
      await addNotice(noticeForm.value)
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

/* ---------- 详情查看 ---------- */
const viewOpen = ref(false)
const viewNotice = ref<SysNotice>()

/** 富文本 HTML 经 DOMPurify 净化后再 v-html 渲染 */
const sanitizedContent = computed<string>(() => DOMPurify.sanitize(viewNotice.value?.noticeContent ?? ''))

async function handleView(row: SysNotice): Promise<void> {
  try {
    // 行数据可能不含富文本内容，统一走详情接口
    viewNotice.value = await getNotice(row.noticeId)
    viewOpen.value = true
  } catch {
    // 详情加载失败：错误已由拦截器提示
  }
}
</script>

<style scoped>
.notice-page__view-meta {
  font-size: 13px;
  color: var(--color-text-2);
  flex-wrap: wrap;
}

.notice-page__view-content {
  max-height: 480px;
  overflow-y: auto;
  font-size: 14px;
  line-height: 1.8;
  word-break: break-word;
}
</style>
