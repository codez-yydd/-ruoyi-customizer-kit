<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete, RefreshRight } from '@element-plus/icons-vue';

import { addType, delType, getType, listType, refreshDictCache, updateType, type SysDictType } from '#/api/system/dictType';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemDictType' });

const router = useRouter();
const { dictMap } = useDict({ status: 'sys_normal_disable' });
const { queryParams, dateRange, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  dictName: '',
  dictType: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysDictType[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const params = addDateRange({ ...queryParams }, dateRange.value, 'CreateTime');
    const res = await listType(params);
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
function handleSelectionChange(sel: SysDictType[]) {
  ids.value = sel.map((i) => i.dictId);
  single.value = sel.length !== 1;
  multiple.value = !sel.length;
}

/** 跳转到字典数据页（传 dictType） */
function handleDictData(row: SysDictType) {
  router.push({ path: '/system/dict/data', query: { dictType: row.dictType, dictId: String(row.dictId) } });
}

const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysDictType>>({});
const rules = {
  dictName: [{ required: true, message: '字典名称不能为空', trigger: 'blur' }],
  dictType: [{ required: true, message: '字典类型不能为空', trigger: 'blur' }],
};

function reset() {
  Object.assign(form, { dictId: undefined, dictName: '', dictType: '', status: '0', remark: '' });
  formRef.value?.resetFields();
}

function handleAdd() {
  reset();
  open.value = true;
  title.value = '添加字典类型';
}

async function handleUpdate(row?: SysDictType) {
  reset();
  const id = row?.dictId ?? ids.value[0]!;
  const res = await getType(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改字典类型';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.dictId) {
    await updateType(form);
    ElMessage.success('修改成功');
  } else {
    await addType(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysDictType) {
  const dIds = row.dictId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除字典编号为"${dIds}"的数据项？`, '提示', { type: 'warning' });
    await delType(dIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

async function handleRefreshCache() {
  await refreshDictCache();
  ElMessage.success('刷新缓存成功');
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="字典名称">
        <ElInput v-model="queryParams.dictName" placeholder="请输入字典名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="字典类型">
        <ElInput v-model="queryParams.dictType" placeholder="请输入字典类型" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <el-select v-model="queryParams.status" placeholder="字典状态" clearable style="width: 200px">
          <el-option v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </el-select>
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
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:dict:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:dict:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:dict:remove']" @click="handleDelete({} as SysDictType)">删除</ElButton>
      <ElButton type="warning" plain :icon="RefreshRight" v-hasPermi="['system:dict:remove']" @click="handleRefreshCache">刷新缓存</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="字典编号" prop="dictId" width="90" align="center" />
      <ElTableColumn label="字典名称" prop="dictName" show-overflow-tooltip />
      <ElTableColumn label="字典类型" prop="dictType" show-overflow-tooltip>
        <template #default="{ row }">
          <ElButton link type="primary" @click="handleDictData(row)">{{ row.dictType }}</ElButton>
        </template>
      </ElTableColumn>
      <ElTableColumn label="状态" width="80" align="center">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="备注" prop="remark" show-overflow-tooltip />
      <ElTableColumn label="创建时间" prop="createTime" width="160" align="center">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="240" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:dict:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="primary" size="small" v-hasPermi="['system:dict:list']" @click="handleDictData(row)">数据</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:dict:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="600px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="80px">
        <ElFormItem label="字典名称" prop="dictName"><ElInput v-model="form.dictName" placeholder="请输入字典名称" /></ElFormItem>
        <ElFormItem label="字典类型" prop="dictType"><ElInput v-model="form.dictType" placeholder="请输入字典类型" /></ElFormItem>
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
</style>
