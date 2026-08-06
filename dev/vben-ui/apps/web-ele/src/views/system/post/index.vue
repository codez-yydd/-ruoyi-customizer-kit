<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElRadio,
  ElRadioGroup,
  ElSelect,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete } from '@element-plus/icons-vue';

import { addPost, delPost, getPost, listPost, updatePost, type SysPost } from '#/api/system/post';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemPost' });

const { dictMap } = useDict({ status: 'sys_normal_disable' });
const { queryParams, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  postCode: '',
  postName: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysPost[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const res = await listPost(queryParams);
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

function handleSelectionChange(selection: SysPost[]) {
  ids.value = selection.map((item) => item.postId);
  single.value = selection.length !== 1;
  multiple.value = !selection.length;
}

// ===== 新增/编辑 =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysPost>>({});

const rules = {
  postName: [{ required: true, message: '岗位名称不能为空', trigger: 'blur' }],
  postCode: [{ required: true, message: '岗位编码不能为空', trigger: 'blur' }],
  postSort: [{ required: true, message: '岗位顺序不能为空', trigger: 'blur' }],
};

function reset() {
  // 用全新对象整体替换 form，确保清掉上一次（修改）操作残留的字段（如
  // createTime/createBy 等）。仅用 Object.assign 合并会保留 form 上未列出的旧字段，
  // 导致新增弹框回显上一个岗位的数据。
  Object.keys(form).forEach((k) => {
    delete (form as any)[k];
  });
  Object.assign(form, {
    postId: undefined,
    postCode: '',
    postName: '',
    postSort: 0,
    status: '0',
    remark: '',
  });
  // 仅清除校验状态（resetFields 会按 ElForm 缓存的初始值重置，反复开关弹框时不可靠）
  formRef.value?.clearValidate();
}

async function handleAdd() {
  reset();
  if (list.value.length > 0) {
    form.postSort = Math.max(...list.value.map((p) => p.postSort ?? 0)) + 1;
  }
  open.value = true;
  title.value = '添加岗位';
}

async function handleUpdate(row?: SysPost) {
  reset();
  const postId = row?.postId ?? ids.value[0]!;
  // 响应拦截器已自动解包 data 字段，res 即岗位对象本身
  const res = await getPost(postId);
  Object.assign(form, res);
  open.value = true;
  title.value = '修改岗位';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.postId) {
    await updatePost(form);
    ElMessage.success('修改成功');
  } else {
    await addPost(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysPost) {
  const postIds = row.postId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除岗位编号为"${postIds}"的数据项？`, '提示', { type: 'warning' });
    await delPost(postIds as any);
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
      <ElFormItem label="岗位编码">
        <ElInput v-model="queryParams.postCode" placeholder="请输入岗位编码" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="岗位名称">
        <ElInput v-model="queryParams.postName" placeholder="请输入岗位名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect v-model="queryParams.status" placeholder="岗位状态" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:post:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:post:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:post:remove']" @click="handleDelete({} as SysPost)">删除</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="岗位编号" align="center" prop="postId" width="90" />
      <ElTableColumn label="岗位编码" align="center" prop="postCode" />
      <ElTableColumn label="岗位名称" align="center" prop="postName" />
      <ElTableColumn label="岗位排序" align="center" prop="postSort" width="90" />
      <ElTableColumn label="状态" align="center" width="80">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="创建时间" align="center" prop="createTime" width="160">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" align="center" width="160" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:post:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:post:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="600px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="80px">
        <ElFormItem label="岗位名称" prop="postName">
          <ElInput v-model="form.postName" placeholder="请输入岗位名称" />
        </ElFormItem>
        <ElFormItem label="岗位编码" prop="postCode">
          <ElInput v-model="form.postCode" placeholder="请输入岗位编码" />
        </ElFormItem>
        <ElFormItem label="岗位顺序" prop="postSort">
          <ElInputNumber v-model="form.postSort" :min="0" controls-position="right" />
        </ElFormItem>
        <ElFormItem label="岗位状态">
          <ElRadioGroup v-model="form.status">
            <ElRadio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</ElRadio>
          </ElRadioGroup>
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
