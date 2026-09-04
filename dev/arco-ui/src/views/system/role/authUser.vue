<template>
  <div class="auth-user">
    <!-- 角色信息头 -->
    <a-card :bordered="false" class="app-page-card auth-user__header">
      <div class="auth-user__header-main">
        <a-button @click="goBack">
          <template #icon><IconLeft /></template>
          {{ t('common.back') }}
        </a-button>
        <div class="auth-user__role-info">
          <span class="auth-user__title">{{ t('system.authUser.title') }}</span>
          <a-tag v-if="role" color="arcoblue" size="small">{{ role.roleName }}（{{ role.roleKey }}）</a-tag>
        </div>
      </div>
    </a-card>

    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="userId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="userName" :label="t('system.user.userName')">
            <a-input
              v-model.trim="queryParams.userName"
              :placeholder="t('common.pleaseEnter', { field: t('system.user.userName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="phonenumber" :label="t('common.fields.phonenumber')">
            <a-input
              v-model.trim="queryParams.phonenumber"
              :placeholder="t('common.pleaseEnter', { field: t('common.fields.phonenumber') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="resetQuery">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #toolbar>
        <a-button
          v-hasPermi="['system:role:edit']"
          type="primary"
          :disabled="multiple"
          @click="handleCancelSelected"
        >
          <template #icon><IconCloseCircle /></template>
          {{ t('system.authUser.batchCancelAuth') }}
        </a-button>
        <a-button v-hasPermi="['system:role:edit']" @click="openAddModal">
          <template #icon><IconPlus /></template>
          {{ t('system.authUser.addUser') }}
        </a-button>
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysNormalDisable" :value="asUser(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-link v-hasPermi="['system:role:edit']" status="danger" @click="handleCancelOne(asUser(record))">
          {{ t('system.authUser.cancelAuth') }}
        </a-link>
      </template>
    </CrudTable>

    <!-- 新增授权用户弹窗 -->
    <a-modal
      :visible="addOpen"
      :title="t('system.authUser.addModalTitle')"
      :width="760"
      :mask-closable="false"
      :ok-loading="addSubmitting"
      @ok="submitAdd"
      @cancel="addOpen = false"
      @close="addOpen = false"
    >
      <a-form :model="addQuery" layout="inline" class="auth-user__add-search">
        <a-form-item field="userName" :label="t('system.user.userName')">
          <a-input
            v-model.trim="addQuery.userName"
            :placeholder="t('common.pleaseEnter', { field: t('system.user.userName') })"
            allow-clear
            style="width: 150px"
            @keyup.enter="loadUnallocated(1)"
          />
        </a-form-item>
        <a-form-item field="phonenumber" :label="t('common.fields.phonenumber')">
          <a-input
            v-model.trim="addQuery.phonenumber"
            :placeholder="t('common.pleaseEnter', { field: t('common.fields.phonenumber') })"
            allow-clear
            style="width: 150px"
            @keyup.enter="loadUnallocated(1)"
          />
        </a-form-item>
        <a-form-item>
          <a-button type="primary" @click="loadUnallocated(1)">
            <template #icon><IconSearch /></template>
            {{ t('common.search') }}
          </a-button>
        </a-form-item>
      </a-form>

      <a-table
        :data="addList"
        :loading="addLoading"
        :pagination="false"
        row-key="userId"
        :row-selection="addRowSelection"
        @selection-change="onAddSelectionChange"
      >
        <template #columns>
          <a-table-column :title="t('system.user.userName')" data-index="userName" :width="120" />
          <a-table-column :title="t('system.user.nickName')" data-index="nickName" :width="120" />
          <a-table-column :title="t('common.fields.email')" data-index="email" ellipsis :min-width="160" />
          <a-table-column :title="t('common.fields.phonenumber')" data-index="phonenumber" :width="130" />
          <a-table-column :title="t('common.fields.status')" :width="90" align="center">
            <template #cell="{ record }">
              <DictTag :options="sysNormalDisable" :value="asUser(record).status" />
            </template>
          </a-table-column>
        </template>
      </a-table>

      <div class="auth-user__add-footer">
        <Pagination
          :page="addPage"
          :limit="addLimit"
          :total="addTotal"
          @update:page="onAddPageChange"
          @change="loadUnallocated()"
        />
      </div>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import type { TableRowSelection, TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconCloseCircle,
  IconLeft,
  IconPlus,
  IconRefresh,
  IconSearch
} from '@arco-design/web-vue/es/icon'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import Pagination from '@/components/Pagination/index.vue'
import {
  allocatedList,
  cancelAuthUser,
  cancelAuthUserAll,
  getRole,
  selectAuthUserAll,
  unallocatedList
} from '@/api/system/role'
import type { AuthUserQuery, SysRole } from '@/api/system/role'
import type { SysUser } from '@/api/types'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 隐藏路由页（无 keep-alive 缓存需求）
defineOptions({ name: 'AuthUser' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const dicts = useDict('sys_normal_disable')
const sysNormalDisable = dicts['sys_normal_disable']

const role = ref<SysRole>()

function currentRoleId(): number {
  return Number(route.params.roleId)
}

function asUser(record: TableData): SysUser {
  return record as SysUser
}

/* ---------- 已授权用户列表 ---------- */
/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'userName', label: t('system.user.userName'), width: 140 },
  { key: 'nickName', label: t('system.user.nickName'), width: 140 },
  { key: 'email', label: t('common.fields.email'), minWidth: 180 },
  { key: 'phonenumber', label: t('common.fields.phonenumber'), width: 140 },
  { key: 'status', label: t('common.fields.status'), width: 110 },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 110 }
])

