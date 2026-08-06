<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from 'vue';

import {
  ElButton,
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
import { Search, Refresh, Plus, Edit, Delete, RefreshRight } from '@element-plus/icons-vue';

import { addConfig, delConfig, getConfig, listConfig, refreshConfigCache, updateConfig, type SysConfig } from '#/api/system/config';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemConfig' });

const { dictMap } = useDict({ type: 'sys_yes_no' });
const { queryParams, dateRange, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  configName: '',
  configKey: '',
  configType: '',
});

const loading = ref(false);
const list = ref<SysConfig[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const params = addDateRange({ ...queryParams }, dateRange.value, 'CreateTime');
    const res = await listConfig(params);
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
function handleSelectionChange(sel: SysConfig[]) {
  ids.value = sel.map((i) => i.configId);
  single.value = sel.length !== 1;
  multiple.value = !sel.length;
}

const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysConfig>>({});
const rules = {
  configName: [{ required: true, message: '参数名称不能为空', trigger: 'blur' }],
  configKey: [{ required: true, message: '参数键名不能为空', trigger: 'blur' }],
  configValue: [{ required: true, message: '参数键值不能为空', trigger: 'blur' }],
};

function reset() {
  // 仅清空表单数据。校验态的清除放在弹框打开后的 nextTick（见 handleAdd/handleUpdate），
  // 否则"打开修改 → 取消 → 打开新增"时 resetFields 会以上一次详情数据为基准回滚，
  // 导致新增弹框残留上一个参数的数据（与 role/menu 页面同源问题）。
  Object.assign(form, { configId: undefined, configName: '', configKey: '', configValue: '', configType: 'Y', remark: '' });
}

async function handleAdd() {
  // 先清数据，再开弹框；开框后 nextTick 清校验态（此时 formRef 已就绪且基准干净）。
  reset();
  open.value = true;
  title.value = '添加参数';
  await nextTick();
  formRef.value?.clearValidate();
}

async function handleUpdate(row?: SysConfig) {
  reset();
  const id = row?.configId ?? ids.value[0]!;
  const res = await getConfig(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改参数';
  await nextTick();
  formRef.value?.clearValidate();
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.configId) {
    await updateConfig(form);
    ElMessage.success('修改成功');
  } else {
    await addConfig(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysConfig) {
  const cids = row.configId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除参数编号为"${cids}"的数据项？`, '提示', { type: 'warning' });
    await delConfig(cids as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

async function handleRefreshCache() {
  await refreshConfigCache();
  ElMessage.success('刷新缓存成功');
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="参数名称">
        <ElInput v-model="queryParams.configName" placeholder="请输入参数名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="参数键名">
        <ElInput v-model="queryParams.configKey" placeholder="请输入参数键名" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="系统内置">
        <ElSelect v-model="queryParams.configType" placeholder="系统内置" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.type" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="创建时间">
        <el-date-picker v-model="dateRange" style="width: 240px" value-format="YYYY-MM-DD" type="daterange" range-separator="-" start-placeholder="开始" end-placeholder="结束" />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:config:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:config:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:config:remove']" @click="handleDelete({} as SysConfig)">删除</ElButton>
      <ElButton type="warning" plain :icon="RefreshRight" v-hasPermi="['system:config:remove']" @click="handleRefreshCache">刷新缓存</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="参数主键" align="center" prop="configId" width="90" />
      <ElTableColumn label="参数名称" align="center" prop="configName" show-overflow-tooltip />
      <ElTableColumn label="参数键名" align="center" prop="configKey" show-overflow-tooltip />
      <ElTableColumn label="参数键值" align="center" prop="configValue" show-overflow-tooltip />
      <ElTableColumn label="系统内置" align="center" width="90">
        <template #default="{ row }"><DictTag :options="dictMap.type" :value="row.configType" /></template>
      </ElTableColumn>
      <ElTableColumn label="备注" align="center" prop="remark" show-overflow-tooltip />
      <ElTableColumn label="创建时间" align="center" prop="createTime" width="160">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" align="center" width="160" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:config:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:config:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="600px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="100px">
        <ElFormItem label="参数名称" prop="configName">
          <ElInput v-model="form.configName" placeholder="请输入参数名称" />
        </ElFormItem>
        <ElFormItem label="参数键名" prop="configKey">
          <ElInput v-model="form.configKey" placeholder="请输入参数键名" />
        </ElFormItem>
        <ElFormItem label="参数键值" prop="configValue">
          <ElInput v-model="form.configValue" placeholder="请输入参数键值" />
        </ElFormItem>
        <ElFormItem label="系统内置">
          <ElSelect v-model="form.configType">
            <ElOption v-for="d in dictMap.type" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="备注">
          <ElInput v-model="form.remark" type="textarea" placeholder="请输入内容" />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="open = false">取 消</ElButton>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
@import '../_common/page.css';
</style>
