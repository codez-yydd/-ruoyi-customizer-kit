<template>
  <div class="gen-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="tableId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="tableName" :label="t('tool.gen.tableName')">
            <a-input
              v-model.trim="queryParams.tableName"
              :placeholder="t('common.pleaseEnter', { field: t('tool.gen.tableName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="tableComment" :label="t('tool.gen.tableComment')">
            <a-input
              v-model.trim="queryParams.tableComment"
              :placeholder="t('common.pleaseEnter', { field: t('tool.gen.tableComment') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
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
        <a-button v-hasPermi="['tool:gen:add']" type="primary" @click="openImport">
          <template #icon><IconImport /></template>
          {{ t('common.import') }}
        </a-button>
        <a-button
          v-hasPermi="['tool:gen:remove']"
          :disabled="multiple"
          status="danger"
          @click="handleDelete()"
        >
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
      </template>

      <template #cell-tableName="{ record }">
        <a-link @click="openDetail(asTable(record))">{{ asTable(record).tableName }}</a-link>
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link v-hasPermi="['tool:gen:preview']" @click="openPreview(asTable(record))">
            <IconEye :size="13" /> {{ t('tool.gen.preview') }}
          </a-link>
          <a-link v-hasPermi="['tool:gen:edit']" @click="openDetail(asTable(record))">
            <IconFile :size="13" /> {{ t('common.detail') }}
          </a-link>
          <a-link v-hasPermi="['tool:gen:edit']" @click="handleSynchDb(asTable(record))">
            <IconSync :size="13" /> {{ t('tool.gen.sync') }}
          </a-link>
          <a-link v-hasPermi="['tool:gen:code']" @click="handleDownload(asTable(record))">
            <IconCode :size="13" /> {{ t('tool.gen.genCode') }}
          </a-link>
          <a-link
            v-hasPermi="['tool:gen:remove']"
            status="danger"
            @click="handleDelete(asTable(record).tableId)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 导入表弹窗 -->
    <a-modal
      :visible="importModal.open"
      :title="t('tool.gen.importTitle')"
      :width="720"
      :mask-closable="false"
      :ok-loading="importLoading"
      :ok-text="t('common.import')"
      @ok="submitImport"
      @cancel="importModal.open = false"
      @close="importModal.open = false"
    >
      <a-form :model="importModal" layout="inline" class="gen-page__import-form">
        <a-form-item field="tableName" :label="t('tool.gen.tableName')">
          <a-input
            v-model.trim="importModal.tableName"
            :placeholder="t('common.pleaseEnter', { field: t('tool.gen.tableName') })"
            allow-clear
            style="width: 150px"
            @keyup.enter="loadDbTables"
            @clear="loadDbTables"
          />
        </a-form-item>
        <a-form-item field="tableComment" :label="t('tool.gen.tableComment')">
          <a-input
            v-model.trim="importModal.tableComment"
            :placeholder="t('common.pleaseEnter', { field: t('tool.gen.tableComment') })"
            allow-clear
            style="width: 150px"
            @keyup.enter="loadDbTables"
            @clear="loadDbTables"
          />
        </a-form-item>
        <a-form-item :label="t('tool.gen.tplWebType')">
          <a-select v-model="importModal.tplWebType" style="width: 140px">
            <a-option value="element-ui">{{ t('tool.gen.tplElementUI') }}</a-option>
            <a-option value="element-plus">{{ t('tool.gen.tplElementPlus') }}</a-option>
          </a-select>
        </a-form-item>
      </a-form>
      <a-table
        class="gen-page__import-table"
        :data="dbTables"
        :loading="dbLoading"
        :pagination="false"
        :row-selection="dbRowSelection"
        :row-key="'tableName'"
        :scroll="{ y: 320 }"
        @selection-change="onDbSelectionChange"
      >
        <template #columns>
          <a-table-column :title="t('tool.gen.tableName')" data-index="tableName" />
          <a-table-column :title="t('tool.gen.tableComment')" data-index="tableComment" :width="220" />
          <a-table-column :title="t('common.fields.createTime')" data-index="createTime" :width="165" />
        </template>
      </a-table>
      <div class="gen-page__import-page">
        <Pagination
          :page="dbPage"
          :limit="dbLimit"
          :total="dbTotal"
          @update:page="(value) => (dbPage = value)"
          @update:limit="(value) => (dbLimit = value)"
          @change="loadDbTables"
        />
      </div>
    </a-modal>

    <!-- 预览弹窗（模板页签 + 代码块） -->
    <a-modal
      :visible="preview.open"
      :title="t('tool.gen.previewTitle')"
      :width="880"
      :footer="false"
      @cancel="preview.open = false"
      @close="preview.open = false"
    >
      <a-empty v-if="preview.templates.length === 0" :description="t('tool.gen.noPreview')" />
      <a-tabs v-else size="small" lazy-load>
        <a-tab-pane v-for="tpl in preview.templates" :key="tpl.name" :title="shortTemplateName(tpl.name)">
          <div class="gen-page__code-wrap">
            <div class="gen-page__code-actions">
              <a-button type="primary" size="small" @click="copyCode(tpl.content)">
                <template #icon><IconCopy /></template>
                {{ t('common.copy') }}
              </a-button>
            </div>
            <pre class="gen-page__code">{{ tpl.content }}</pre>
          </div>
        </a-tab-pane>
      </a-tabs>
    </a-modal>

    <!-- 详情弹窗（表信息 + 字段列表） -->
    <a-modal
      :visible="detail.open"
      :title="t('tool.gen.detailTitle', { name: detail.table?.tableName ?? '' })"
      :width="880"
      :footer="false"
      @cancel="detail.open = false"
      @close="detail.open = false"
    >
      <a-spin :loading="detailLoading" style="display: block">
        <a-descriptions :column="3" bordered size="medium">
          <a-descriptions-item :label="t('tool.gen.tableName')">{{ detail.table?.tableName }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.tableComment')">{{ detail.table?.tableComment }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.entityClassName')">{{ detail.table?.className }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.moduleName')">{{ detail.table?.moduleName }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.businessName')">{{ detail.table?.businessName }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.functionAuthor')">{{ detail.table?.functionAuthor }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.packageName')" :span="3">{{ detail.table?.packageName }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.functionName')">{{ detail.table?.functionName }}</a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.tplCategory')" :span="2">
            {{ tplCategoryLabel(detail.table?.tplCategory) }}
          </a-descriptions-item>
          <a-descriptions-item :label="t('tool.gen.tplWebTypeLabel')" :span="3">
            {{ tplWebTypeLabel(detail.table?.tplWebType) }}
          </a-descriptions-item>
        </a-descriptions>
        <a-table
          class="gen-page__column-table"
          :data="detail.columns"
          :pagination="false"
          row-key="columnId"
          :scroll="{ y: 280 }"
          size="small"
        >
          <template #columns>
            <a-table-column :title="t('tool.gen.columnName')" data-index="columnName" />
            <a-table-column :title="t('tool.gen.columnComment')" data-index="columnComment" />
            <a-table-column :title="t('tool.gen.physicalType')" data-index="columnType" />
            <a-table-column :title="t('tool.gen.javaType')" data-index="javaType" />
            <a-table-column :title="t('tool.gen.javaField')" data-index="javaField" />
            <a-table-column :title="t('tool.gen.isPk')">
              <template #cell="{ record }">{{ (asColumn(record).isPk ?? '') === '1' ? t('common.yes') : t('common.no') }}</template>
            </a-table-column>
            <a-table-column :title="t('tool.gen.isInsert')">
              <template #cell="{ record }">{{ (asColumn(record).isInsert ?? '') === '1' ? t('common.yes') : t('common.no') }}</template>
            </a-table-column>
            <a-table-column :title="t('tool.gen.isEdit')">
              <template #cell="{ record }">{{ (asColumn(record).isEdit ?? '') === '1' ? t('common.yes') : t('common.no') }}</template>
            </a-table-column>
            <a-table-column :title="t('tool.gen.isList')">
              <template #cell="{ record }">{{ (asColumn(record).isList ?? '') === '1' ? t('common.yes') : t('common.no') }}</template>
            </a-table-column>
            <a-table-column :title="t('tool.gen.isQuery')">
              <template #cell="{ record }">{{ (asColumn(record).isQuery ?? '') === '1' ? t('common.yes') : t('common.no') }}</template>
            </a-table-column>
          </template>
        </a-table>
      </a-spin>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { TableData, TableRowSelection } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconCode,
  IconCopy,
  IconDelete,
  IconEye,
  IconFile,
  IconImport,
  IconRefresh,
  IconSearch,
  IconSync
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import Pagination from '@/components/Pagination/index.vue'
import {
  delGenTable,
  downloadGenCode,
  getGenTable,
  importGenTable,
  listGen,
  listGenDb,
  previewGenTable,
  synchGenTable
} from '@/api/tool/gen'
import type { GenQuery, GenTable, GenTableColumn } from '@/api/tool/gen'
import { useCrud } from '@/hooks/useCrud'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Gen' })

