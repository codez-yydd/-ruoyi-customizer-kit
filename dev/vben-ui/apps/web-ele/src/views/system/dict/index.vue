<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from 'vue';

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
import { Search, Refresh, Plus, Edit, Delete, RefreshRight } from '@element-plus/icons-vue';

import { addType, delType, getType, listType, refreshDictCache, updateType, type SysDictType } from '#/api/system/dictType';
import { addData, delData, getData, listData, updateData, type SysDictData } from '#/api/system/dictData';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemDictType' });

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

/**
 * 打开「字典数据」弹框：展示该字典类型下的数据列表，并支持在弹框内增删改。
 * 替代原 router.push 跳转到独立 data 页（该页路由未注册会 404）。
 */
function handleDictData(row: SysDictType) {
  currentDictType.value = row.dictType;
  // 重置数据查询条件到该类型，回到第一页
  Object.assign(dataQueryParams, { dictType: row.dictType, dictLabel: '', status: '' });
  dataQueryParams.pageNum = 1;
  dataOpen.value = true;
  getListData();
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
  // 仅重置表单数据为默认值；不调用 formRef.resetFields()，避免其「挂载基准值」
  // 在编辑后被污染，导致后续新增时残留上次编辑数据。校验红字由调用方用 clearValidate 清除。
  Object.assign(form, { dictId: undefined, dictName: '', dictType: '', status: '0', remark: '' });
}

function handleAdd() {
  reset();
  open.value = true;
  title.value = '添加字典类型';
  nextTick(() => formRef.value?.clearValidate());
}

