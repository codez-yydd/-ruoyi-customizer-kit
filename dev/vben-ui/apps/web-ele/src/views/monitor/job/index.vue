<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';

import {
  ElButton,
  ElCol,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElRadio,
  ElRadioButton,
  ElRadioGroup,
  ElRow,
  ElSelect,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTooltip,
} from 'element-plus';
import {
  Delete,
  Download,
  Edit,
  Plus,
  Refresh,
  Search,
  Operation,
} from '@element-plus/icons-vue';

import {
  addJob,
  changeJobStatus,
  delJob,
  exportJob,
  getJob,
  listJob,
  runJob,
  updateJob,
  type SysJob,
} from '#/api/monitor/job';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { saveBlobFile } from '#/utils/ruoyi';

import JobDetail from './detail.vue';

defineOptions({ name: 'MonitorJob' });

const router = useRouter();
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

/** 启停任务：开关已先改值，取消或失败时回滚 */
async function handleStatusChange(row: SysJob) {
  const text = row.status === '0' ? '启用' : '停用';
  try {
    await ElMessageBox.confirm(`确认要"${text}""${row.jobName}"任务吗？`, '提示', {
      type: 'warning',
    });
    await changeJobStatus(row.jobId, row.status!);
    ElMessage.success(`${text}成功`);
  } catch {
    row.status = row.status === '0' ? '1' : '0';
  }
}

