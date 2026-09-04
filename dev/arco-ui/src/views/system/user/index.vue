<template>
  <div class="user-page">
    <!-- 左侧部门树 -->
    <a-card class="user-page__dept" :bordered="false">
      <a-input-search
        v-model="deptKeyword"
        :placeholder="t('system.user.deptSearchPlaceholder')"
        allow-clear
      />
      <div class="user-page__dept-tree">
        <a-tree
          :data="filteredDeptTree"
          :field-names="treeFieldNames"
          :selected-keys="deptSelectedKeys"
          block-node
          default-expand-all
          @select="onDeptSelect"
        />
      </div>
    </a-card>

    <!-- 右侧主表 -->
    <CrudTable
      class="user-page__main"
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
          <a-form-item field="status" :label="t('common.fields.status')">
            <a-select
              v-model="queryParams.status"
              :options="statusOptions"
              :placeholder="t('system.user.statusPlaceholder')"
              allow-clear
              style="width: 140px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item :label="t('common.fields.createTime')">
            <a-range-picker v-model="dateRange" style="width: 240px" />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="handleReset">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #toolbar>
        <a-button v-hasPermi="['system:user:add']" type="primary" :loading="preparing" @click="handleAddClick">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button
          v-hasPermi="['system:user:edit']"
          :disabled="single"
          :loading="preparing"
          @click="handleUpdateSelection"
        >
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:user:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['system:user:import']" @click="importOpen = true">
          <template #icon><IconUpload /></template>
          {{ t('common.import') }}
        </a-button>
        <a-button v-hasPermi="['system:user:export']" :loading="exportLoading" @click="handleExportClick">
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-deptName="{ record }">{{ asUser(record).dept?.deptName ?? '-' }}</template>

      <template #cell-status="{ record }">
        <a-space :size="8">
          <a-switch
            size="small"
            :model-value="asUser(record).status === '0'"
            :disabled="asUser(record).userId === 1"
            :before-change="() => beforeStatusChange(asUser(record))"
          />
          <DictTag :options="sysNormalDisable" :value="asUser(record).status" />
        </a-space>
      </template>

      <template #cell-operation="{ record }">
        <template v-if="asUser(record).userId !== 1">
          <a-space :size="4">
            <a-link v-hasPermi="['system:user:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
            <a-link v-hasPermi="['system:user:remove']" status="danger" @click="handleDeleteRow(record)">
              {{ t('common.delete') }}
            </a-link>
            <!-- 更多菜单内各项均无权限时不渲染入口，避免空菜单 -->
            <a-dropdown
              v-if="checkPermi(['system:user:resetPwd', 'system:user:edit'])"
              position="tr"
            >
              <a-link>{{ t('common.more') }}<IconDown :size="12" /></a-link>
              <template #content>
                <a-doption v-hasPermi="['system:user:resetPwd']" @click="handleResetPwdRow(record)">
                  <template #icon><IconLock /></template>
                  {{ t('system.user.resetPwd') }}
                </a-doption>
                <a-doption v-hasPermi="['system:user:edit']" @click="handleAuthRoleRow(record)">
                  <template #icon><IconUserAdd /></template>
                  {{ t('system.user.authRole') }}
                </a-doption>
              </template>
            </a-dropdown>
          </a-space>
        </template>
        <span v-else class="user-page__admin-tip">{{ t('system.user.builtInAdmin') }}</span>
      </template>
    </CrudTable>

    <!-- 新增/编辑用户弹窗 -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="700"
      :ok-loading="submitting"
      :mask-closable="false"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-row :gutter="12">
          <a-col :span="12">
            <a-form-item field="userName" :label="t('system.user.userName')">
              <a-input
                v-model.trim="userForm.userName"
                :placeholder="t('common.pleaseEnter', { field: t('system.user.userName') })"
                :disabled="userForm.userId != null"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="nickName" :label="t('system.user.nickName')">
              <a-input
                v-model.trim="userForm.nickName"
                :placeholder="t('common.pleaseEnter', { field: t('system.user.nickName') })"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col v-if="userForm.userId == null" :span="12">
            <a-form-item field="password" :label="t('system.user.password')">
              <a-input-password
                v-model="userForm.password"
                :placeholder="t('system.user.passwordPlaceholder')"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="deptId" :label="t('system.user.deptId')">
              <a-tree-select
                v-model="userForm.deptId"
                :data="deptTree"
                :field-names="treeFieldNames"
                :placeholder="t('system.user.deptIdPlaceholder')"
                allow-clear
                allow-search
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="phonenumber" :label="t('common.fields.phonenumber')">
              <a-input
                v-model.trim="userForm.phonenumber"
                :placeholder="t('common.pleaseEnter', { field: t('common.fields.phonenumber') })"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="email" :label="t('common.fields.email')">
              <a-input
                v-model.trim="userForm.email"
                :placeholder="t('common.pleaseEnter', { field: t('common.fields.email') })"
                allow-clear
              />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="sex" :label="t('system.user.sex')">
              <a-radio-group v-model="userForm.sex">
                <a-radio v-for="item in sysUserSex" :key="item.dictValue" :value="item.dictValue">
                  {{ item.dictLabel }}
                </a-radio>
              </a-radio-group>
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item field="status" :label="t('common.fields.status')">
              <a-radio-group v-model="userForm.status">
                <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
                  {{ item.dictLabel }}
                </a-radio>
              </a-radio-group>
            </a-form-item>
          </a-col>
          <a-col :span="24">
            <a-form-item field="postIds" :label="t('system.user.post')">
              <a-checkbox-group v-model="postIdsValue" direction="vertical">
                <a-checkbox v-for="post in postOptions" :key="post.postId" :value="post.postId">
                  {{ post.postName }}
                </a-checkbox>
              </a-checkbox-group>
            </a-form-item>
          </a-col>
          <a-col :span="24">
            <a-form-item field="roleIds" :label="t('system.user.role')">
              <a-checkbox-group v-model="roleIdsValue" direction="vertical">
                <a-checkbox v-for="role in roleOptions" :key="role.roleId" :value="role.roleId">
                  {{ role.roleName }}
                </a-checkbox>
              </a-checkbox-group>
            </a-form-item>
          </a-col>
          <a-col :span="24">
            <a-form-item field="remark" :label="t('common.fields.remark')">
              <a-textarea
                v-model="userForm.remark"
                :placeholder="t('common.inputContent')"
                :max-length="500"
                show-word-limit
                :auto-size="{ minRows: 2, maxRows: 4 }"
              />
            </a-form-item>
          </a-col>
        </a-row>
      </a-form>
    </a-modal>

    <!-- 重置密码弹窗 -->
    <a-modal
      :visible="resetPwdOpen"
      :title="t('system.user.resetPwd')"
      :width="440"
      :mask-closable="false"
      :ok-loading="resetPwdLoading"
      @ok="submitResetPwd"
      @cancel="resetPwdOpen = false"
      @close="resetPwdOpen = false"
    >
      <a-alert type="warning" class="user-page__reset-tip">
        {{ t('system.user.resetPwdTip', { name: resetPwdForm.userName }) }}
      </a-alert>
      <a-form ref="resetPwdFormRef" :model="resetPwdForm" :rules="resetPwdRules" auto-label-width>
        <a-form-item field="password" :label="t('system.user.newPassword')">
          <a-input-password
            v-model="resetPwdForm.password"
            :placeholder="t('common.newPwdPlaceholder')"
            allow-clear
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 用户导入弹窗 -->
    <a-modal
      :visible="importOpen"
      :title="t('system.user.importTitle')"
      :width="480"
      :mask-closable="false"
      :on-before-ok="handleImportOk"
      @cancel="importOpen = false"
      @close="importOpen = false"
    >
      <a-alert type="info">{{ t('system.user.importTip') }}</a-alert>
      <div class="user-page__import-options">
        <a-checkbox v-model="updateSupport">{{ t('system.user.updateExisting') }}</a-checkbox>
        <a-link @click="handleImportTemplate">
          <template #icon><IconDownload /></template>
          {{ t('system.user.downloadTemplate') }}
        </a-link>
      </div>
      <a-upload
        v-model:file-list="importFileList"
        drag
        :auto-upload="false"
        :limit="1"
        accept=".xls,.xlsx"
        @exceed-limit="Message.warning(t('system.user.onlyOneFile'))"
      >
        <template #tip>{{ t('system.user.dragTip') }}</template>
      </a-upload>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref } from 'vue'
