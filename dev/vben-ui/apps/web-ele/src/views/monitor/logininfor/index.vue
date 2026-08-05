<script setup lang="ts">
import { onMounted, ref } from 'vue';

import {
  ElButton,
  ElDatePicker,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElPagination,
  ElSelect,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Delete, Refresh, Search, Unlock } from '@element-plus/icons-vue';

import {
  cleanLogininfor,
  delLogininfor,
  listLogininfor,
  unlockLogininfor,
  type SysLogininfor,
} from '#/api/monitor/logininfor';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import DictTag from '#/components/DictTag/index.vue';
import { addDateRange, parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'MonitorLogininfor' });

const { dictMap } = useDict({ status: 'sys_common_status' });

const {
  queryParams,
  dateRange,
  total,
  handleQuery,
  resetQuery: resetQueryBase,
} = usePagination({
  ipaddr: '',
  userName: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysLogininfor[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);
const selectName = ref<string[]>([]);

const defaultSort = { prop: 'loginTime', order: 'descending' } as const;
const orderByColumn = ref('loginTime');
const isAsc = ref('desc');

async function getList() {
  loading.value = true;
  try {
    const params = addDateRange(
      { ...queryParams, orderByColumn: orderByColumn.value, isAsc: isAsc.value },
      dateRange.value,
      'Time',
    );
    const res = await listLogininfor(params);
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
  orderByColumn.value = defaultSort.prop;
  isAsc.value = 'desc';
  getList();
}

function handleSelectionChange(selection: SysLogininfor[]) {
  ids.value = selection.map((item) => item.infoId);
  single.value = selection.length !== 1;
  multiple.value = !selection.length;
  selectName.value = selection.map((item) => item.userName);
}

// 排序
function handleSortChange({ prop, order }: { prop: string; order: string }) {
  orderByColumn.value = prop;
  isAsc.value = order === 'ascending' ? 'asc' : 'desc';
  getList();
}

// 删除
async function handleDelete(row?: SysLogininfor) {
  const infoIds = row?.infoId || ids.value;
  try {
    await ElMessageBox.confirm(
      `是否确认删除访问编号为"${infoIds}"的数据项？`,
      '提示',
      { type: 'warning' },
    );
    await delLogininfor(infoIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

// 清空
async function handleClean() {
  try {
    await ElMessageBox.confirm('是否确认清空所有登录日志数据项？', '提示', {
      type: 'warning',
    });
    await cleanLogininfor();
    getList();
    ElMessage.success('清空成功');
  } catch {
    /* 取消 */
  }
}

// 解锁
async function handleUnlock() {
  const username = selectName.value;
  try {
    await ElMessageBox.confirm(
      `是否确认解锁用户"${username}"数据项？`,
      '提示',
      { type: 'warning' },
    );
    await unlockLogininfor(username as any);
    ElMessage.success(`用户${username}解锁成功`);
  } catch {
    /* 取消 */
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
      <ElFormItem label="状态">
        <ElSelect
          v-model="queryParams.status"
          placeholder="登录状态"
          clearable
          style="width: 200px"
        >
          <ElOption
            v-for="d in dictMap.status"
            :key="d.dictValue"
            :label="d.dictLabel"
            :value="d.dictValue"
          />
        </ElSelect>
      </ElFormItem>
      <ElFormItem label="登录时间">
        <ElDatePicker
          v-model="dateRange"
          style="width: 240px"
          value-format="YYYY-MM-DD HH:mm:ss"
          type="daterange"
          range-separator="-"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          :default-time="[new Date(2000, 0, 1, 0, 0, 0), new Date(2000, 0, 1, 23, 59, 59)]"
        />
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">
          搜索
        </ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        :disabled="multiple"
        v-hasPermi="['monitor:logininfor:remove']"
        @click="handleDelete()"
      >
        删除
      </ElButton>
      <ElButton
        type="danger"
        plain
        :icon="Delete"
        v-hasPermi="['monitor:logininfor:remove']"
        @click="handleClean"
      >
        清空
      </ElButton>
      <ElButton
        type="primary"
        plain
        :icon="Unlock"
        :disabled="single"
        v-hasPermi="['monitor:logininfor:unlock']"
        @click="handleUnlock"
      >
        解锁
      </ElButton>
    </div>

    <ElTable
      v-loading="loading"
      :data="list"
      border
      :default-sort="defaultSort"
      @selection-change="handleSelectionChange"
      @sort-change="handleSortChange"
    >
      <ElTableColumn type="selection" width="55" align="center" />
      <ElTableColumn label="访问编号" align="center" prop="infoId" width="90" />
      <ElTableColumn
        label="用户名称"
        align="center"
        prop="userName"
        show-overflow-tooltip
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      />
      <ElTableColumn
        label="登录地址"
        align="center"
        prop="ipaddr"
        width="130"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="登录地点"
        align="center"
        prop="loginLocation"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="浏览器"
        align="center"
        prop="browser"
        show-overflow-tooltip
      />
      <ElTableColumn label="操作系统" align="center" prop="os" />
      <ElTableColumn label="登录状态" align="center" prop="status">
        <template #default="{ row }">
          <DictTag :options="dictMap.status" :value="row.status" />
        </template>
      </ElTableColumn>
      <ElTableColumn
        label="操作信息"
        align="center"
        prop="msg"
        show-overflow-tooltip
      />
      <ElTableColumn
        label="登录日期"
        align="center"
        prop="loginTime"
        width="180"
        sortable="custom"
        :sort-orders="['descending', 'ascending']"
      >
        <template #default="{ row }">
          {{ parseTime(row.loginTime) }}
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <ElPagination
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
  </div>
</template>

<style scoped>
@import '../../system/_common/page.css';
</style>