const { t } = useI18n()

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'tableId', label: t('tool.gen.tableId'), width: 80 },
  { key: 'tableName', label: t('tool.gen.tableName'), minWidth: 180 },
  { key: 'tableComment', label: t('tool.gen.tableComment'), minWidth: 160, ellipsis: true, tooltip: true },
  { key: 'className', label: t('tool.gen.className'), width: 160, ellipsis: true, tooltip: true },
  { key: 'createTime', label: t('common.fields.createTime'), width: 165 },
  { key: 'updateTime', label: t('tool.gen.updateTime'), width: 165 },
  { key: 'operation', label: t('common.fields.operation'), width: 300 }
])

const crud = useCrud<GenTable, GenQuery>({
  listApi: listGen,
  deleteApi: delGenTable,
  pkField: 'tableId'
})

const { loading, list, total, page, limit, queryParams, getList, handleQuery, resetQuery, setSelection, multiple, handleDelete } =
  crud

function asTable(record: TableData): GenTable {
  return record as GenTable
}

function asColumn(record: TableData): GenTableColumn {
  return record as GenTableColumn
}

/* ---------- 导入表 ---------- */
const importModal = reactive<{ open: boolean; tableName: string; tableComment: string; tplWebType: string }>({
  open: false,
  tableName: '',
  tableComment: '',
  tplWebType: 'element-ui'
})
const importLoading = ref(false)
const dbTables = ref<GenTable[]>([])
const dbLoading = ref(false)
const dbPage = ref(1)
const dbLimit = ref(10)
const dbTotal = ref(0)
const dbSelected = ref<string[]>([])

