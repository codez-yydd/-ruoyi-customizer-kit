<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElTable,
  ElTableColumn,
  ElTreeSelect,
} from 'element-plus';
import { Search, Refresh, Plus } from '@element-plus/icons-vue';

import { addDept, delDept, getDept, listDept, listDeptExcludeChild, updateDept, type SysDept } from '#/api/system/dept';
import { useDict } from '#/composables/useDict';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemDept' });

const { dictMap } = useDict({ status: 'sys_normal_disable' });

const loading = ref(false);
const list = ref<SysDept[]>([]);
const queryParams = reactive({ deptName: '', status: '' });

async function getList() {
  loading.value = true;
  try {
    list.value = await listDept(queryParams);
  } finally {
    loading.value = false;
  }
}

function handleSearch() {
  getList();
}
function handleResetQuery() {
  queryParams.deptName = '';
  queryParams.status = '';
  getList();
}

// ===== 新增/编辑 =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysDept>>({});
const deptOptions = ref<any[]>([]);

const rules = {
  parentId: [{ required: true, message: '上级部门不能为空', trigger: 'blur' }],
  deptName: [{ required: true, message: '部门名称不能为空', trigger: 'blur' }],
  orderNum: [{ required: true, message: '显示排序不能为空', trigger: 'blur' }],
};

function reset() {
  // 注意：ElDialog 默认 lazy，表单在首次打开时才挂载，ElForm 会把那一刻的 form 值
  // 缓存成字段初始值快照。若用户首次操作是「修改某部门」，deptName 等就会被缓存为
  // 该部门值，后续 resetFields() 会把新增表单也重置回该污染值（例如"深圳总公司"）。
  // 因此这里不依赖 resetFields() 清值，改用确定空值整体赋值，仅用 clearValidate 清校验提示。
  Object.assign(form, {
    deptId: undefined,
    parentId: 0,
    deptName: '',
    orderNum: 0,
    leader: '',
    phone: '',
    email: '',
    status: '0',
  });
  nextTick(() => formRef.value?.clearValidate());
}

async function handleAdd(row?: SysDept) {
  reset();
  if (row?.deptId) {
    form.parentId = row.deptId;
  }
  // 新增时加载完整部门树
  deptOptions.value = await listDept({});
  open.value = true;
  title.value = '添加部门';
}

async function handleUpdate(row: SysDept) {
  reset();
  // 编辑时排除自身及子节点，避免循环选择；getDept 经响应拦截器已解包，直接取 res
  const res = await getDept(row.deptId);
  Object.assign(form, res);
  deptOptions.value = await listDeptExcludeChild(row.deptId);
  open.value = true;
  title.value = '修改部门';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.deptId) {
    await updateDept(form);
    ElMessage.success('修改成功');
  } else {
    await addDept(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysDept) {
  try {
    await ElMessageBox.confirm(`是否确认删除部门"${row.deptName}"？`, '提示', { type: 'warning' });
    await delDept(row.deptId);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

/** 树形表格数据转换（若依返回的是平铺带 parentId，需转树） */
const treeData = computedTree();

import { computed } from 'vue';
function computedTree() {
  return computed(() => buildTree(list.value));
}
function buildTree(items: SysDept[], parentId = 0): SysDept[] {
  return items
    .filter((i) => i.parentId === parentId)
    .map((i) => ({ ...i, children: buildTree(items, i.deptId) }))
    .sort((a, b) => (a.orderNum ?? 0) - (b.orderNum ?? 0));
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="部门名称">
        <ElInput v-model="queryParams.deptName" placeholder="请输入部门名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <el-select v-model="queryParams.status" placeholder="部门状态" clearable style="width: 200px">
          <el-option v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </el-select>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:dept:add']" @click="handleAdd()">新增</ElButton>
    </div>

    <ElTable v-loading="loading" :data="treeData" row-key="deptId" border default-expand-all>
      <ElTableColumn label="部门名称" prop="deptName" width="200" />
      <ElTableColumn label="排序" prop="orderNum" width="80" align="center" />
      <ElTableColumn label="负责人" prop="leader" width="100" align="center" />
      <ElTableColumn label="联系电话" prop="phone" width="120" align="center" />
      <ElTableColumn label="邮箱" prop="email" align="center" show-overflow-tooltip />
      <ElTableColumn label="状态" width="80" align="center">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="创建时间" prop="createTime" width="160" align="center">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="220" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:dept:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="primary" size="small" v-hasPermi="['system:dept:add']" @click="handleAdd(row)">新增</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:dept:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <el-dialog v-model="open" :title="title" width="600px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="80px">
        <ElFormItem label="上级部门" prop="parentId">
          <ElTreeSelect
            v-model="form.parentId"
            :data="[{ deptId: 0, deptName: '顶级', children: buildTree(deptOptions) }]"
            :props="{ label: 'deptName', value: 'deptId', children: 'children' }"
            check-strictly
            value-key="deptId"
            placeholder="选择上级部门"
            style="width: 100%"
          />
        </ElFormItem>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="部门名称" prop="deptName">
              <ElInput v-model="form.deptName" placeholder="请输入部门名称" />
            </ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="显示排序" prop="orderNum">
              <el-input-number v-model="form.orderNum" :min="0" controls-position="right" />
            </ElFormItem>
          </el-col>
        </el-row>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="负责人"><ElInput v-model="form.leader" placeholder="请输入负责人" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="联系电话"><ElInput v-model="form.phone" placeholder="请输入联系电话" /></ElFormItem>
          </el-col>
        </el-row>
        <ElFormItem label="邮箱"><ElInput v-model="form.email" placeholder="请输入邮箱" /></ElFormItem>
        <ElFormItem label="部门状态">
          <el-radio-group v-model="form.status">
            <el-radio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</el-radio>
          </el-radio-group>
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
