<script setup lang="ts">
import { onMounted, ref } from 'vue';

import {
  ElButton,
  ElCol,
  ElDatePicker,
  ElDialog,
  ElForm,
  ElFormItem,
  ElIcon,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElRow,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
} from 'element-plus';
import {
  Delete,
  Download,
  Refresh,
  Search,
  View,
  Warning,
} from '@element-plus/icons-vue';

import {
  cleanOperlog,
  delOperlog,
  exportOperlog,
  listOperlog,
  type SysOperLog,
} from '#/api/monitor/operlog';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime, saveBlobFile } from '#/utils/ruoyi';

defineOptions({ name: 'MonitorOperlog' });

const { dictMap } = useDict({
  operType: 'sys_oper_type',
  status: 'sys_common_status',
});

const {
  queryParams,
  dateRange,
  total,
  handleQuery,
  resetQuery: resetQueryBase,
} = usePagination({
  // 与若依原版一致：支持按操作地址筛选
  operIp: '',
  title: '',
  operName: '',
  businessType: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysOperLog[]>([]);
const ids = ref<number[]>([]);
const multiple = ref(true);

const defaultSort = { prop: 'operTime', order: 'descending' } as const;
const orderByColumn = ref('operTime');
const isAsc = ref('desc');

/** 组装列表/导出共用的查询参数（含排序与时间范围） */
function buildQueryParams() {
  // 不传 propName：Mapper 读取 params.beginTime / params.endTime
  return addDateRange(
    {
      ...queryParams,
      orderByColumn: orderByColumn.value,
      isAsc: isAsc.value,
    },
    dateRange.value,
  );
}

async function getList() {
  loading.value = true;
  try {
    const res = await listOperlog(buildQueryParams());
    list.value = res.rows ?? [];
    total.value = res.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function handleSearch() {
  handleQuery();
  getList();
}

function handleResetQuery() {
  resetQueryBase();
  orderByColumn.value = defaultSort.prop;
  isAsc.value = 'desc';
  getList();
}

function handleSelectionChange(selection: SysOperLog[]) {
  ids.value = selection.map((item) => item.operId);
  multiple.value = !selection.length;
}

// 排序：取消排序时回退到默认按操作时间倒序
function handleSortChange({
  prop,
  order,
}: {
  prop: string;
  order: string | null;
}) {
  if (!order) {
    orderByColumn.value = defaultSort.prop;
    isAsc.value = 'desc';
  } else {
    orderByColumn.value = prop;
    isAsc.value = order === 'ascending' ? 'asc' : 'desc';
  }
  getList();
}

// 删除
async function handleDelete(row?: SysOperLog) {
  const operIds = row?.operId || ids.value;
  try {
    await ElMessageBox.confirm(
      `是否确认删除日志编号为"${operIds}"的数据项？`,
      '提示',
      { type: 'warning' },
    );
    await delOperlog(operIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

// 清空
async function handleClean() {
  try {
    await ElMessageBox.confirm('是否确认清空所有操作日志数据项？', '提示', {
      type: 'warning',
    });
    await cleanOperlog();
    getList();
    ElMessage.success('清空成功');
  } catch {
    /* 取消 */
  }
}

// 导出：按当前查询条件导出 Excel
async function handleExport() {
  try {
    await ElMessageBox.confirm('是否确认导出所有操作日志数据项？', '警告', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    });
  } catch {
    return;
  }
  const response: any = await exportOperlog(buildQueryParams());
  const ok = await saveBlobFile(response, `operlog_${Date.now()}.xlsx`);
  if (ok) {
    ElMessage.success('导出成功');
  }
}

// ===== 详情对话框 =====
const detailOpen = ref(false);
const detailForm = ref<Partial<SysOperLog>>({});

function handleDetail(row: SysOperLog) {
  detailForm.value = row;
  detailOpen.value = true;
}

function formatJson(str?: string) {
  if (!str) return '（无数据）';
  try {
    return JSON.stringify(JSON.parse(str), null, 2);
  } catch {
    return str;
  }
}

async function copyText(str?: string) {
  const text = formatJson(str);
  try {
    await navigator.clipboard.writeText(text);
    ElMessage.success('已复制');
  } catch {
    const ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
    ElMessage.success('已复制');
  }
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm
      :inline="true"
      :model="queryParams"
      size="small"
      class="search-form"
    >
      <ElFormItem label="操作地址">
        <ElInput
          v-model="queryParams.operIp"
          placeholder="请输入操作地址"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="系统模块">
        <ElInput
          v-model="queryParams.title"
          placeholder="请输入系统模块"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="操作人员">
        <ElInput
          v-model="queryParams.operName"
          placeholder="请输入操作人员"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="类型">
        <ElSelect
          v-model="queryParams.businessType"
          placeholder="操作类型"
          clearable
          style="width: 200px"
        >
          <ElOption
            v-for="d in dictMap.operType"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect
          v-model="queryParams.status"
          placeholder="操作状态"
          clearable
          style="width: 200px"
        >
          <ElOption
            v-for="d in dictMap.status"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="操作时间">
        <ElDatePicker
          v-model="dateRange"
          style="width: 240px"
          value-format="YYYY-MM-DD HH:mm:ss"
          type="daterange"
          range-separator="-"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          :default-time="[new Date(2000, 0, 1, 0, 0, 0), new Date(2000, 0, 1, 23, 59, 59)]"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">
          搜索
        </ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        :disabled="multiple"
        v-hasPermi="['monitor:operlog:remove']"
        @click="handleDelete()"
      >
        删除
      </ElButton>
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        v-hasPermi="['monitor:operlog:remove']"
        @click="handleClean"
      >
        清空
      </ElButton>
      <ElButton
        type="warning"
        plain
        :icon="Download"
        v-hasPermi="['monitor:operlog:export']"
        @click="handleExport"
      >
        导出
      </ElButton>
    </div>

    <ElTable
      v-loading="loading"
      :data="list"
      border
      :default-sort="defaultSort"
      @selection-change="handleSelectionChange"
      @sort-change="handleSortChange"
    >
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="日志编号" align="center" prop="operId" width="90" />
      <ElTableColumn
        label="系统模块"
        align="center"
        prop="title"
        show-overflow-tooltip
      />
      <ElTableColumn label="操作类型" align="center" prop="businessType">
        <template #default="{ row }">
          <DictTag :options="dictMap.operType" :value="row.businessType" />
        </template>
      </ElTableColumn>
      <ElTableColumn
        label="操作人员"
        align="center"
        prop="operName"
        width="110"
        show-overflow-tooltip
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      />
      <ElTableColumn
        label="操作地址"
        align="center"
        prop="operIp"
        width="130"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="操作地点"
        align="center"
        prop="operLocation"
        show-overflow-tooltip
      />
      <ElTableColumn label="操作状态" align="center" prop="status">
        <template #default="{ row }">
          <DictTag :options="dictMap.status" :value="row.status" />
        </template>
      </ElTableColumn>
      <ElTableColumn
        label="操作日期"
        align="center"
        prop="operTime"
        width="160"
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      >
        <template #default="{ row }">
          {{ parseTime(row.operTime) }}
        </template>
      </ElTableColumn>
      <ElTableColumn
        label="消耗时间"
        align="center"
        prop="costTime"
        width="110"
        show-overflow-tooltip
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      >
        <template #default="{ row }">{{ row.costTime }}毫秒</template>
      </ElTableColumn>
      <ElTableColumn label="操作" align="center" width="100" fixed="right">
        <template #default="{ row }">
          <ElButton
            link
            type="primary"
            size="small"
            :icon="View"
            v-hasPermi="['monitor:operlog:query']"
            @click="handleDetail(row)"
          >
            详细
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <ElPagination
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

    <!-- 详情对话框 -->
    <ElDialog
      v-model="detailOpen"
      title="操作日志详细"
      width="780px"
      append-to-body
    >
      <div class="detail-wrap">
        <!-- 基本信息 -->
        <div class="detail-card">
          <div class="detail-card-title">基本信息</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">操作模块</span>
                <span class="detail-value">{{ detailForm.title }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">业务类型</span>
                <span class="detail-value">
                  <DictTag
                    :options="dictMap.operType"
                    :value="detailForm.businessType"
                  />
                </span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">操作时间</span>
                <span class="detail-value">
                  {{ parseTime(detailForm.operTime) }}
                </span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">执行状态</span>
                <span class="detail-value">
                  <ElTag
                    v-if="detailForm.status === 0"
                    type="success"
                    size="small"
                  >
                    正常
                  </ElTag>
                  <ElTag v-else type="danger" size="small">异常</ElTag>
                </span>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <!-- 操作人员 -->
        <div class="detail-card">
          <div class="detail-card-title">操作人员</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">操作人员</span>
                <span class="detail-value">{{ detailForm.operName }}</span>
              </div>
            </ElCol>
            <ElCol v-if="detailForm.deptName" :span="12">
              <div class="detail-item">
                <span class="detail-label">所属部门</span>
                <span class="detail-value">{{ detailForm.deptName }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="24">
              <div class="detail-item">
                <span class="detail-label">操作地址</span>
                <span class="detail-value">
                  {{ detailForm.operIp }}
                  <span v-if="detailForm.operLocation" class="detail-location">
                    （{{ detailForm.operLocation }}）
                  </span>
                </span>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <!-- 请求信息 -->
        <div class="detail-card">
          <div class="detail-card-title">请求信息</div>
          <ElRow class="detail-row">
            <ElCol :span="24">
              <div class="detail-item">
                <span class="detail-label">请求地址</span>
                <span class="detail-value">
                  <ElTag size="small" style="margin-right: 6px">
                    {{ detailForm.requestMethod }}
                  </ElTag>
                  {{ detailForm.operUrl }}
                </span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="24">
              <div class="detail-item">
                <span class="detail-label">操作方法</span>
                <span class="detail-value mono">{{ detailForm.method }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">消耗时间</span>
                <span class="detail-value">
                  {{ detailForm.costTime }} 毫秒
                </span>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <!-- 请求参数 -->
        <div class="detail-card">
          <div class="detail-card-title">
            请求参数
            <ElButton
              link
              type="primary"
              size="small"
              @click="copyText(detailForm.operParam)"
            >
              复制
            </ElButton>
          </div>
          <pre class="code-pre">{{ formatJson(detailForm.operParam) }}</pre>
        </div>

        <!-- 返回参数 -->
        <div class="detail-card">
          <div class="detail-card-title">
            返回参数
            <ElButton
              link
              type="primary"
              size="small"
              @click="copyText(detailForm.jsonResult)"
            >
              复制
            </ElButton>
          </div>
          <pre class="code-pre">{{ formatJson(detailForm.jsonResult) }}</pre>
        </div>

        <!-- 异常信息 -->
        <div v-if="detailForm.status !== 0" class="detail-card">
          <div class="detail-card-title error-title">
            <ElIcon><Warning /></ElIcon> 异常信息
          </div>
          <div class="error-body">{{ detailForm.errorMsg }}</div>
        </div>
      </div>
      <template #footer>
        <ElButton @click="detailOpen = false">关 闭</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';

.detail-wrap {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.detail-card {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px 16px;
}

.detail-card-title {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.detail-card-title.error-title {
  color: var(--el-color-danger);
}

.detail-row {
  margin-bottom: 8px;
}

.detail-item {
  display: flex;
  font-size: 13px;
  line-height: 24px;
}

.detail-label {
  width: 80px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.detail-value {
  flex: 1;
  word-break: break-all;
}

.detail-location {
  color: var(--el-text-color-secondary);
}

.mono {
  font-family: 'Consolas', 'Menlo', monospace;
  font-size: 12px;
}

.code-pre {
  margin: 0;
  padding: 10px;
  background: var(--el-bg-color-page);
  border-radius: 4px;
  font-family: 'Consolas', 'Menlo', monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow: auto;
}

.error-body {
  font-size: 13px;
  color: var(--el-color-danger);
  word-break: break-all;
  background: var(--el-color-danger-light-9);
  padding: 10px;
  border-radius: 4px;
}
</style>