const dbRowSelection = computed<TableRowSelection>(() => ({
  type: 'checkbox',
  showCheckedAll: true,
  selectedRowKeys: dbSelected.value,
  width: 44
}))

function openImport(): void {
  importModal.tableName = ''
  importModal.tableComment = ''
  dbSelected.value = []
  dbPage.value = 1
  importModal.open = true
  void loadDbTables()
}

async function loadDbTables(): Promise<void> {
  dbLoading.value = true
  try {
    const result = await listGenDb({
      pageNum: dbPage.value,
      pageSize: dbLimit.value,
      tableName: importModal.tableName || undefined,
      tableComment: importModal.tableComment || undefined
    })
    dbTables.value = result.rows ?? []
    dbTotal.value = result.total ?? 0
  } finally {
    dbLoading.value = false
  }
}

function onDbSelectionChange(keys: Array<string | number>): void {
  // row-key 为 tableName，选中键即表名集合
  dbSelected.value = keys.map(String)
}

async function submitImport(): Promise<void> {
  if (dbSelected.value.length === 0) {
    Message.warning(t('tool.gen.selectTablesFirst'))
    return
  }
  importLoading.value = true
  try {
    await importGenTable(dbSelected.value.join(','), importModal.tplWebType)
    Message.success(t('tool.gen.importSuccess'))
    importModal.open = false
    await getList()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    importLoading.value = false
  }
}