async function handleUpdate(row?: SysDictType) {
  reset();
  const id = row?.dictId ?? ids.value[0]!;
  const res = await getType(id);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改字典类型';
  nextTick(() => formRef.value?.clearValidate());
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

// ==================== 字典数据（弹框）====================
// 弹框替代原独立 data.vue 页：点击字典类型链接/「数据」按钮打开，
// 展示该类型下所有字典数据，并可在弹框内增删改。

const dataOpen = ref(false);
const currentDictType = ref('');
const dataLoading = ref(false);
const dataList = ref<SysDictData[]>([]);
const dataTotal = ref(0);
const dataQueryParams = reactive({
  pageNum: 1,
  pageSize: 10,
  dictType: '',
  dictLabel: '',
  status: '',
});

async function getListData() {
  dataLoading.value = true;
  try {
    const res = await listData(dataQueryParams);
    dataList.value = res.rows ?? [];
    dataTotal.value = res.total ?? 0;
  } finally {
    dataLoading.value = false;
  }
}

function handleDataSearch() {
  dataQueryParams.pageNum = 1;
  getListData();
}

function handleDataResetQuery() {
  Object.assign(dataQueryParams, { dictLabel: '', status: '' });
  dataQueryParams.dictType = currentDictType.value;
  dataQueryParams.pageNum = 1;
  getListData();
}

// 标签回显样式选项
const listClassOptions = [
  { value: 'default', label: '默认' },
  { value: 'primary', label: '主要' },
  { value: 'success', label: '成功' },
  { value: 'info', label: '信息' },
  { value: 'warning', label: '警告' },
  { value: 'danger', label: '危险' },
];

const dataIds = ref<number[]>([]);
const dataSingle = ref(true);
const dataMultiple = ref(true);

function handleDataSelectionChange(sel: SysDictData[]) {
  dataIds.value = sel.map((i) => i.dictCode);
  dataSingle.value = sel.length !== 1;
  dataMultiple.value = !sel.length;
}

// 字典数据表单弹框（嵌套在数据弹框内，append-to-body 避免遮挡）
const dataFormOpen = ref(false);
const dataTitle = ref('');
const dataFormRef = ref();
const dataForm = reactive<Partial<SysDictData>>({});
const dataRules = {
  dictLabel: [{ required: true, message: '数据标签不能为空', trigger: 'blur' }],
  dictValue: [{ required: true, message: '数据键值不能为空', trigger: 'blur' }],
  dictSort: [{ required: true, message: '显示排序不能为空', trigger: 'blur' }],
};

function resetDataForm() {
  // 同类型表单：不调用 resetFields()，避免基准值污染。用 clearValidate 清除校验。
  Object.assign(dataForm, {
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
}

function handleDataAdd() {
  resetDataForm();
  // 新数据默认排序取当前最大值 + 1
  if (dataList.value.length > 0) {
    dataForm.dictSort = Math.max(...dataList.value.map((d) => d.dictSort ?? 0)) + 1;
  }
  dataFormOpen.value = true;
  dataTitle.value = '添加字典数据';
  nextTick(() => dataFormRef.value?.clearValidate());
}

async function handleDataUpdate(row?: SysDictData) {
  resetDataForm();
  const id = row?.dictCode ?? dataIds.value[0]!;
  const res = await getData(id);
  Object.assign(dataForm, res.data);
  dataFormOpen.value = true;
  dataTitle.value = '修改字典数据';
  nextTick(() => dataFormRef.value?.clearValidate());
}

async function submitDataForm() {
  await dataFormRef.value?.validate();
  if (dataForm.dictCode) {
    await updateData(dataForm);
    ElMessage.success('修改成功');
  } else {
    await addData(dataForm);
    ElMessage.success('新增成功');
  }
  dataFormOpen.value = false;
  getListData();
}

async function handleDataDelete(row: SysDictData) {
  const dCodes = row.dictCode || dataIds.value;
  try {
    await ElMessageBox.confirm(`是否确认删除字典编码为"${dCodes}"的数据项？`, '提示', { type: 'warning' });
    await delData(dCodes as any);
    getListData();
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

    <!-- 字典数据弹框：点击字典类型链接 / 数据按钮打开，替代原独立 data 页 -->
    <el-dialog v-model="dataOpen" :title="`字典数据 - ${currentDictType}`" width="900px" append-to-body>
      <ElForm :inline="true" :model="dataQueryParams" size="small" class="search-form">
        <ElFormItem label="数据标签">
          <ElInput v-model="dataQueryParams.dictLabel" placeholder="请输入数据标签" clearable style="width: 200px" @keyup.enter="handleDataSearch" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="dataQueryParams.status" placeholder="数据状态" clearable style="width: 200px">
            <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem>
          <ElButton type="primary" :icon="Search" @click="handleDataSearch">搜索</ElButton>
          <ElButton :icon="Refresh" @click="handleDataResetQuery">重置</ElButton>
        </ElFormItem>
      </ElForm>

      <div class="toolbar">
        <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:dict:add']" @click="handleDataAdd">新增</ElButton>
        <ElButton type="success" plain :icon="Edit" :disabled="dataSingle" v-hasPermi="['system:dict:edit']" @click="handleDataUpdate()">修改</ElButton>
        <ElButton type="danger" plain :icon="Delete" :disabled="dataMultiple" v-hasPermi="['system:dict:remove']" @click="handleDataDelete({} as SysDictData)">删除</ElButton>
      </div>

      <ElTable v-loading="dataLoading" :data="dataList" border @selection-change="handleDataSelectionChange">
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
            <ElButton link type="primary" size="small" v-hasPermi="['system:dict:edit']" @click="handleDataUpdate(row)">修改</ElButton>
            <ElButton link type="danger" size="small" v-hasPermi="['system:dict:remove']" @click="handleDataDelete(row)">删除</ElButton>
          </template>
        </ElTableColumn>
      </ElTable>

      <div class="pagination">
        <el-pagination v-model:current-page="dataQueryParams.pageNum" v-model:page-size="dataQueryParams.pageSize" :total="dataTotal" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getListData" @current-change="getListData" />
      </div>

      <!-- 嵌套：字典数据表单弹框（append-to-body 避免被外层弹框遮挡） -->
      <el-dialog v-model="dataFormOpen" :title="dataTitle" width="600px" append-to-body>
        <ElForm ref="dataFormRef" :model="dataForm" :rules="dataRules" label-width="100px">
          <ElFormItem label="数据标签" prop="dictLabel"><ElInput v-model="dataForm.dictLabel" placeholder="请输入数据标签" /></ElFormItem>
          <ElFormItem label="数据键值" prop="dictValue"><ElInput v-model="dataForm.dictValue" placeholder="请输入数据键值" /></ElFormItem>
          <ElFormItem label="显示排序" prop="dictSort"><ElInputNumber v-model="dataForm.dictSort" :min="0" controls-position="right" /></ElFormItem>
          <ElFormItem label="回显样式">
            <ElSelect v-model="dataForm.listClass">
              <ElOption v-for="o in listClassOptions" :key="o.value" :label="o.label" :value="o.value" />
            </ElSelect>
          </ElFormItem>
          <ElFormItem label="默认">
            <el-radio-group v-model="dataForm.isDefault">
              <el-radio value="Y">是</el-radio>
              <el-radio value="N">否</el-radio>
            </el-radio-group>
          </ElFormItem>
          <ElFormItem label="状态">
            <el-radio-group v-model="dataForm.status">
              <el-radio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</el-radio>
            </el-radio-group>
          </ElFormItem>
          <ElFormItem label="备注"><ElInput v-model="dataForm.remark" type="textarea" placeholder="请输入内容" /></ElFormItem>
        </ElForm>
        <template #footer>
          <ElButton type="primary" @click="submitDataForm">确 定</ElButton>
          <ElButton @click="dataFormOpen = false">取 消</ElButton>
        </template>
      </el-dialog>
    </el-dialog>
  </div>
</template>

<style scoped>
@import '../_common/page.css';
</style>
