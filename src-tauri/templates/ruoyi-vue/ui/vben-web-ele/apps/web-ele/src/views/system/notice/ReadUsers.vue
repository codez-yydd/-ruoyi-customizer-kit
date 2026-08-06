<script setup lang="ts">
/**
 * 公告已读用户弹窗
 * 对齐若依 ReadUsers：按公告 ID 分页查询已读用户，支持登录名/昵称搜索。
 */
import { reactive, ref } from 'vue';

import { ElButton, ElDialog, ElForm, ElFormItem, ElInput, ElTable, ElTableColumn } from 'element-plus';
import { Search, Refresh } from '@element-plus/icons-vue';

import { listNoticeReadUsers, type NoticeReadUser, type SysNotice } from '#/api/system/notice';
import { parseTime } from '#/utils/ruoyi';

const visible = ref(false);
const loading = ref(false);
const noticeTitle = ref('');
const total = ref(0);
const userList = ref<NoticeReadUser[]>([]);

const queryParams = reactive({
  pageNum: 1,
  pageSize: 10,
  noticeId: undefined as number | undefined,
  searchValue: '',
});

function open(row: SysNotice) {
  noticeTitle.value = row.noticeTitle;
  queryParams.noticeId = row.noticeId;
  queryParams.searchValue = '';
  queryParams.pageNum = 1;
  visible.value = true;
  getList();
}

async function getList() {
  if (!queryParams.noticeId) return;
  loading.value = true;
  try {
    const res = await listNoticeReadUsers(queryParams);
    userList.value = res.rows ?? [];
    total.value = res.total ?? 0;
  } finally {
    loading.value = false;
  }
}

function handleQuery() {
  queryParams.pageNum = 1;
  getList();
}

function resetQuery() {
  queryParams.searchValue = '';
  handleQuery();
}

function handleClose() {
  userList.value = [];
  total.value = 0;
  queryParams.searchValue = '';
}

defineExpose({ open });
</script>

<template>
  <ElDialog
    v-model="visible"
    :title="`「${noticeTitle}」已读用户`"
    width="760px"
    top="6vh"
    append-to-body
    destroy-on-close
    @close="handleClose"
  >
    <ElForm :inline="true" size="small" class="read-users-search">
      <ElFormItem>
        <ElInput
          v-model="queryParams.searchValue"
          placeholder="登录名称 / 用户名称"
          clearable
          style="width: 220px"
          @keyup.enter="handleQuery"
          @clear="handleQuery"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleQuery">搜索</ElButton>
        <ElButton :icon="Refresh" @click="resetQuery">重置</ElButton>
      </ElFormItem>
      <ElFormItem class="read-stat-item">
        <span class="read-stat">
          共 <strong>{{ total }}</strong> 人已读
        </span>
      </ElFormItem>
    </ElForm>

    <ElTable v-loading="loading" :data="userList" size="small" stripe height="340px">
      <ElTableColumn type="index" label="序号" width="55" align="center" />
      <ElTableColumn label="登录名称" prop="userName" align="center" show-overflow-tooltip />
      <ElTableColumn label="用户名称" prop="nickName" align="center" show-overflow-tooltip />
      <ElTableColumn label="所属部门" prop="deptName" align="center" show-overflow-tooltip />
      <ElTableColumn label="手机号码" prop="phonenumber" align="center" width="120" />
      <ElTableColumn label="阅读时间" prop="readTime" align="center" width="160">
        <template #default="{ row }">{{ parseTime(row.readTime) }}</template>
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
  </ElDialog>
</template>

<style scoped>
.read-users-search {
  margin-bottom: 4px;
}

.read-stat-item {
  float: right;
  margin-right: 0;
}

.read-stat {
  font-size: 13px;
  color: #606266;
  line-height: 28px;
}

.read-stat strong {
  color: #409eff;
  font-size: 15px;
  margin: 0 2px;
}

.pagination {
  display: flex;
  justify-content: flex-end;
  padding: 6px 0;
}
</style>
