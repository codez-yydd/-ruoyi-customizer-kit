<template>
  <div class="role-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="roleId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="roleName" :label="t('system.role.roleName')">
            <a-input
              v-model.trim="queryParams.roleName"
              :placeholder="t('common.pleaseEnter', { field: t('system.role.roleName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="roleKey" :label="t('common.fields.permKey')">
            <a-input
              v-model.trim="queryParams.roleKey"
              :placeholder="t('common.pleaseEnter', { field: t('common.fields.permKey') })"
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
              :placeholder="t('system.role.statusPlaceholder')"
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
        <a-button v-hasPermi="['system:role:add']" type="primary" :loading="preparing" @click="handleAddClick">
          <template #icon><IconPlus /></template>
          {{ t('common.add') }}
        </a-button>
        <a-button
          v-hasPermi="['system:role:edit']"
          :disabled="single"
          :loading="preparing"
          @click="handleUpdateSelection"
        >
          <template #icon><IconEdit /></template>
          {{ t('common.edit') }}
        </a-button>
        <a-button v-hasPermi="['system:role:remove']" :disabled="multiple" @click="handleDelete()">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['system:role:export']" :loading="exportLoading" @click="handleExportClick">
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-status="{ record }">
        <a-space :size="8">
          <a-switch
            size="small"
            :model-value="asRole(record).status === '0'"
            :disabled="asRole(record).roleId === 1"
            :before-change="() => beforeStatusChange(asRole(record))"
          />
          <DictTag :options="sysNormalDisable" :value="asRole(record).status" />
        </a-space>
      </template>

      <template #cell-operation="{ record }">
        <template v-if="asRole(record).roleId !== 1">
          <a-space :size="4">
            <a-link v-hasPermi="['system:role:edit']" @click="handleUpdateRow(record)">{{ t('common.edit') }}</a-link>
            <!-- 更多菜单内各项均无权限时不渲染入口，避免空菜单 -->
            <a-dropdown
              v-if="checkPermi(['system:role:edit', 'system:role:remove'])"
              position="tr"
            >
              <a-link>{{ t('common.more') }}<IconDown :size="12" /></a-link>
              <template #content>
                <a-doption v-hasPermi="['system:role:edit']" @click="handleDataScopeRow(record)">
                  <template #icon><IconCommon /></template>
                  {{ t('system.role.dataPermission') }}
                </a-doption>
                <a-doption v-hasPermi="['system:role:edit']" @click="handleAuthUserRow(record)">
                  <template #icon><IconUser /></template>
                  {{ t('system.role.assignUser') }}
                </a-doption>
                <a-doption v-hasPermi="['system:role:remove']" @click="handleDeleteRow(record)">
                  <template #icon><IconDelete /></template>
                  {{ t('common.delete') }}
                </a-doption>
              </template>
            </a-dropdown>
          </a-space>
        </template>
        <span v-else class="role-page__admin-tip">{{ t('system.role.builtInRole') }}</span>
      </template>
    </CrudTable>

    <!-- 新增/编辑角色弹窗 -->
    <a-modal
      :visible="modal.open"
      :title="modal.title"
      :width="640"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="cancel"
      @close="cancel"
    >
      <a-form ref="formRef" :model="modal.form" :rules="formRules" auto-label-width>
        <a-form-item field="roleName" :label="t('system.role.roleName')">
          <a-input
            v-model.trim="roleForm.roleName"
            :placeholder="t('common.pleaseEnter', { field: t('system.role.roleName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="roleKey" :label="t('common.fields.permKey')">
          <a-input
            v-model.trim="roleForm.roleKey"
            :placeholder="t('common.pleaseEnter', { field: t('common.fields.permKey') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="roleSort" :label="t('common.fields.displayOrder')">
          <a-input-number v-model="roleForm.roleSort" :min="0" :placeholder="t('common.pleaseEnter', { field: t('common.fields.displayOrder') })" />
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-radio-group v-model="roleForm.status">
            <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item :label="t('system.role.menuPermission')">
          <a-checkbox v-model="roleForm.menuCheckStrictly">{{ t('system.role.parentChildLink') }}</a-checkbox>
          <div class="role-page__menu-tree">
            <a-tree
              :data="menuTree"
              :field-names="treeFieldNames"
              checkable
              :check-strictly="!roleForm.menuCheckStrictly"
              :checked-keys="menuCheckedKeys"
              :default-expand-all="true"
              @check="onMenuCheck"
            />
          </div>
        </a-form-item>
        <a-form-item field="remark" :label="t('common.fields.remark')">
          <a-textarea
            v-model="roleForm.remark"
            :placeholder="t('common.inputContent')"
            :max-length="500"
            show-word-limit
            :auto-size="{ minRows: 2, maxRows: 4 }"
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 数据权限弹窗 -->
    <a-modal
      :visible="dataScopeOpen"
      :title="t('system.role.dataScopeTitle')"
      :width="520"
      :mask-closable="false"
      :ok-loading="dataScopeSaving"
      @ok="submitDataScope"
      @cancel="dataScopeOpen = false"
      @close="dataScopeOpen = false"
    >
      <a-form :model="dataScopeForm" auto-label-width>
        <a-form-item :label="t('system.role.roleName')">
          <a-input :model-value="dataScopeForm.roleName" disabled />
        </a-form-item>
        <a-form-item :label="t('common.fields.permKey')">
          <a-input :model-value="dataScopeForm.roleKey" disabled />
        </a-form-item>
        <a-form-item :label="t('system.role.dataScope')">
          <a-radio-group v-model="dataScopeForm.dataScope" direction="vertical">
            <a-radio v-for="item in sysDataScope" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item v-if="dataScopeForm.dataScope === '2'" :label="t('system.role.dataPermission')">
          <a-checkbox v-model="dataScopeForm.deptCheckStrictly">{{ t('system.role.parentChildLink') }}</a-checkbox>
          <div class="role-page__menu-tree">
            <a-tree
              :data="deptTree"
              :field-names="treeFieldNames"
              checkable
              :check-strictly="!dataScopeForm.deptCheckStrictly"
              :checked-keys="deptCheckedKeys"
              :default-expand-all="true"
              @check="onDeptCheck"
            />
          </div>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref } from 'vue'