import type { FieldRule, FormInstance, TableData, TreeFieldNames } from '@arco-design/web-vue'
import type { FileItem } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconDelete,
  IconDown,
  IconDownload,
  IconEdit,
  IconLock,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconUpload,
  IconUserAdd
} from '@arco-design/web-vue/es/icon'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import { userDeptTree } from '@/api/system/dept'
import type { DeptTreeNode } from '@/api/system/dept'
import {
  addUser,
  changeUserStatus,
  delUser,
  getUser,
  getUserInit,
  importTemplate,
  importUser,
  listUser,
  resetUserPwd,
  updateUser
} from '@/api/system/user'
import type { SysPost, SysRoleBasic, UserQuery } from '@/api/system/user'
import type { SysUser } from '@/api/types'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'
import { downloadBlob, exportRequest } from '@/utils/download'
import { checkPermi } from '@/utils/permission'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'User' })

/** 弹窗表单类型（用户字段 + 角色/岗位 id 集合） */
type UserForm = Partial<SysUser> & {
  roleIds?: number[]
  postIds?: number[]
  password?: string
}

const router = useRouter()
const { t } = useI18n()
const dicts = useDict('sys_normal_disable', 'sys_user_sex')
const sysNormalDisable = dicts['sys_normal_disable']
const sysUserSex = dicts['sys_user_sex']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/* ---------- 部门树 ---------- */
const treeFieldNames: TreeFieldNames = { key: 'id', title: 'label', children: 'children' }
const deptTree = ref<DeptTreeNode[]>([])
const deptKeyword = ref('')
const deptSelectedKeys = ref<Array<string | number>>([])