const crud = useCrud<SysUser, AuthUserQuery>({
  listApi: (query) => allocatedList({ ...query, roleId: currentRoleId() }),
  pkField: 'userId'
})

const { loading, list, total, page, limit, getList, handleQuery, resetQuery, setSelection, multiple } = crud

const queryParams = crud.queryParams

function goBack(): void {
  router.push('/system/role')
}

/** 单个取消授权 */
function handleCancelOne(user: SysUser): void {
  Modal.confirm({
    title: t('system.authUser.cancelConfirmTitle'),
    content: t('system.authUser.cancelOneConfirm', { name: user.userName }),
    hideCancel: false,
    onOk: async () => {
      await cancelAuthUser(user.userId, currentRoleId())
      Message.success(t('system.authUser.cancelSuccess'))
      await getList()
    }
  })
}

/** 批量取消授权 */
function handleCancelSelected(): void {
  const userIds = crud.ids.value
  if (userIds.length === 0) {
    Message.warning(t('system.authUser.selectCancelUsers'))
    return
  }
  Modal.confirm({
    title: t('system.authUser.cancelConfirmTitle'),
    content: t('system.authUser.cancelSelectedConfirm', { count: userIds.length }),
    hideCancel: false,
    onOk: async () => {
      await cancelAuthUserAll(currentRoleId(), userIds)
      Message.success(t('system.authUser.cancelSuccess'))
      await getList()
    }
  })
}

/* ---------- 新增授权用户弹窗 ---------- */
const addOpen = ref(false)
const addLoading = ref(false)
const addSubmitting = ref(false)
const addQuery = reactive<{ userName?: string; phonenumber?: string }>({})
const addList = ref<SysUser[]>([])
const addTotal = ref(0)
const addPage = ref(1)
const addLimit = ref(10)
const addSelectedKeys = ref<Array<string | number>>([])

const addRowSelection = computed<TableRowSelection>(() => ({
  type: 'checkbox',
  showCheckedAll: true,
  selectedRowKeys: addSelectedKeys.value,
  width: 44
}))

function onAddSelectionChange(keys: Array<string | number>): void {
  addSelectedKeys.value = keys
}

function onAddPageChange(value: number): void {
  addPage.value = value
}

async function loadUnallocated(pageOverride?: number): Promise<void> {
  if (pageOverride != null) addPage.value = pageOverride
  addLoading.value = true
  try {
    const res = await unallocatedList({
      roleId: currentRoleId(),
      userName: addQuery.userName,
      phonenumber: addQuery.phonenumber,
      pageNum: addPage.value,
      pageSize: addLimit.value
    })
    addList.value = res.rows ?? []
    addTotal.value = res.total ?? 0
  } finally {
    addLoading.value = false
  }
}

function openAddModal(): void {
  addQuery.userName = ''
  addQuery.phonenumber = ''
  addSelectedKeys.value = []
  addPage.value = 1
  addOpen.value = true
  void loadUnallocated()
}

async function submitAdd(): Promise<void> {
  if (addSelectedKeys.value.length === 0) {
    Message.warning(t('system.authUser.selectAuthUsers'))
    return
  }
  addSubmitting.value = true
  try {
    await selectAuthUserAll(currentRoleId(), addSelectedKeys.value)
    Message.success(t('system.authUser.grantSuccess'))
    addOpen.value = false
    await getList()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    addSubmitting.value = false
  }
}

/* ---------- 初始化 ---------- */
onMounted(async () => {
  if (!Number.isFinite(currentRoleId())) {
    Message.warning(t('system.authUser.missingRoleId'))
    void goBack()
    return
  }
  try {
    role.value = await getRole(currentRoleId())
  } catch {
    // 角色信息加载失败不阻塞列表查询
  }
  void getList()
})
</script>

<style scoped>
.auth-user {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.auth-user__header-main {
  display: flex;
  align-items: center;
  gap: 16px;
}

.auth-user__role-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.auth-user__title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-1);
}

.auth-user__add-search {
  margin-bottom: 16px;
}

.auth-user__add-footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}
</style>
