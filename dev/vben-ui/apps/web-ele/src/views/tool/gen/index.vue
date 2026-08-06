<script setup lang="ts">
import { onActivated, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import {
  ElButton,
  ElDatePicker,
  ElDialog,
  ElForm,
  ElFormItem,
  ElInput,
  ElLink,
  ElMessage,
  ElMessageBox,
  ElTabPane,
  ElTable,
  ElTableColumn,
  ElTabs,
  ElTooltip,
} from 'element-plus';
import {
  Delete,
  DocumentCopy,
  Download,
  Edit,
  Plus,
  Refresh,
  Search,
  Upload,
  View,
} from '@element-plus/icons-vue';

import {
  delTable,
  downloadBatchGenCode,
  genCode,
  listTable,
  previewTable,
  synchDb,
  type GenTable,
} from '#/api/tool/gen';
import { usePagination } from '#/composables/usePagination';
import { addDateRange, saveBlobFile } from '#/utils/ruoyi';

import CreateTable from './createTable.vue';
import ImportTable from './importTable.vue';

/**
 * 代码生成列表页
 * 移植自 RuoYi-Vue3 views/tool/gen/index.vue，按 Vben 约定适配。
 */
defineOptions({ name: 'ToolGen' });

const route = useRoute();
const router = useRouter();

const {
  queryParams,
  dateRange,
  total,
  handleQuery: resetPage,
  resetQuery: resetQueryBase,
} = usePagination({
  tableName: '',
  tableComment: '',
  orderByColumn: 'createTime',
  isAsc: 'descending',
});

const loading = ref(false);
const showSearch = ref(true);
const tableList = ref<GenTable[]>([]);
const ids = ref<number[]>([]);
const tableNames = ref<string[]>([]);
const single = ref(true);
const multiple = ref(true);
const uniqueId = ref('');
const defaultSort = { prop: 'createTime', order: 'descending' as const };
const genRef = ref();
const importRef = ref<InstanceType<typeof ImportTable>>();
const createRef = ref<InstanceType<typeof CreateTable>>();

const preview = reactive({
  open: false,
  title: '代码预览',
  data: {} as Record<string, string>,
  activeName: 'domain.java',
});

async function getList() {
  loading.value = true;
  try {
    const params = addDateRange({ ...queryParams }, dateRange.value);
    const res = await listTable(params);
    tableList.value = res.rows ?? [];
    total.value = res.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function handleSearch() {
  resetPage();
  getList();
}

function handleResetQuery() {
  resetQueryBase();
  queryParams.orderByColumn = defaultSort.prop;
  queryParams.isAsc = defaultSort.order;
  genRef.value?.sort(defaultSort.prop, defaultSort.order);
  getList();
}

function handleSelectionChange(selection: GenTable[]) {
  ids.value = selection.map((item) => item.tableId);
  tableNames.value = selection.map((item) => item.tableName);
  single.value = selection.length !== 1;
  multiple.value = !selection.length;
}

function handleSortChange(column: { prop: string; order: string | null }) {
  queryParams.orderByColumn = column.prop;
  queryParams.isAsc = column.order || '';
  getList();
}

/** 预览代码：拦截器已解包 data，直接拿到模板映射 */
async function handlePreview(row: GenTable) {
  const data = await previewTable(row.tableId);
  preview.data = data ?? {};
  preview.open = true;
  preview.activeName = 'domain.java';
}

function previewTabLabel(key: string) {
  return key.substring(key.lastIndexOf('/') + 1, key.indexOf('.vm'));
}

async function copyPreviewCode(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    ElMessage.success('复制成功');
  } catch {
    ElMessage.error('复制失败');
  }
}

/** 生成代码：自定义路径走 genCode；否则下载 zip */
async function handleGenTable(row?: GenTable) {
  const tbNames = row?.tableName || tableNames.value;
  if (!tbNames || (Array.isArray(tbNames) && tbNames.length === 0)) {
    ElMessage.error('请选择要生成的数据');
    return;
  }
  if (row?.genType === '1') {
    await genCode(row.tableName);
    ElMessage.success(`成功生成到自定义路径：${row.genPath}`);
    return;
  }
  const tablesParam = Array.isArray(tbNames) ? tbNames.join(',') : tbNames;
  const zipName = Array.isArray(tbNames) ? 'ruoyi.zip' : `${tbNames}.zip`;
  const response = await downloadBatchGenCode(tablesParam);
  await saveBlobFile(response as any, zipName);
}

async function handleSynchDb(row: GenTable) {
  try {
    await ElMessageBox.confirm(`确认要强制同步"${row.tableName}"表结构吗？`, '提示', {
      type: 'warning',
    });
    await synchDb(row.tableName);
    ElMessage.success('同步成功');
  } catch {
    /* 取消 */
  }
}

function openImportTable() {
  importRef.value?.show();
}

function openCreateTable() {
  createRef.value?.show();
}

/** 跳转隐藏路由编辑页（builtinMenus 注入 /tool/gen-edit） */
function handleEditTable(row?: GenTable) {
  const tableId = row?.tableId ?? ids.value[0];
  const tableName = row?.tableName ?? tableNames.value[0];
  if (!tableId) {
    ElMessage.error('请选择要修改的数据');
    return;
  }
  router.push({
    path: `/tool/gen-edit/index/${tableId}`,
    query: { pageNum: String(queryParams.pageNum), t: String(Date.now()) },
  });
  // 标题由路由 meta 控制；这里用 document.title 辅助可读性
  void tableName;
}

async function handleDelete(row?: GenTable) {
  const tableIds = row?.tableId ?? ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除表编号为"${tableIds}"的数据项？`, '提示', {
      type: 'warning',
    });
    await delTable(tableIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

/** 从编辑页返回时带 t 参数刷新列表 */
onActivated(() => {
  const time = route.query.t;
  if (time != null && String(time) !== uniqueId.value) {
    uniqueId.value = String(time);
    if (route.query.pageNum) {
      queryParams.pageNum = Number(route.query.pageNum);
    }
    dateRange.value = [];
    getList();
  }
});

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm
      v-show="showSearch"
      :inline="true"
      :model="queryParams"
      size="small"
      class="search-form"
    >
      <ElFormItem label="表名称">
        <ElInput
          v-model="queryParams.tableName"
          placeholder="请输入表名称"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="表描述">
        <ElInput
          v-model="queryParams.tableComment"
          placeholder="请输入表描述"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="创建时间">
        <ElDatePicker
          v-model="dateRange"
          style="width: 240px"
          value-format="YYYY-MM-DD"
          type="daterange"
          range-separator="-"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton
        type="primary"
        plain
        :icon="Download"
        :disabled="multiple"
        v-hasPermi="['tool:gen:code']"
        @click="handleGenTable()"
      >
        生成
      </ElButton>
      <ElButton type="primary" plain :icon="Plus" v-hasRole="['admin']" @click="openCreateTable">
        创建
      </ElButton>
      <ElButton
        type="info"
        plain
        :icon="Upload"
        v-hasPermi="['tool:gen:import']"
        @click="openImportTable"
      >
        导入
      </ElButton>
      <ElButton
        type="success"
        plain
        :icon="Edit"
        :disabled="single"
        v-hasPermi="['tool:gen:edit']"
        @click="handleEditTable()"
      >
        修改
      </ElButton>
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        :disabled="multiple"
        v-hasPermi="['tool:gen:remove']"
        @click="handleDelete()"
      >
        删除
      </ElButton>
    </div>

    <ElTable
      ref="genRef"
      v-loading="loading"
      :data="tableList"
      border
      :default-sort="defaultSort"
      @selection-change="handleSelectionChange"
      @sort-change="handleSortChange"
    >
      <ElTableColumn type="selection" width="55" align="center" />
      <ElTableColumn label="序号" type="index" width="50" align="center">
        <template #default="{ $index }">
          <span>{{ (queryParams.pageNum - 1) * queryParams.pageSize + $index + 1 }}</span>
        </template>
      </ElTableColumn>
      <ElTableColumn label="表名称" align="center" prop="tableName" show-overflow-tooltip />
      <ElTableColumn label="表描述" align="center" prop="tableComment" show-overflow-tooltip />
      <ElTableColumn label="实体" align="center" prop="className" show-overflow-tooltip />
      <ElTableColumn
        label="创建时间"
        align="center"
        prop="createTime"
        width="160"
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      />
      <ElTableColumn
        label="更新时间"
        align="center"
        prop="updateTime"
        width="160"
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      />
      <ElTableColumn label="操作" align="center" width="280" fixed="right">
        <template #default="{ row }">
          <ElTooltip content="预览" placement="top">
            <ElButton
              link
              type="primary"
              :icon="View"
              v-hasPermi="['tool:gen:preview']"
              @click="handlePreview(row)"
            />
          </ElTooltip>
          <ElTooltip content="编辑" placement="top">
            <ElButton
              link
              type="primary"
              :icon="Edit"
              v-hasPermi="['tool:gen:edit']"
              @click="handleEditTable(row)"
            />
          </ElTooltip>
          <ElTooltip content="删除" placement="top">
            <ElButton
              link
              type="primary"
              :icon="Delete"
              v-hasPermi="['tool:gen:remove']"
              @click="handleDelete(row)"
            />
          </ElTooltip>
          <ElTooltip content="同步" placement="top">
            <ElButton
              link
              type="primary"
              :icon="Refresh"
              v-hasPermi="['tool:gen:edit']"
              @click="handleSynchDb(row)"
            />
          </ElTooltip>
          <ElTooltip content="生成代码" placement="top">
            <ElButton
              link
              type="primary"
              :icon="Download"
              v-hasPermi="['tool:gen:code']"
              @click="handleGenTable(row)"
            />
          </ElTooltip>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination
        v-model:current-page="queryParams.pageNum"
        v-model:page-size="queryParams.pageSize"
        :total="total"
        :page-sizes="[10, 20, 30, 50]"
        layout="total, sizes, prev, pager, next, jumper"
        background
        @size-change="getList"
        @current-change="getList"
      />
    </div>

    <ElDialog v-model="preview.open" :title="preview.title" width="80%" top="5vh" append-to-body>
      <ElTabs v-model="preview.activeName">
        <ElTabPane
          v-for="(value, key) in preview.data"
          :key="key"
          :label="previewTabLabel(key)"
          :name="previewTabLabel(key)"
        >
          <ElLink
            :underline="false"
            :icon="DocumentCopy"
            style="float: right"
            @click="copyPreviewCode(value)"
          >
            复制
          </ElLink>
          <pre class="preview-code">{{ value }}</pre>
        </ElTabPane>
      </ElTabs>
    </ElDialog>

    <ImportTable ref="importRef" @ok="handleSearch" />
    <CreateTable ref="createRef" @ok="handleSearch" />
  </div>
</template>

<style scoped>
.preview-code {
  max-height: 65vh;
  overflow: auto;
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