import type { FieldRule, TableData, TreeFieldNames } from '@arco-design/web-vue'
import type { TreeNodeKey } from '@arco-design/web-vue/es/tree/interface'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconCommon,
  IconDelete,
  IconDown,
  IconDownload,
  IconEdit,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconUser
} from '@arco-design/web-vue/es/icon'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  addRole,
  changeRoleStatus,
  delRole,
  getRole,
  listRole,
  menuTreeselect,
  roleDeptTree,
  roleMenuTreeselect,
  updateDataScope,
  updateRole
} from '@/api/system/role'
import type { MenuTreeNode, RoleQuery, SysRole } from '@/api/system/role'
import type { DeptTreeNode } from '@/api/system/dept'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'
import type { DictDataOption } from '@/api/system/dict'
import { exportRequest } from '@/utils/download'
import { checkPermi } from '@/utils/permission'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Role' })

/** 弹窗表单类型 */
type RoleForm = Partial<SysRole>

/**
 * 数据范围字典兜底项（label 随语言切换联动）：部分后端库未初始化 sys_data_scope 字典
 * （接口返回空数组），为保证数据权限功能可用，字典为空时回退到若依标准五项。
 */
const dataScopeFallback = computed<DictDataOption[]>(() => [
  { dictLabel: t('system.role.dataScopeOptions.all'), dictValue: '1', dictType: 'sys_data_scope', cssClass: null, listClass: 'default', dictSort: 1 },
  { dictLabel: t('system.role.dataScopeOptions.custom'), dictValue: '2', dictType: 'sys_data_scope', cssClass: null, listClass: 'default', dictSort: 2 },
  { dictLabel: t('system.role.dataScopeOptions.dept'), dictValue: '3', dictType: 'sys_data_scope', cssClass: null, listClass: 'default', dictSort: 3 },
  { dictLabel: t('system.role.dataScopeOptions.deptAndChild'), dictValue: '4', dictType: 'sys_data_scope', cssClass: null, listClass: 'default', dictSort: 4 },
  { dictLabel: t('system.role.dataScopeOptions.self'), dictValue: '5', dictType: 'sys_data_scope', cssClass: null, listClass: 'default', dictSort: 5 }
])

const router = useRouter()
const { t } = useI18n()
const dicts = useDict('sys_normal_disable', 'sys_data_scope')
const sysNormalDisable = dicts['sys_normal_disable']
const sysDataScope = computed<DictDataOption[]>(() => {
  const fromDict = dicts['sys_data_scope'].value
  return fromDict.length > 0 ? fromDict : dataScopeFallback.value
})

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

const treeFieldNames: TreeFieldNames = { key: 'id', title: 'label', children: 'children' }

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: RoleQuery): RoleQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'roleId', label: t('system.role.roleId'), width: 90 },
  { key: 'roleName', label: t('system.role.roleName'), width: 150 },
  { key: 'roleKey', label: t('common.fields.permKey'), width: 150 },
  { key: 'roleSort', label: t('common.fields.displayOrder'), width: 90, align: 'center' },
  { key: 'status', label: t('common.fields.status'), width: 130 },
  { key: 'createTime', label: t('common.fields.createTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 220 }
])