function filterDeptTree(nodes: DeptTreeNode[], keyword: string): DeptTreeNode[] {
  const result: DeptTreeNode[] = []
  for (const node of nodes) {
    const matchedChildren = node.children ? filterDeptTree(node.children, keyword) : []
    if (node.label.includes(keyword)) {
      result.push(node)
    } else if (matchedChildren.length > 0) {
      result.push({ ...node, children: matchedChildren })
    }
  }
  return result
}

const filteredDeptTree = computed<DeptTreeNode[]>(() => {
  const keyword = deptKeyword.value.trim()
  if (!keyword) return deptTree.value
  return filterDeptTree(deptTree.value, keyword)
})

function onDeptSelect(keys: Array<string | number>): void {
  deptSelectedKeys.value = keys
  queryParams.deptId = keys.length > 0 ? Number(keys[0]) : undefined
  handleQuery()
}

async function loadDeptTree(): Promise<void> {
  deptTree.value = (await userDeptTree()) ?? []
}

/* ---------- 列表 CRUD ---------- */
/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'userId', label: t('system.user.userId'), width: 90 },
  { key: 'userName', label: t('system.user.userName'), width: 120 },
  { key: 'nickName', label: t('system.user.nickName'), width: 120 },
  { key: 'deptName', label: t('system.user.dept'), width: 140 },
  { key: 'phonenumber', label: t('common.fields.phonenumber'), width: 130 },
  { key: 'status', label: t('common.fields.status'), width: 130 },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 210 }
])

/** 日期范围（提交/导出时转换为 params[beginTime]/params[endTime]） */
const dateRange = ref<[string, string] | undefined>()

