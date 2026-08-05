<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElCol,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElRadio,
  ElRadioGroup,
  ElRow,
  ElSelect,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete } from '@element-plus/icons-vue';

import { addNotice, delNotice, getNotice, listNotice, updateNotice, type SysNotice } from '#/api/system/notice';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemNotice' });

const { dictMap } = useDict({
  type: 'sys_notice_type',
  status: 'sys_notice_status',
});
const { queryParams, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  noticeTitle: '',
  createBy: '',
  noticeType: '',
});

const loading = ref(false);
const list = ref<SysNotice[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const res = await listNotice(queryParams);
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

function handleSelectionChange(selection: SysNotice[]) {
  ids.value = selection.map((item) => item.noticeId);
  single.value = selection.length !== 1;
  multiple.value = !selection.length;
}

// ===== 新增/编辑 =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysNotice>>({});

const rules = {
  noticeTitle: [{ required: true, message: '公告标题不能为空', trigger: 'blur' }],
  noticeType: [{ required: true, message: '公告类型不能为空', trigger: 'change' }],
};

function reset() {
  Object.assign(form, {
    noticeId: undefined,
    noticeTitle: '',
    noticeType: '',
    noticeContent: '',
    status: '0',
  });
  formRef.value?.resetFields();
}

async function handleAdd() {
  reset();
  open.value = true;
  title.value = '添加公告';
}

async function handleUpdate(row?: SysNotice) {
  reset();
  const noticeId = row?.noticeId ?? ids.value[0]!;
  const res = await getNotice(noticeId);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改公告';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.noticeId) {
    await updateNotice(form);
    ElMessage.success('修改成功');
  } else {
    await addNotice(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysNotice) {
  const noticeIds = row.noticeId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除公告编号为"${noticeIds}"的数据项？`, '提示', { type: 'warning' });
    await delNotice(noticeIds as any);
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
      <ElFormItem label="公告标题">
        <ElInput v-model="queryParams.noticeTitle" placeholder="请输入公告标题" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="操作人员">
        <ElInput v-model="queryParams.createBy" placeholder="请输入操作人员" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="类型">
        <ElSelect v-model="queryParams.noticeType" placeholder="公告类型" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.type" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:notice:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:notice:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:notice:remove']" @click="handleDelete({} as SysNotice)">删除</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="序号" align="center" prop="noticeId" width="90" />
      <ElTableColumn label="公告标题" align="center" prop="noticeTitle" show-overflow-tooltip />
      <ElTableColumn label="公告类型" align="center" prop="noticeType" width="100">
        <template #default="{ row }"><DictTag :options="dictMap.type" :value="row.noticeType" /></template>
      </ElTableColumn>
      <ElTableColumn label="状态" align="center" prop="status" width="100">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="创建者" align="center" prop="createBy" width="120" />
      <ElTableColumn label="创建时间" align="center" prop="createTime" width="160">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" align="center" width="160" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:notice:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:notice:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <el-dialog v-model="open" :title="title" width="780px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="80px">
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="公告标题" prop="noticeTitle">
              <ElInput v-model="form.noticeTitle" placeholder="请输入公告标题" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="公告类型" prop="noticeType">
              <ElSelect v-model="form.noticeType" placeholder="请选择公告类型">
                <ElOption v-for="d in dictMap.type" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="24">
            <ElFormItem label="状态">
              <ElRadioGroup v-model="form.status">
                <ElRadio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </ElCol>
          <ElCol :span="24">
            <ElFormItem label="内容">
              <ElInput v-model="form.noticeContent" type="textarea" :rows="6" placeholder="请输入内容" />
            </ElFormItem>
          </ElCol>
        </ElRow>
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
