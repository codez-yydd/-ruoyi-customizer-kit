<template>
  <div class="auth-role">
    <a-card :bordered="false" class="app-page-card">
      <div class="auth-role__header">
        <a-button @click="goBack">
          <template #icon><IconLeft /></template>
          {{ t('common.back') }}
        </a-button>
        <span class="auth-role__title">
          {{ t('system.authRole.title') }}{{ user ? t('system.authRole.userSuffix', { nickName: user.nickName, userName: user.userName }) : '' }}
        </span>
      </div>

      <a-alert type="info" class="auth-role__tip">
        {{ t('system.authRole.tip') }}
      </a-alert>

      <a-table
        :data="roles"
        :loading="loading"
        :pagination="false"
        row-key="roleId"
        :row-selection="rowSelection"
        @selection-change="onSelectionChange"
      >
        <template #columns>
          <a-table-column :title="t('system.role.roleId')" data-index="roleId" :width="100" align="center" />
          <a-table-column :title="t('system.role.roleName')" data-index="roleName" :width="180" />
          <a-table-column :title="t('common.fields.permKey')" data-index="roleKey" :width="160" />
          <a-table-column :title="t('common.fields.displayOrder')" data-index="roleSort" :width="100" align="center" />
          <a-table-column :title="t('common.fields.status')" :width="100" align="center">
            <template #cell="{ record }">
              <DictTag :options="sysNormalDisable" :value="asRole(record).status" />
            </template>
          </a-table-column>
          <a-table-column :title="t('common.fields.createTime')" data-index="createTime" :width="170" />
        </template>
      </a-table>

      <div class="auth-role__footer">
        <a-space>
          <a-button type="primary" :loading="submitting" @click="submit">{{ t('common.submit') }}</a-button>
          <a-button @click="goBack">{{ t('common.cancel') }}</a-button>
        </a-space>
      </div>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import type { TableRowSelection, TableData } from '@arco-design/web-vue'
import { Message } from '@arco-design/web-vue'
import { IconLeft } from '@arco-design/web-vue/es/icon'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import DictTag from '@/components/DictTag/index.vue'
import { getUserAuthRole, setUserAuthRole } from '@/api/system/user'
import type { SysRoleBasic } from '@/api/system/user'
import type { SysUser } from '@/api/types'
import { useDict } from '@/hooks/useDict'

// 隐藏路由页（无 keep-alive 缓存需求）
defineOptions({ name: 'AuthRole' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const dicts = useDict('sys_normal_disable')
const sysNormalDisable = dicts['sys_normal_disable']

const loading = ref(false)
const submitting = ref(false)
const user = ref<SysUser>()
const roles = ref<SysRoleBasic[]>([])
const selectedKeys = ref<Array<string | number>>([])

const rowSelection = computed<TableRowSelection>(() => ({
  type: 'checkbox',
  showCheckedAll: true,
  selectedRowKeys: selectedKeys.value,
  width: 44
}))

function asRole(record: TableData): SysRoleBasic {
  return record as SysRoleBasic
}

function onSelectionChange(keys: Array<string | number>): void {
  selectedKeys.value = keys
}

async function loadDetail(): Promise<void> {
  const userId = Number(route.params.userId)
  if (!Number.isFinite(userId)) {
    Message.warning(t('system.authRole.missingUserId'))
    return
  }
  loading.value = true
  try {
    const res = await getUserAuthRole(userId)
    user.value = res.user
    roles.value = res.roles ?? []
    // 后端 flag 标记已分配角色，默认勾选
    selectedKeys.value = roles.value
      .filter((role) => String(role.flag ?? '') === 'true')
      .map((role) => role.roleId)
  } finally {
    loading.value = false
  }
}

async function submit(): Promise<void> {
  if (!user.value) return
  submitting.value = true
  try {
    // 允许清空全部角色（与若依语义一致：空 roleIds 即取消全部授权）
    await setUserAuthRole(
      user.value.userId,
      selectedKeys.value.map((key) => Number(key))
    )
    Message.success(t('system.authRole.grantSuccess'))
    goBack()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    submitting.value = false
  }
}

function goBack(): void {
  router.push('/system/user')
}

onMounted(() => {
  void loadDetail()
})
</script>

<style scoped>
.auth-role__header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.auth-role__title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
}

.auth-role__tip {
  margin-bottom: 16px;
}

.auth-role__footer {
  display: flex;
  justify-content: center;
  margin-top: 20px;
}
</style>