/* ---------- 预览 ---------- */
const preview = reactive<{ open: boolean; templates: Array<{ name: string; content: string }> }>({
  open: false,
  templates: []
})

function shortTemplateName(name: string): string {
  // 'vm/java/domain.java.vm' -> 'domain.java'
  const parts = name.split('/')
  const file = parts[parts.length - 1] ?? name
  return file.replace(/\.vm$/, '')
}

async function openPreview(row: GenTable): Promise<void> {
  preview.templates = []
  preview.open = true
  try {
    const data = await previewGenTable(row.tableId)
    preview.templates = Object.entries(data ?? {}).map(([name, content]) => ({ name, content }))
  } catch {
    // 失败提示已由响应拦截器统一弹出，弹窗内空态兜底
  }
}

async function copyCode(content: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(content)
    Message.success(t('common.codeCopied'))
  } catch {
    Message.warning(t('common.copyUnsupported'))
  }
}

/* ---------- 详情 ---------- */
const detail = reactive<{ open: boolean; table: GenTable | null; columns: GenTableColumn[] }>({
  open: false,
  table: null,
  columns: []
})
const detailLoading = ref(false)

async function openDetail(row: GenTable): Promise<void> {
  detail.table = row
  detail.columns = []
  detail.open = true
  detailLoading.value = true
  try {
    const data = await getGenTable(row.tableId)
    detail.table = { ...row, ...(data?.info ?? {}) }
    detail.columns = data?.rows ?? []
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    detailLoading.value = false
  }
}

/** 生成模板展示文案（crud/tree/sub） */
function tplCategoryLabel(value?: string): string {
  const labels: Record<string, string> = {
    crud: t('tool.gen.tplCrud'),
    tree: t('tool.gen.tplTree'),
    sub: t('tool.gen.tplSub')
  }
  return value ? (labels[value] ?? value) : '-'
}

/** 前端类型展示文案 */
function tplWebTypeLabel(value?: string): string {
  const labels: Record<string, string> = {
    'element-ui': t('tool.gen.tplElementUI'),
    'element-plus': t('tool.gen.tplElementPlus')
  }
  return value ? (labels[value] ?? value) : '-'
}

/* ---------- 同步/生成 ---------- */
function handleSynchDb(row: GenTable): void {
  Modal.confirm({
    title: t('tool.gen.syncConfirmTitle'),
    content: t('tool.gen.syncConfirm', { name: row.tableName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await synchGenTable(row.tableName ?? '')
        Message.success(t('tool.gen.syncSuccess'))
        await getList()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

async function handleDownload(row: GenTable): Promise<void> {
  try {
    await downloadGenCode(row.tableName ?? '', `ruoyi-${row.tableName}.zip`)
    Message.success(t('tool.gen.genSuccess'))
  } catch {
    // 失败提示已由 download.ts/拦截器提示
  }
}

/* ---------- 初始化 ---------- */
void getList()
</script>

<style scoped>
.gen-page__import-form {
  margin-bottom: 12px;
}

.gen-page__import-table {
  width: 100%;
}

.gen-page__import-page {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
}

.gen-page__code-wrap {
  display: flex;
  flex-direction: column;
}

.gen-page__code-actions {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 8px;
}

.gen-page__code {
  margin: 0;
  max-height: 460px;
  overflow: auto;
  padding: 12px;
  font-size: 12px;
  line-height: 1.6;
  background-color: var(--color-fill-2);
  border-radius: 4px;
  white-space: pre;
}

.gen-page__column-table {
  margin-top: 12px;
  width: 100%;
}
</style>
