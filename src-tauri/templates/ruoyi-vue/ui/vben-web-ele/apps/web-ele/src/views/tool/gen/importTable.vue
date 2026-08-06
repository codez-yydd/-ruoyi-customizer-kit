<script setup lang="ts">
import { reactive, ref } from 'vue';

import { ElButton, ElForm, ElFormItem, ElInput, ElMessage, ElTable, ElTableColumn } from 'element-plus';
import { Refresh, Search } from '@element-plus/icons-vue';

import { importTable, listDbTable, type GenTable } from '#/api/tool/gen';

/**
 * 导入数据库表弹窗
 * 选择库中尚未导入的表，写入 gen_table。
 */
defineOptions({ name: 'ToolGenImportTable' });

const emit = defineEmits<{ ok: [] }>();

const visible = ref(false);
const total = ref(0);
const tables = ref<string[]>([]);
const dbTableList = ref<GenTable[]>([]);
const tableRef = ref();
const queryRef = ref();

const queryParams = reactive({
  pageNum: 1,
  pageSize: 10,
  tableName: '',
  tableComment: '',
});

function show() {
  getList();
  visible.value = true;
}

function clickRow(row: GenTable) {
  tableRef.value?.toggleRowSelection(row);
}

function handleSelectionChange(selection: GenTable[]) {
  tables.value = selection.map((item) => item.tableName);
}

async function getList() {
  const res = await listDbTable({ ...queryParams });
  dbTableList.value = res.rows ?? [];
  total.value = res.total ?? 0;
}

function handleQuery() {
  queryParams.pageNum = 1;
  getList();
}

function resetQuery() {
  queryParams.tableName = '';
  queryParams.tableComment = '';
  queryRef.value?.resetFields?.();
  handleQuery();
}

async function handleImportTable() {
  const tableNames = tables.value.join(',');
  if (!tableNames) {
    ElMessage.error('请选择要导入的表');
    return;
  }
  // 默认生成 Vue3 Element Plus 前端模板，与当前技术栈一致
  const res = (await importTable({
    tables: tableNames,
    tplWebType: 'element-plus',
  })) as { code?: number; msg?: string };
  ElMessage.success(res.msg || '导入成功');
  if (res.code === 200) {
    visible.value = false;
    emit('ok');
  }
}

defineExpose({ show });
</script>

<template>
  <el-dialog v-model="visible" title="导入表" width="800px" top="5vh" append-to-body destroy-on-close>
    <ElForm ref="queryRef" :model="queryParams" :inline="true" size="small">
      <ElFormItem label="表名称" prop="tableName">
        <ElInput
          v-model="queryParams.tableName"
          placeholder="请输入表名称"
          clearable
          style="width: 180px"
          @keyup.enter="handleQuery"
        />
      </ElFormItem>
      <ElFormItem label="表描述" prop="tableComment">
        <ElInput
          v-model="queryParams.tableComment"
          placeholder="请输入表描述"
          clearable
          style="width: 180px"
          @keyup.enter="handleQuery"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleQuery">搜索</ElButton>
        <ElButton :icon="Refresh" @click="resetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>
    <ElTable
      ref="tableRef"
      :data="dbTableList"
      height="260px"
      @row-click="clickRow"
      @selection-change="handleSelectionChange"
    >
      <ElTableColumn type="selection" width="55" />
      <ElTableColumn prop="tableName" label="表名称" show-overflow-tooltip />
      <ElTableColumn prop="tableComment" label="表描述" show-overflow-tooltip />
      <ElTableColumn prop="createTime" label="创建时间" width="160" />
      <ElTableColumn prop="updateTime" label="更新时间" width="160" />
    </ElTable>
    <div class="pagination" style="margin-top: 12px">
      <el-pagination
        v-model:current-page="queryParams.pageNum"
        v-model:page-size="queryParams.pageSize"
        :total="total"
        :page-sizes="[10, 20, 30, 50]"
        layout="total, sizes, prev, pager, next"
        background
        @size-change="getList"
        @current-change="getList"
      />
    </div>
    <template #footer>
      <ElButton type="primary" @click="handleImportTable">确 定</ElButton>
      <ElButton @click="visible = false">取 消</ElButton>
    </template>
  </el-dialog>
</template>
