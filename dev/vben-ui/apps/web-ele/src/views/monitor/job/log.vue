<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import {
  ElButton,
  ElDatePicker,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSelect,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Close, Delete, Download, Refresh, Search } from '@element-plus/icons-vue';

import { getJob } from '#/api/monitor/job';
import {
  cleanJobLog,
  delJobLog,
  exportJobLog,
  listJobLog,
  type SysJobLog,
} from '#/api/monitor/jobLog';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime, saveBlobFile } from '#/utils/ruoyi';

import JobDetail from './detail.vue';

defineOptions({ name: 'MonitorJobLog' });

const route = useRoute();
const router = useRouter();

const { dictMap } = useDict({
  group: 'sys_job_group',
  status: 'sys_common_status',
});

const {
  queryParams,
  dateRange,
  total,
  handleQuery,
  resetQuery: resetQueryBase,
} = usePagination({
  jobName: '',
  jobGroup: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysJobLog[]>([]);
const ids = ref<number[]>([]);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const params = addDateRange({ ...queryParams }, dateRange.value);
    const res = await listJobLog(params);
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
  getList();
}

function handleSelectionChange(selection: SysJobLog[]) {
  ids.value = selection.map((item) => item.jobLogId);
  multiple.value = !selection.length;
}

const open = ref(false);
const detailForm = ref<Partial<SysJobLog>>({});

function handleView(row: SysJobLog) {
  detailForm.value = row;
  open.value = true;
}

async function handleDelete() {
  const jobLogIds = ids.value;
  try {
    await ElMessageBox.confirm(
      `是否确认删除调度日志编号为"${jobLogIds}"的数据项？`,
      '提示',
      { type: 'warning' },
    );
    await delJobLog(jobLogIds);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

async function handleClean() {
  try {
    await ElMessageBox.confirm('是否确认清空所有调度日志数据项？', '提示', {
      type: 'warning',
    });
    await cleanJobLog();
    getList();
    ElMessage.success('清空成功');
  } catch {
    /* 取消 */
  }
}

async function handleExport() {
  try {
    await ElMessageBox.confirm('是否确认导出所有调度日志数据项？', '警告', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    });
  } catch {
    return;
  }
  const params = addDateRange({ ...queryParams }, dateRange.value);
  const response: any = await exportJobLog(params);
  const ok = await saveBlobFile(response, `log_${Date.now()}.xlsx`);
  if (ok) {
    ElMessage.success('导出成功');
  }
}

function handleClose() {
  router.push('/monitor/job');
}

onMounted(async () => {
  // 路由参数 jobId：非 0 时按该任务名称/分组过滤日志
  const jobIdParam = route.params?.jobId;
  const jobId = Number(jobIdParam);
  if (jobIdParam !== undefined && !Number.isNaN(jobId) && jobId !== 0) {
    try {
      const res = await getJob(jobId);
      queryParams.jobName = res.data?.jobName ?? '';
      queryParams.jobGroup = res.data?.jobGroup ?? '';
    } catch {
      /* 任务不存在时仍加载全部日志 */
    }
  }
  getList();
});
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="任务名称">
        <ElInput
          v-model="queryParams.jobName"
          placeholder="请输入任务名称"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="任务组名">
        <ElSelect
          v-model="queryParams.jobGroup"
          placeholder="请选择任务组名"
          clearable
          style="width: 200px"
        >
          <ElOption
            v-for="d in dictMap.group"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="执行状态">
        <ElSelect
          v-model="queryParams.status"
          placeholder="请选择执行状态"
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
      <ElFormItem label="执行时间">
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
        type="danger"
        plain
        :icon="Delete"
        :disabled="multiple"
        v-hasPermi="['monitor:job:remove']"
        @click="handleDelete"
      >
        删除
      </ElButton>
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        v-hasPermi="['monitor:job:remove']"
        @click="handleClean"
      >
        清空
      </ElButton>
      <ElButton
        type="warning"
        plain
        :icon="Download"
        v-hasPermi="['monitor:job:export']"
        @click="handleExport"
      >
        导出
      </ElButton>
      <ElButton type="warning" plain :icon="Close" @click="handleClose">关闭</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="日志编号" prop="jobLogId" width="90" align="center" />
      <ElTableColumn label="任务名称" prop="jobName" show-overflow-tooltip />
      <ElTableColumn label="任务组名" prop="jobGroup" width="100" align="center">
        <template #default="{ row }">
          <DictTag :options="dictMap.group" :value="row.jobGroup" />
        </template>
      </ElTableColumn>
      <ElTableColumn label="调用目标字符串" prop="invokeTarget" show-overflow-tooltip />
      <ElTableColumn label="日志信息" prop="jobMessage" show-overflow-tooltip />
      <ElTableColumn label="执行状态" prop="status" width="100" align="center">
        <template #default="{ row }">
          <DictTag :options="dictMap.status" :value="row.status" />
        </template>
      </ElTableColumn>
      <ElTableColumn label="执行时间" prop="createTime" width="180" align="center">
        <template #default="{ row }">
          <span>{{ parseTime(row.createTime) }}</span>
        </template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="100" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton
            link
            type="primary"
            size="small"
            v-hasPermi="['monitor:job:query']"
            @click="handleView(row)"
          >
            详细
          </ElButton>
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

    <JobDetail v-model:visible="open" :row="detailForm" type="log" />
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';
</style>