/** 立即执行一次（需 changeStatus 权限，与若依后端一致） */
async function handleRun(row: SysJob) {
  try {
    await ElMessageBox.confirm(`确认要立即执行一次"${row.jobName}"任务吗？`, '提示', {
      type: 'warning',
    });
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
  // 仅清空数据；校验态在弹框打开后 nextTick 清除，避免「改→取消→新增」残留
  Object.assign(form, {
    jobId: undefined,
    jobName: '',
    jobGroup: 'DEFAULT',
    invokeTarget: '',
    cronExpression: '',
    misfirePolicy: '1',
    concurrent: '1',
    status: '0',
    remark: '',
  });
}

function handleAdd() {
  reset();
  open.value = true;
  title.value = '添加任务';
  nextTick(() => formRef.value?.clearValidate());
}

async function handleUpdate(row?: SysJob) {
  reset();
  const id = row?.jobId ?? ids.value[0]!;
  const res = await getJob(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改任务';
  nextTick(() => formRef.value?.clearValidate());
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

async function handleDelete(row?: SysJob) {
  const jIds = row?.jobId ?? ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除定时任务编号为"${jIds}"的数据项？`, '提示', {
      type: 'warning',
    });
    await delJob(jIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

async function handleExport() {
  try {
    await ElMessageBox.confirm('是否确认导出所有定时任务数据项？', '警告', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    });
  } catch {
    return;
  }
  const response: any = await exportJob({ ...queryParams });
  const ok = await saveBlobFile(response, `job_${Date.now()}.xlsx`);
  if (ok) {
    ElMessage.success('导出成功');
  }
}

/** 跳转调度日志；jobId=0 表示查看全部 */
function handleJobLog(row?: SysJob) {
  const jobId = row?.jobId ?? 0;
  router.push(`/monitor/job-log/index/${jobId}`);
}

// ===== 任务详情 =====
const openView = ref(false);
const viewForm = ref<Partial<SysJob>>({});

async function handleView(row: SysJob) {
  const res = await getJob(row.jobId);
  viewForm.value = res.data ?? {};
  openView.value = true;
}

// ===== Cron 常用表达式选择 =====
const openCron = ref(false);
const cronDraft = ref('');

const cronPresets = [
  { label: '每秒', value: '* * * * * ?' },
  { label: '每10秒', value: '0/10 * * * * ?' },
  { label: '每分钟', value: '0 * * * * ?' },
  { label: '每5分钟', value: '0 0/5 * * * ?' },
  { label: '每小时', value: '0 0 * * * ?' },
  { label: '每天0点', value: '0 0 0 * * ?' },
  { label: '每天1点', value: '0 0 1 * * ?' },
  { label: '每周一0点', value: '0 0 0 ? * MON' },
  { label: '每月1日0点', value: '0 0 0 1 * ?' },
];

function handleShowCron() {
  cronDraft.value = form.cronExpression || '';
  openCron.value = true;
}

function applyCronPreset(value: string) {
  cronDraft.value = value;
}

function confirmCron() {
  form.cronExpression = cronDraft.value;
  openCron.value = false;
}

function handleCommand(command: string, row: SysJob) {
  if (command === 'handleRun') {
    handleRun(row);
  } else if (command === 'handleJobLog') {
    handleJobLog(row);
  }
}

onMounted(getList);
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
        <ElSelect v-model="queryParams.jobGroup" placeholder="任务组名" clearable style="width: 200px">
          <ElOption
            v-for="d in dictMap.group"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="任务状态">
        <ElSelect v-model="queryParams.status" placeholder="任务状态" clearable style="width: 200px">
          <ElOption
            v-for="d in dictMap.status"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['monitor:job:add']" @click="handleAdd">
        新增
      </ElButton>
      <ElButton
        type="success"
        plain
        :icon="Edit"
        :disabled="single"
        v-hasPermi="['monitor:job:edit']"
        @click="handleUpdate()"
      >
        修改
      </ElButton>
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        :disabled="multiple"
        v-hasPermi="['monitor:job:remove']"
        @click="handleDelete()"
      >
        删除
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
      <ElButton
        type="info"
        plain
        :icon="Operation"
        v-hasPermi="['monitor:job:query']"
        @click="handleJobLog()"
      >
        日志
      </ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="任务编号" prop="jobId" width="90" align="center" />
      <ElTableColumn label="任务名称" prop="jobName" show-overflow-tooltip>
        <template #default="{ row }">
          <a class="link-type" @click="handleView(row)">{{ row.jobName }}</a>
        </template>
      </ElTableColumn>
      <ElTableColumn label="任务组名" prop="jobGroup" width="100" align="center">
        <template #default="{ row }">
          <DictTag :options="dictMap.group" :value="row.jobGroup" />
        </template>
      </ElTableColumn>
      <ElTableColumn label="调用目标字符串" prop="invokeTarget" show-overflow-tooltip />
      <ElTableColumn label="cron执行表达式" prop="cronExpression" width="160" show-overflow-tooltip />
      <ElTableColumn label="状态" width="100" align="center">
        <template #default="{ row }">
          <ElSwitch
            v-model="row.status"
            active-value="0"
            inactive-value="1"
            @change="handleStatusChange(row)"
          />
        </template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="220" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton
            link
            type="primary"
            size="small"
            v-hasPermi="['monitor:job:edit']"
            @click="handleUpdate(row)"
          >
            修改
          </ElButton>
          <ElButton
            link
            type="danger"
            size="small"
            v-hasPermi="['monitor:job:remove']"
            @click="handleDelete(row)"
          >
            删除
          </ElButton>
          <ElDropdown
            v-hasPermi="['monitor:job:changeStatus', 'monitor:job:query']"
            @command="(cmd: string) => handleCommand(cmd, row)"
          >
            <ElButton link type="primary" size="small">更多</ElButton>
            <template #dropdown>
              <ElDropdownMenu>
                <ElDropdownItem command="handleRun" v-hasPermi="['monitor:job:changeStatus']">
                  执行一次
                </ElDropdownItem>
                <ElDropdownItem command="handleJobLog" v-hasPermi="['monitor:job:query']">
                  调度日志
                </ElDropdownItem>
              </ElDropdownMenu>
            </template>
          </ElDropdown>
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

    <el-dialog v-model="open" :title="title" width="800px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="120px">
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="任务名称" prop="jobName">
              <ElInput v-model="form.jobName" placeholder="请输入任务名称" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="任务分组" prop="jobGroup">
              <ElSelect v-model="form.jobGroup" placeholder="请选择任务分组" style="width: 100%">
                <ElOption
                  v-for="d in dictMap.group"
                  :key="d.dictValue"
                  :label="d.dictLabel"
                  :value="d.dictValue"
                />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="24">
            <ElFormItem prop="invokeTarget">
              <template #label>
                <span>
                  调用方法
                  <ElTooltip placement="top">
                    <template #content>
                      <div>Bean调用示例：ryTask.ryParams('ry')</div>
                      <div>Class类调用示例：com.ruoyi.quartz.task.RyTask.ryParams('ry')</div>
                      <div>参数说明：支持字符串，布尔类型，长整型，浮点型，整型</div>
                    </template>
                    <span class="label-tip">?</span>
                  </ElTooltip>
                </span>
              </template>
              <ElInput v-model="form.invokeTarget" placeholder="请输入调用目标字符串" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="24">
            <ElFormItem label="cron表达式" prop="cronExpression">
              <div class="cron-input-row">
                <ElInput v-model="form.cronExpression" placeholder="请输入cron执行表达式" />
                <ElButton type="primary" @click="handleShowCron">生成表达式</ElButton>
              </div>
            </ElFormItem>
          </ElCol>
          <ElCol v-if="form.jobId !== undefined" :span="24">
            <ElFormItem label="状态">
              <ElRadioGroup v-model="form.status">
                <ElRadio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">
                  {{ d.dictLabel }}
                </ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="执行策略" prop="misfirePolicy">
              <ElRadioGroup v-model="form.misfirePolicy" size="small">
                <ElRadioButton value="1">立即执行</ElRadioButton>
                <ElRadioButton value="2">执行一次</ElRadioButton>
                <ElRadioButton value="3">放弃执行</ElRadioButton>
              </ElRadioGroup>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="是否并发" prop="concurrent">
              <ElRadioGroup v-model="form.concurrent" size="small">
                <ElRadioButton value="0">允许</ElRadioButton>
                <ElRadioButton value="1">禁止</ElRadioButton>
              </ElRadioGroup>
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="open = false">取 消</ElButton>
      </template>
    </el-dialog>

    <el-dialog v-model="openCron" title="Cron表达式生成器" width="640px" append-to-body>
      <div class="cron-help">
        <p class="cron-help-tip">
          Quartz 格式：秒 分 时 日 月 周 [年]。可选择常用表达式，也可直接编辑后填入。
        </p>
        <div class="cron-preset-list">
          <ElButton
            v-for="item in cronPresets"
            :key="item.value"
            size="small"
            @click="applyCronPreset(item.value)"
          >
            {{ item.label }}
          </ElButton>
        </div>
        <ElInput v-model="cronDraft" placeholder="请输入或选择 cron 表达式" />
      </div>
      <template #footer>
        <ElButton type="primary" @click="confirmCron">确 定</ElButton>
        <ElButton @click="openCron = false">取 消</ElButton>
      </template>
    </el-dialog>

    <JobDetail v-model:visible="openView" :row="viewForm" type="job" />
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';

.link-type {
  color: var(--el-color-primary);
  cursor: pointer;
}

.link-type:hover {
  text-decoration: underline;
}

.label-tip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  margin-left: 2px;
  border-radius: 50%;
  border: 1px solid var(--el-color-info);
  color: var(--el-color-info);
  font-size: 11px;
  cursor: help;
  vertical-align: middle;
}

.cron-input-row {
  display: flex;
  gap: 8px;
  width: 100%;
}

.cron-help-tip {
  margin: 0 0 12px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.cron-preset-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
</style>