/** 组装含日期范围的查询参数 */
function mergeDateRange(query: UserQuery): UserQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  userName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.user.userName') }) },
    { minLength: 2, maxLength: 30, message: t('common.lengthRange', { min: 2, max: 30 }) }
  ],
  nickName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.user.nickName') }) },
    { maxLength: 30, message: t('common.maxLengthTip', { max: 30 }) }
  ],
  password: [
    { required: true, message: t('system.user.passwordPlaceholder') },
    { minLength: 5, maxLength: 20, message: t('common.passwordLengthTip') }
  ],
  phonenumber: [{ match: /^1[3-9]\d{9}$/, message: t('common.phoneInvalid') }],
  email: [{ match: /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/, message: t('common.emailInvalid') }]
}))

const crud = useCrud<UserForm, UserQuery>({
  listApi: (query) => listUser(mergeDateRange(query)),
  addApi: (data) => addUser(data as Parameters<typeof addUser>[0]),
  updateApi: (data) => updateUser(data as Parameters<typeof updateUser>[0]),
  deleteApi: delUser,
  pkField: 'userId'
})

const {
  loading,
  exportLoading,
  list,
  total,
  page,
  limit,
  getList,
  handleQuery,
  resetQuery,
  setSelection,
  single,
  multiple,
  modal,
  formRef,
  handleDelete,
  submitForm,
  cancel
} = crud

const queryParams = crud.queryParams

/* ---------- 行工具 ---------- */
function asUser(record: TableData): SysUser {
  return record as SysUser
}

/** 状态切换：确认后调 changeUserStatus */
function beforeStatusChange(record: SysUser): Promise<boolean> {
  const newStatus = record.status === '0' ? '1' : '0'
  const action = newStatus === '0' ? t('common.enabled') : t('common.disabled')
  return new Promise((resolve) => {
    Modal.confirm({
      title: t('common.statusChangeConfirm'),
      content: t('system.user.statusChangeTip', { name: record.userName, action }),
      hideCancel: false,
      onOk: async () => {
        try {
          await changeUserStatus(record.userId, newStatus)
          record.status = newStatus
          Message.success(newStatus === '0' ? t('common.enableSuccess') : t('common.disableSuccess'))
          resolve(true)
        } catch {
          resolve(false)
        }
      },
      onCancel: () => resolve(false)
    })
  })
}

function handleDeleteRow(record: TableData): void {
  const user = asUser(record)
  handleDelete(user.userId, user.userName)
}

function handleAuthRoleRow(record: TableData): void {
  router.push(`/system/user-auth/role/${asUser(record).userId}`)
}

function handleUpdateSelection(): void {
  const first = crud.selection.value[0]
  if (first) void handleUpdateRow(first)
}

/* ---------- 新增/编辑弹窗 ---------- */
const postOptions = ref<SysPost[]>([])
const roleOptions = ref<SysRoleBasic[]>([])
const submitting = ref(false)
/** 弹窗预加载（拉岗位/角色选项、用户详情）期间的新增/修改按钮 loading */
const preparing = ref(false)

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const userForm = computed(() => modal.form as UserForm)

/** a-checkbox-group 需要确定数组，绑定代理保证初始为 [] */
const postIdsValue = computed<number[]>({
  get: () => userForm.value.postIds ?? [],
  set: (value) => {
    userForm.value.postIds = value
  }
})

const roleIdsValue = computed<number[]>({
  get: () => userForm.value.roleIds ?? [],
  set: (value) => {
    userForm.value.roleIds = value
  }
})

async function openUserModal(title: string, form: UserForm): Promise<void> {
  modal.title = title
  modal.form = form
  modal.open = true
  await nextTick()
  formRef.value?.clearValidate()
}