/** 弹窗表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  roleName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.role.roleName') }) },
    { maxLength: 30, message: t('common.maxLengthTip', { max: 30 }) }
  ],
  roleKey: [
    { required: true, message: t('common.pleaseEnter', { field: t('common.fields.permKey') }) },
    { maxLength: 100, message: t('common.maxLengthTip', { max: 100 }) }
  ],
  roleSort: [{ required: true, message: t('common.pleaseEnter', { field: t('common.fields.displayOrder') }) }]
}))

const crud = useCrud<SysRole, RoleQuery>({
  listApi: (query) => listRole(mergeDateRange(query)),
  deleteApi: delRole,
  pkField: 'roleId'
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
  cancel
} = crud

const queryParams = crud.queryParams

function asRole(record: TableData): SysRole {
  return record as SysRole
}

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportRequest(
      '/system/role/export',
      mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value }),
      `${t('system.role.exportFileName')}.xlsx`
    )
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/* ---------- 菜单树（提交 menuIds = 全选 + 半选，保证后端权限完整） ---------- */
const menuTree = ref<MenuTreeNode[]>([])
const menuCheckedKeys = ref<TreeNodeKey[]>([])
const menuHalfCheckedKeys = ref<TreeNodeKey[]>([])

/**
 * Arco a-tree 的 @check 事件第二参数携带 halfCheckedKeys（半选节点）；
 * 受控维护两个集合，提交时合并为完整 menuIds。
 */
function onMenuCheck(
  keys: TreeNodeKey[],
  ev: { halfCheckedKeys?: TreeNodeKey[] }
): void {
  menuCheckedKeys.value = keys
  menuHalfCheckedKeys.value = ev.halfCheckedKeys ?? []
}

/** 收集完整菜单权限（全选 + 半选父节点） */
function collectMenuIds(): number[] {
  return [...menuCheckedKeys.value, ...menuHalfCheckedKeys.value].map((key) => Number(key))
}

/**
 * 回显时补算半选祖先集合：
 * 后端 checkedKeys 只含"叶端勾选项"（有勾选子节点的父被 SQL 排除），
 * Arco 受控树不会像 Element el-tree 那样把联动出的父级写回 checkedKeys，
 * 因此需要根据树结构找出"有勾选后代"的祖先节点补入半选集合，
 * 保证"打开弹窗未点击树直接保存"时提交的 menuIds/deptIds 仍包含全部父级链路。
 */
function collectAncestorKeys(
  nodes: Array<MenuTreeNode | DeptTreeNode>,
  leafKeys: Set<number>,
  ancestors: Set<number>
): boolean {
  let hasChecked = false
  for (const node of nodes) {
    const childHasChecked = node.children
      ? collectAncestorKeys(node.children, leafKeys, ancestors)
      : false
    if (childHasChecked) {
      ancestors.add(node.id)
    }
    if (childHasChecked || leafKeys.has(node.id)) {
      hasChecked = true
    }
  }
  return hasChecked
}

/** 按后端 checkedKeys 回显：checked 透传，半选集合由树结构补算 */
function echoCheckedKeys(
  tree: Array<MenuTreeNode | DeptTreeNode>,
  checkedKeys: number[]
): { checked: TreeNodeKey[]; half: TreeNodeKey[] } {
  const leaf = new Set(checkedKeys)
  const ancestors = new Set<number>()
  collectAncestorKeys(tree, leaf, ancestors)
  return { checked: checkedKeys, half: [...ancestors] }
}

/* ---------- 新增/编辑弹窗 ---------- */
const submitting = ref(false)
/** 弹窗预加载（拉菜单树、角色详情）期间的新增/修改按钮 loading */
const preparing = ref(false)

/** 模板中对 modal.form 使用带类型视图（整体替换后经 computed 保持引用最新） */
const roleForm = computed(() => modal.form as RoleForm)

async function openRoleModal(title: string, form: RoleForm): Promise<void> {
  modal.title = title
  modal.form = form
  modal.open = true
  await nextTick()
  formRef.value?.clearValidate()
}

async function handleAddClick(): Promise<void> {
  preparing.value = true
  try {
    const [menus] = await Promise.all([menuTreeselect()])
    menuTree.value = menus ?? []
    menuCheckedKeys.value = []
    menuHalfCheckedKeys.value = []
    await openRoleModal(t('system.role.addTitle'), {
      status: '0',
      roleSort: 0,
      menuCheckStrictly: true,
      deptCheckStrictly: true,
      roleName: '',
      roleKey: '',
      remark: ''
    })
  } finally {
    preparing.value = false
  }
}

