<template>
  <div class="post-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="postId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="postCode" :label="t('system.post.postCode')">
            <a-input
              v-model.trim="queryParams.postCode"
              :placeholder="t('common.pleaseEnter', { field: t('system.post.postCode') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="postName" :label="t('system.post.postName')">
            <a-input
              v-model.trim="queryParams.postName"
              :placeholder="t('common.pleaseEnter', { field: t('system.post.postName') })"
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
              :placeholder="t('system.post.postStatus')"
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
        <a-button v-hasPermi="['system:post:add']" type="primary" @click="handleAdd">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button v-hasPermi="['system:post:edit']" :disabled="single" @click="handleUpdateSelection">
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:post:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button
          v-hasPermi="['system:post:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysNormalDisable" :value="asPost(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['system:post:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
          <a-link
            v-hasPermi="['system:post:remove']"
            status="danger"
            @click="handleDelete(asPost(record).postId, asPost(record).postName)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 新增/编辑岗位弹窗 -->
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
        <a-form-item field="postName" :label="t('system.post.postName')">
          <a-input
            v-model.trim="postForm.postName"
            :placeholder="t('common.pleaseEnter', { field: t('system.post.postName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="postCode" :label="t('system.post.postCode')">
          <a-input
            v-model.trim="postForm.postCode"
            :placeholder="t('common.pleaseEnter', { field: t('system.post.postCode') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="postSort" :label="t('system.post.postSort')">
          <a-input-number v-model="postForm.postSort" :min="0" :placeholder="t('common.pleaseEnter', { field: t('system.post.postSort') })" />
        </a-form-item>
        <a-form-item field="status" :label="t('system.post.postStatus')">
          <a-radio-group v-model="postForm.status">
            <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="postForm.remark"
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
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import { addPost, delPost, exportPost, listPost, updatePost } from '@/api/system/post'
import type { PostQuery, SysPost } from '@/api/system/post'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Post' })

/** 弹窗表单类型 */
type PostForm = Partial<SysPost>

const { t } = useI18n()
const dicts = useDict('sys_normal_disable')
const sysNormalDisable = dicts['sys_normal_disable']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'postId', label: t('system.post.postId'), width: 90 },
  { key: 'postCode', label: t('system.post.postCode'), width: 140 },
  { key: 'postName', label: t('system.post.postName'), width: 160 },
  { key: 'postSort', label: t('system.post.postSortColumn'), width: 90, align: 'center' },
  { key: 'status', label: t('common.fields.status'), width: 100 },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 140 }
])

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  postName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.post.postName') }) },
    { maxLength: 50, message: t('common.maxLengthTip', { max: 50 }) }
  ],
  postCode: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.post.postCode') }) },
    { maxLength: 50, message: t('common.maxLengthTip', { max: 50 }) }
  ],
  postSort: [{ required: true, message: t('common.pleaseEnter', { field: t('system.post.postSort') }) }]
}))

const crud = useCrud<SysPost, PostQuery>({
  listApi: listPost,
  addApi: addPost,
  updateApi: updatePost,
  deleteApi: delPost,
  pkField: 'postId',
  formFactory: () => ({ postSort: 0, status: '0' })
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

function asPost(record: TableData): SysPost {
  return record as SysPost
}

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const postForm = computed(() => modal.form as PostForm)

const submitting = ref(false)

async function handleUpdateRow(record: TableData): Promise<void> {
  crud.handleUpdate(asPost(record))
}

function handleUpdateSelection(): void {
  const first = crud.selection.value[0]
  if (first) handleUpdateRow(first)
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
    if (postForm.value.postId != null) {
      await updatePost(postForm.value)
      Message.success(t('common.updateSuccess'))
    } else {
      await addPost(postForm.value)
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
    await exportPost({ ...queryParams, pageNum: page.value, pageSize: limit.value })
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/* ---------- 初始化 ---------- */
void getList()
</script>