async function handleAddClick(): Promise<void> {
  preparing.value = true
  try {
    const res = await getUserInit()
    postOptions.value = res.posts ?? []
    roleOptions.value = res.roles ?? []
    await openUserModal(t('system.user.addTitle'), {
      status: '0',
      sex: '0',
      userName: '',
      nickName: '',
      password: '',
      postIds: [],
      roleIds: []
    })
  } finally {
    preparing.value = false
  }
}

async function handleUpdateRow(record: TableData): Promise<void> {
  preparing.value = true
  try {
    const res = await getUser(asUser(record).userId)
    postOptions.value = res.posts ?? []
    roleOptions.value = res.roles ?? []
    await openUserModal(t('system.user.editTitle'), {
      ...res.data,
      postIds: res.postIds ?? [],
      roleIds: res.roleIds ?? []
    })
  } finally {
    preparing.value = false
  }
}

async function onSubmit(): Promise<void> {
  submitting.value = true
  try {
    await submitForm()
  } finally {
    submitting.value = false
  }
}

/* ---------- 重置密码 ---------- */
const resetPwdFormRef = ref<FormInstance>()
const resetPwdOpen = ref(false)
const resetPwdLoading = ref(false)
const resetPwdForm = reactive<{ userId: number; userName: string; password: string }>({
  userId: 0,
  userName: '',
  password: ''
})

const resetPwdRules = computed<Record<string, FieldRule[]>>(() => ({
  password: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.user.newPassword') }) },
    { minLength: 5, maxLength: 20, message: t('common.passwordLengthTip') }
  ]
}))

function handleResetPwdRow(record: TableData): void {
  const user = asUser(record)
  resetPwdForm.userId = user.userId
  resetPwdForm.userName = user.userName
  resetPwdForm.password = ''
  resetPwdOpen.value = true
  void nextTick(() => resetPwdFormRef.value?.clearValidate())
}

async function submitResetPwd(): Promise<void> {
  try {
    await resetPwdFormRef.value?.validate()
  } catch {
    return
  }
  resetPwdLoading.value = true
  try {
    await resetUserPwd(resetPwdForm.userId, resetPwdForm.password)
    Message.success(t('system.user.resetPwdSuccess'))
    resetPwdOpen.value = false
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    resetPwdLoading.value = false
  }
}

/* ---------- 查询/导出 ---------- */
function handleReset(): void {
  dateRange.value = undefined
  deptSelectedKeys.value = []
  queryParams.deptId = undefined
  resetQuery()
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportRequest(
      '/system/user/export',
      mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value }),
      `${t('system.user.exportFileName')}.xlsx`
    )
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/* ---------- 导入 ---------- */
const importOpen = ref(false)
const updateSupport = ref(false)
const importFileList = ref<FileItem[]>([])

async function handleImportTemplate(): Promise<void> {
  try {
    const response = await importTemplate()
    downloadBlob(response, `${t('system.user.templateFileName')}.xlsx`)
  } catch {
    // 失败提示已由拦截器/download.ts 统一弹出
  }
}

async function handleImportOk(): Promise<boolean> {
  const file = importFileList.value[0]?.file
  if (!file) {
    Message.warning(t('system.user.selectImportFile'))
    return false
  }
  try {
    const res = await importUser(file, updateSupport.value)
    Message.success(res.msg || t('system.user.importSuccess'))
    importFileList.value = []
    void getList()
    return true
  } catch {
    return false
  }
}

/* ---------- 初始化 ---------- */
void loadDeptTree()
void getList()
</script>

<style scoped>
.user-page {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}

.user-page__dept {
  width: 240px;
  flex-shrink: 0;
}

.user-page__dept-tree {
  margin-top: 8px;
  max-height: calc(100vh - 260px);
  overflow-y: auto;
}

.user-page__main {
  flex: 1;
  min-width: 0;
}

.user-page__admin-tip {
  font-size: 12px;
  color: var(--color-text-3);
}

.user-page__reset-tip {
  margin-bottom: 16px;
}

.user-page__import-options {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 16px 0;
}
</style>