async function handleUpdateRow(record: TableData): Promise<void> {
  preparing.value = true
  try {
    const roleId = asRole(record).roleId
    const [roleRes, treeRes] = await Promise.all([getRole(roleId), roleMenuTreeselect(roleId)])
    menuTree.value = treeRes.menus ?? []
    // 后端 checkedKeys 已排除"有勾选子节点的父节点"（只含叶端勾选项），
    // 联动模式下 Arco 会自动向上传播得到全选/半选父级的渲染状态；
    // 半选祖先集合另行补算（见 echoCheckedKeys），保证直接保存不丢父级权限
    const echoed = echoCheckedKeys(menuTree.value, treeRes.checkedKeys ?? [])
    menuCheckedKeys.value = echoed.checked
    menuHalfCheckedKeys.value = echoed.half
    await openRoleModal(t('system.role.editTitle'), { ...roleRes })
  } finally {
    preparing.value = false
  }
}

function handleUpdateSelection(): void {
  const first = crud.selection.value[0]
  if (first) void handleUpdateRow(first)
}

async function onSubmit(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    const data: RoleForm = { ...roleForm.value, menuIds: collectMenuIds() }
    if (data.roleId != null) {
      await updateRole(data)
      Message.success(t('common.updateSuccess'))
    } else {
      await addRole(data)
      Message.success(t('common.addSuccess'))
    }
    modal.open = false
    await getList()
  } catch {
    // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
  } finally {
    submitting.value = false
  }
}

/* ---------- 状态切换 ---------- */
function beforeStatusChange(record: SysRole): Promise<boolean> {
  const newStatus = record.status === '0' ? '1' : '0'
  const action = newStatus === '0' ? t('common.enabled') : t('common.disabled')
  return new Promise((resolve) => {
    Modal.confirm({
      title: t('common.statusChangeConfirm'),
      content: t('system.role.statusChangeTip', { name: record.roleName, action }),
      hideCancel: false,
      onOk: async () => {
        try {
          await changeRoleStatus(record.roleId, newStatus)
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
  const role = asRole(record)
  handleDelete(role.roleId, role.roleName)
}

function handleAuthUserRow(record: TableData): void {
  router.push(`/system/role-auth/user/${asRole(record).roleId}`)
}

/* ---------- 数据权限 ---------- */
const dataScopeOpen = ref(false)
const dataScopeSaving = ref(false)
const deptTree = ref<DeptTreeNode[]>([])
const deptCheckedKeys = ref<TreeNodeKey[]>([])
const deptHalfCheckedKeys = ref<TreeNodeKey[]>([])
const dataScopeForm = reactive<RoleForm>({})

function onDeptCheck(keys: TreeNodeKey[], ev: { halfCheckedKeys?: TreeNodeKey[] }): void {
  deptCheckedKeys.value = keys
  deptHalfCheckedKeys.value = ev.halfCheckedKeys ?? []
}

function collectDeptIds(): number[] {
  return [...deptCheckedKeys.value, ...deptHalfCheckedKeys.value].map((key) => Number(key))
}

async function handleDataScopeRow(record: TableData): Promise<void> {
  const role = asRole(record)
  Object.assign(dataScopeForm, {
    roleId: role.roleId,
    roleName: role.roleName,
    roleKey: role.roleKey,
    dataScope: role.dataScope ?? '1',
    deptCheckStrictly: role.deptCheckStrictly ?? true
  })
  dataScopeOpen.value = true
  try {
    const res = await roleDeptTree(role.roleId)
    deptTree.value = res.depts ?? []
    const echoed = echoCheckedKeys(deptTree.value, res.checkedKeys ?? [])
    deptCheckedKeys.value = echoed.checked
    deptHalfCheckedKeys.value = echoed.half
  } catch {
    dataScopeOpen.value = false
  }
}

async function submitDataScope(): Promise<void> {
  if (!dataScopeForm.roleId) return
  dataScopeSaving.value = true
  try {
    await updateDataScope({
      roleId: dataScopeForm.roleId,
      dataScope: dataScopeForm.dataScope,
      deptCheckStrictly: dataScopeForm.deptCheckStrictly,
      // 与菜单权限一致：提交全选 + 半选，保证部门权限链路完整
      deptIds: dataScopeForm.dataScope === '2' ? collectDeptIds() : []
    })
    Message.success(t('system.role.dataScopeUpdated'))
    dataScopeOpen.value = false
    await getList()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    dataScopeSaving.value = false
  }
}

/* ---------- 初始化 ---------- */
void getList()
</script>

<style scoped>
.role-page__admin-tip {
  font-size: 12px;
  color: var(--color-text-3);
}

.role-page__menu-tree {
  width: 100%;
  max-height: 280px;
  padding: 8px;
  overflow-y: auto;
  border: 1px solid var(--color-border-2);
  border-radius: 4px;
}
</style>
