<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import {
  ElButton,
  ElMessage,
  ElTable,
  ElTableColumn,
} from 'element-plus';

import { authRole, updateAuthRole } from '#/api/system/user';
import { useDict } from '#/composables/useDict';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemUserAuthRole' });

const route = useRoute();
const router = useRouter();

// 字典（角色状态回显）
const { dictMap } = useDict({ status: 'sys_normal_disable' });

const loading = ref(false);
const saveLoading = ref(false);

// 用户信息（顶部卡片展示）
const user = ref<{
  userName?: string;
  nickName?: string;
  dept?: { deptName?: string };
  phonenumber?: string;
  createTime?: string;
}>({});

// 角色列表
const roleList = ref<any[]>([]);
// 当前选中角色 id（表格多选）
const roleIds = ref<number[]>([]);
// 表格引用（用于回显已勾选行）
const tableRef = ref();

// 从路由参数取 userId
const userId = Number(route.params?.userId);

async function getList() {
  if (!userId) {
    ElMessage.error('缺少用户参数');
    return;
  }
  loading.value = true;
  try {
    const res = await authRole(userId);
    user.value = res.user ?? {};
    roleList.value = res.roles ?? [];
    // 后端返回 checked=true 表示该用户已分配此角色，回显到表格勾选
    const checkedRows = roleList.value.filter((r: any) => r.checked);
    // 等待表格渲染完成后再勾选
    setTimeout(() => {
      checkedRows.forEach((row: any) => {
        tableRef.value?.toggleRowSelection(row, true);
      });
      roleIds.value = checkedRows.map((r: any) => r.roleId);
    }, 0);
  } finally {
    loading.value = false;
  }
}

function handleSelectionChange(selection: any[]) {
  roleIds.value = selection.map((item) => item.roleId);
}

async function handleSubmit() {
  saveLoading.value = true;
  try {
    await updateAuthRole(userId, roleIds.value);
    ElMessage.success('分配成功');
    back();
  } finally {
    saveLoading.value = false;
  }
}

function back() {
  router.push('/system/user');
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page auth-role-page">
    <!-- 用户信息卡片 -->
    <div class="user-card">
      <div class="user-card__title">当前用户信息</div>
      <div class="user-card__body">
        <div class="user-card__item">
          <span class="label">用户昵称：</span>
          <span>{{ user.nickName || '-' }}</span>
        </div>
        <div class="user-card__item">
          <span class="label">登录账号：</span>
          <span>{{ user.userName || '-' }}</span>
        </div>
        <div class="user-card__item">
          <span class="label">所属部门：</span>
          <span>{{ user.dept?.deptName || '-' }}</span>
        </div>
        <div class="user-card__item">
          <span class="label">手机号码：</span>
          <span>{{ user.phonenumber || '-' }}</span>
        </div>
        <div class="user-card__item">
          <span class="label">创建时间：</span>
          <span>{{ parseTime(user.createTime) || '-' }}</span>
        </div>
      </div>
    </div>

    <!-- 角色列表 -->
    <div class="role-section">
      <div class="role-section__head">
        <span>角色列表（勾选要分配的角色）</span>
      </div>
      <ElTable
        ref="tableRef"
        v-loading="loading"
        :data="roleList"
        border
        row-key="roleId"
        @selection-change="handleSelectionChange"
      >
        <ElTableColumn type="selection" width="55" align="center" />
        <ElTableColumn label="角色编号" align="center" prop="roleId" width="100" />
        <ElTableColumn label="角色名称" align="center" prop="roleName" show-overflow-tooltip />
        <ElTableColumn label="权限字符" align="center" prop="roleKey" show-overflow-tooltip />
        <ElTableColumn label="显示顺序" align="center" prop="roleSort" width="100" />
        <ElTableColumn label="状态" align="center" width="90">
          <template #default="{ row }">
            {{ dictMap.status?.find((d: any) => d.dictValue === row.status)?.dictLabel || row.status }}
          </template>
        </ElTableColumn>
        <ElTableColumn label="创建时间" align="center" prop="createTime" width="160">
          <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
        </ElTableColumn>
      </ElTable>

      <div class="role-section__footer">
        <ElButton type="primary" :loading="saveLoading" @click="handleSubmit">保 存</ElButton>
        <ElButton @click="back">返 回</ElButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.auth-role-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  height: 100%;
}
.user-card {
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  padding: 16px;
}
.user-card__title {
  font-weight: 600;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.user-card__body {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 8px 24px;
}
.user-card__item .label {
  color: var(--el-text-color-secondary);
}
.role-section {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  padding: 16px;
}
.role-section__head {
  font-weight: 600;
  margin-bottom: 12px;
}
.role-section__footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 16px;
}
</style>
