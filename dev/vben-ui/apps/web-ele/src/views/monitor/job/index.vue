<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSelect,
  ElSwitch,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete } from '@element-plus/icons-vue';

import { changeJobStatus, delJob, getJob, listJob, runJob, updateJob, addJob, type SysJob } from '#/api/monitor/job';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';

defineOptions({ name: 'MonitorJob' });

const { dictMap } = useDict({ group: 'sys_job_group', status: 'sys_job_status' });
const { queryParams, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  jobName: '',
  jobGroup: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysJob[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const res = await listJob(queryParams);
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
function handleSelectionChange(sel: SysJob[]) {
  ids.value = sel.map((i) => i.jobId);
  single.value = sel.length !== 1;
  multiple.value = !sel.length;
}

// 状态切换
async function handleStatusChange(row: SysJob) {
  const text = row.status === '0' ? '启用' : '停用';
  try {
    await ElMessageBox.confirm(`确认要"${text}""${row.jobName}"任务吗？`, '提示', { type: 'warning' });
    await changeJobStatus(row.jobId, row.status!);
    ElMessage.success(text + '成功');
  } catch {
    row.status = row.status === '0' ? '1' : '0';
  }
}

// 立即执行一次
async function handleRun(row: SysJob) {
  try {
    await ElMessageBox.confirm(`确认要立即执行一次"${row.jobName}"任务吗？`, '提示', { type: 'warning' });
    await runJob(row.jobId, row.jobGroup);
    ElMessage.success('执行成功');
  } catch {
    /* 取消 */
  }
}

const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysJob>>({});
const rules = {
  jobName: [{ required: true, message: '任务名称不能为空', trigger: 'blur' }],
  invokeTarget: [{ required: true, message: '调用目标字符串不能为空', trigger: 'blur' }],
  cronExpression: [{ required: true, message: 'cron表达式不能为空', trigger: 'blur' }],
};

function reset() {
  Object.assign(form, {
    jobId: undefined,
    jobName: '',
    jobGroup: 'DEFAULT',
    invokeTarget: '',
    cronExpression: '',
    misfirePolicy: '3',
    concurrent: '1',
    status: '0',
    remark: '',
  });
  formRef.value?.resetFields();
}

function handleAdd() {
  reset();
  open.value = true;
  title.value = '添加任务';
}

async function handleUpdate(row?: SysJob) {
  reset();
  const id = row?.jobId ?? ids.value[0]!;
  const res = await getJob(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改任务';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.jobId) {
    await updateJob(form);
    ElMessage.success('修改成功');
  } else {
    await addJob(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysJob) {
  const jIds = row.jobId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除定时任务编号为"${jIds}"的数据项？`, '提示', { type: 'warning' });
    await delJob(jIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="任务名称">
        <ElInput v-model="queryParams.jobName" placeholder="请输入任务名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="任务组名">
        <ElSelect v-model="queryParams.jobGroup" placeholder="任务组名" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.group" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="任务状态">
        <ElSelect v-model="queryParams.status" placeholder="任务状态" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['monitor:job:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['monitor:job:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['monitor:job:remove']" @click="handleDelete({} as SysJob)">删除</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="任务编号" prop="jobId" width="90" align="center" />
      <ElTableColumn label="任务名称" prop="jobName" show-overflow-tooltip />
      <ElTableColumn label="任务组名" prop="jobGroup" width="100" align="center">
        <template #default="{ row }"><DictTag :options="dictMap.group" :value="row.jobGroup" /></template>
      </ElTableColumn>
      <ElTableColumn label="调用目标字符串" prop="invokeTarget" show-overflow-tooltip />
      <ElTableColumn label="cron执行表达式" prop="cronExpression" width="160" />
      <ElTableColumn label="状态" width="100" align="center">
        <template #default="{ row }">
          <ElSwitch v-model="row.status" active-value="0" inactive-value="1" @change="handleStatusChange(row)" />
        </template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="240" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['monitor:job:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="primary" size="small" v-hasPermi="['monitor:job:changeStatus']" @click="handleRun(row)">执行一次</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['monitor:job:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="700px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="120px">
        <el-row>
          <el-col :span="12">
            <ElFormItem label="任务名称" prop="jobName"><ElInput v-model="form.jobName" placeholder="请输入任务名称" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="任务组名">
              <ElSelect v-model="form.jobGroup">
                <ElOption v-for="d in dictMap.group" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
              </ElSelect>
            </ElFormItem>
          </el-col>
        </el-row>
        <ElFormItem label="调用方法" prop="invokeTarget"><ElInput v-model="form.invokeTarget" placeholder="如 ryTask.ryParams('ry')" /></ElFormItem>
        <ElFormItem label="cron表达式" prop="cronExpression"><ElInput v-model="form.cronExpression" placeholder="如 0/10 * * * * ?" /></ElFormItem>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="执行策略">
              <ElSelect v-model="form.misfirePolicy">
                <ElOption value="1" label="立即执行" />
                <ElOption value="2" label="执行一次" />
                <ElOption value="3" label="放弃执行" />
              </ElSelect>
            </ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="是否并发">
              <ElSelect v-model="form.concurrent">
                <ElOption value="0" label="允许" />
                <ElOption value="1" label="禁止" />
              </ElSelect>
            </ElFormItem>
          </el-col>
        </el-row>
        <ElFormItem label="备注"><ElInput v-model="form.remark" type="textarea" placeholder="请输入内容" /></ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="open = false">取 消</ElButton>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';
</style>
