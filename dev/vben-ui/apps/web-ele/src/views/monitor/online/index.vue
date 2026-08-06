<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElPagination,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Delete, Refresh, Search } from '@element-plus/icons-vue';

import {
  forceLogout,
  listOnline,
  type SysUserOnline,
} from '#/api/monitor/online';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'MonitorOnline' });

const queryParams = ref({
  ipaddr: '',
  userName: '',
});

const loading = ref(false);
const list = ref<SysUserOnline[]>([]);
const pageNum = ref(1);
const pageSize = ref(10);

// 后端 online/list 不做 startPage，返回 Redis 全量；此处前端切片分页
const total = computed(() => list.value.length);
const pagedList = computed(() =>
  list.value.slice(
    (pageNum.value - 1) * pageSize.value,
    pageNum.value * pageSize.value,
  ),
);

/** 强退或筛选后当前页可能越界，回退到有效页 */
function clampPageNum() {
  const maxPage = Math.max(1, Math.ceil(list.value.length / pageSize.value));
  if (pageNum.value > maxPage) {
    pageNum.value = maxPage;
  }
}

async function getList() {
  loading.value = true;
  try {
    const res = await listOnline(queryParams.value);
    list.value = res.rows ?? [];
    clampPageNum();
  } finally {
    loading.value = false;
  }
}

function handleSearch() {
  pageNum.value = 1;
  getList();
}

function handleResetQuery() {
  queryParams.value = { ipaddr: '', userName: '' };
  handleSearch();
}

/** 每页条数变化后校正当前页，避免越界导致表格空白 */
function handleSizeChange() {
  clampPageNum();
}

/** 强制退出指定会话（二次确认后调用强退接口并刷新列表） */
async function handleForceLogout(row: SysUserOnline) {
  try {
    await ElMessageBox.confirm(
      `是否确认强退名称为"${row.userName}"的用户？`,
      '提示',
      { type: 'warning' },
    );
    await forceLogout(row.tokenId);
    await getList();
    ElMessage.success('强退成功');
  } catch (error) {
    // 用户取消确认框时不提示；业务错误由全局拦截器提示
    if (error === 'cancel' || error === 'close') {
      return;
    }
  }
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm
      :inline="true"
      :model="queryParams"
      size="small"
      class="search-form"
    >
      <ElFormItem label="登录地址">
        <ElInput
          v-model="queryParams.ipaddr"
          placeholder="请输入登录地址"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem label="用户名称">
        <ElInput
          v-model="queryParams.userName"
          placeholder="请输入用户名称"
          clearable
          style="width: 200px"
          @keyup.enter="handleSearch"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">
          搜索
        </ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <ElTable v-loading="loading" :data="pagedList" border style="width: 100%">
      <ElTableColumn label="序号" type="index" align="center" width="70">
        <template #default="{ $index }">
          {{ (pageNum - 1) * pageSize + $index + 1 }}
        </template>
      </ElTableColumn>
      <ElTableColumn
        label="会话编号"
        align="center"
        prop="tokenId"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="登录名称"
        align="center"
        prop="userName"
        show-overflow-tooltip
      />
      <ElTableColumn label="部门名称" align="center" prop="deptName" />
      <ElTableColumn
        label="主机"
        align="center"
        prop="ipaddr"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="登录地点"
        align="center"
        prop="loginLocation"
        show-overflow-tooltip
      />
      <ElTableColumn label="浏览器" align="center" prop="browser" />
      <ElTableColumn label="操作系统" align="center" prop="os" />
      <ElTableColumn
        label="登录时间"
        align="center"
        prop="loginTime"
        width="180"
      >
        <template #default="{ row }">
          {{ parseTime(row.loginTime) }}
        </template>
      </ElTableColumn>
      <ElTableColumn label="操作" align="center" width="120" fixed="right">
        <template #default="{ row }">
          <ElButton
            link
            type="danger"
            size="small"
            :icon="Delete"
            v-hasPermi="['monitor:online:forceLogout']"
            @click="handleForceLogout(row)"
          >
            强退
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div v-show="total > 0" class="pagination">
      <ElPagination
        v-model:current-page="pageNum"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 30, 50]"
        layout="total, sizes, prev, pager, next, jumper"
        background
        @size-change="handleSizeChange"
      />
    </div>
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';
</style>
