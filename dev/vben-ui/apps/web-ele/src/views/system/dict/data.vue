<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSelect,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete, Back } from '@element-plus/icons-vue';

import { addData, delData, getData, listData, updateData, type SysDictData } from '#/api/system/dictData';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemDictData' });

const route = useRoute();
const router = useRouter();
const { dictMap } = useDict({ status: 'sys_normal_disable' });
const currentDictType = ref(String(route.query.dictType || ''));

const { queryParams, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  dictType: currentDictType.value,
  dictLabel: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysDictData[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const res = await listData(queryParams);
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
  queryParams.dictType = currentDictType.value;
  getList();
}
function handleSelectionChange(sel: SysDictData[]) {
  ids.value = sel.map((i) => i.dictCode);
  single.value = sel.length !== 1;
  multiple.value = !sel.length;
}

// 标签样式类型选项
const listClassOptions = [
  { value: 'default', label: '默认' },
  { value: 'primary', label: '主要' },
  { value: 'success', label: '成功' },
  { value: 'info', label: '信息' },
  { value: 'warning', label: '警告' },
  { value: 'danger', label: '危险' },
];

const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysDictData>>({});
const rules = {
  dictLabel: [{ required: true, message: '数据标签不能为空', trigger: 'blur' }],
  dictValue: [{ required: true, message: '数据键值不能为空', trigger: 'blur' }],
  dictSort: [{ required: true, message: '显示排序不能为空', trigger: 'blur' }],
};

function reset() {
  Object.assign(form, {
    dictCode: undefined,
    dictSort: 0,
    dictLabel: '',
    dictValue: '',
    dictType: currentDictType.value,
    cssClass: '',
    listClass: 'default',
    isDefault: 'Y',
    status: '0',
    remark: '',
  });
  formRef.value?.resetFields();
}

function handleAdd() {
  reset();
  if (list.value.length > 0) {
    form.dictSort = Math.max(...list.value.map((d) => d.dictSort ?? 0)) + 1;
  }
  open.value = true;
  title.value = '添加字典数据';
}

async function handleUpdate(row?: SysDictData) {
  reset();
  const id = row?.dictCode ?? ids.value[0]!;
  const res = await getData(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改字典数据';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.dictCode) {
    await updateData(form);
    ElMessage.success('修改成功');
  } else {
    await addData(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysDictData) {
  const dCodes = row.dictCode || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除字典编码为"${dCodes}"的数据项？`, '提示', { type: 'warning' });
    await delData(dCodes as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

function goBack() {
  router.push('/system/dict');
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <div class="dict-data-head">
      <ElButton :icon="Back" @click="goBack">返回字典类型</ElButton>
      <span class="current-type">当前字典类型：<strong>{{ currentDictType }}</strong></span>
    </div>

    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="数据标签">
        <ElInput v-model="queryParams.dictLabel" placeholder="请输入数据标签" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect v-model="queryParams.status" placeholder="数据状态" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:dict:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:dict:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:dict:remove']" @click="handleDelete({} as SysDictData)">删除</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="字典编码" prop="dictCode" width="90" align="center" />
      <ElTableColumn label="数据标签" prop="dictLabel" show-overflow-tooltip>
        <template #default="{ row }">
          <el-tag :type="(row.listClass || 'info')" size="small">{{ row.dictLabel }}</el-tag>
        </template>
      </ElTableColumn>
      <ElTableColumn label="数据键值" prop="dictValue" align="center" />
      <ElTableColumn label="显示排序" prop="dictSort" width="90" align="center" />
      <ElTableColumn label="状态" width="80" align="center">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="备注" prop="remark" show-overflow-tooltip />
      <ElTableColumn label="创建时间" prop="createTime" width="160" align="center">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="160" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:dict:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:dict:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="600px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="100px">
        <ElFormItem label="数据标签" prop="dictLabel"><ElInput v-model="form.dictLabel" placeholder="请输入数据标签" /></ElFormItem>
        <ElFormItem label="数据键值" prop="dictValue"><ElInput v-model="form.dictValue" placeholder="请输入数据键值" /></ElFormItem>
        <ElFormItem label="显示排序" prop="dictSort"><ElInputNumber v-model="form.dictSort" :min="0" controls-position="right" /></ElFormItem>
        <ElFormItem label="回显样式">
          <ElSelect v-model="form.listClass">
            <ElOption v-for="o in listClassOptions" :key="o.value" :label="o.label" :value="o.value" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="默认">
          <el-radio-group v-model="form.isDefault">
            <el-radio value="Y">是</el-radio>
            <el-radio value="N">否</el-radio>
          </el-radio-group>
        </ElFormItem>
        <ElFormItem label="状态">
          <el-radio-group v-model="form.status">
            <el-radio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</el-radio>
          </el-radio-group>
        </ElFormItem>
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
@import '../_common/page.css';
.dict-data-head {
  display: flex;
  align-items: center;
  gap: 16px;
}
.current-type {
  font-size: 14px;
  color: var(--el-text-color-secondary);
}
</style>
