<template>
  <div class="dept-page">
    <!-- 搜索区 -->
    <a-card :bordered="false" class="app-page-card dept-page__search">
      <a-form :model="queryParams" layout="inline">
        <a-form-item field="deptName" :label="t('system.dept.deptName')">
          <a-input
            v-model.trim="queryParams.deptName"
            :placeholder="t('common.pleaseEnter', { field: t('system.dept.deptName') })"
            allow-clear
            style="width: 160px"
            @keyup.enter="getList"
            @clear="getList"
          />
        </a-form-item>
        <a-form-item field="status" :label="t('common.fields.status')">
          <a-select
            v-model="queryParams.status"
            :options="statusOptions"
            :placeholder="t('system.dept.statusPlaceholder')"
            allow-clear
            style="width: 140px"
            @change="getList"
          />
        </a-form-item>
        <a-form-item>
          <a-space>
            <a-button type="primary" @click="getList">
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
    </a-card>

    <!-- 树表区 -->
    <a-card :bordered="false" class="app-page-card">
      <div class="dept-page__toolbar">
        <a-space :size="8">
          <a-button v-hasPermi="['system:dept:add']" type="primary" @click="handleAdd()">
            <template #icon><IconPlus /></template>
            {{ t('common.add') }}
          </a-button>
          <a-button @click="expandAll">
            <template #icon><IconExpand /></template>
            {{ t('common.expandAll') }}
          </a-button>
          <a-button @click="collapseAll">
            <template #icon><IconShrink /></template>
            {{ t('common.collapseAll') }}
          </a-button>
        </a-space>
        <a-button size="small" :loading="loading" @click="getList">
          <template #icon><IconRefresh /></template>
        </a-button>
      </div>

      <a-table
        class="dept-page__table"
        :data="deptTree"
        :loading="loading"
        :pagination="false"
        row-key="deptId"
        :expanded-keys="expandedKeys"
        @expanded-keys-change="onExpandedKeysChange"
      >
        <template #columns>
          <a-table-column :title="t('system.dept.deptName')" data-index="deptName" :width="260" />
          <a-table-column :title="t('common.fields.sort')" data-index="orderNum" :width="80" align="center" />
          <a-table-column :title="t('system.dept.leader')" data-index="leader" :width="110" ellipsis tooltip />
          <a-table-column :title="t('system.dept.phone')" data-index="phone" :width="140" />
          <a-table-column :title="t('common.fields.email')" data-index="email" :width="180" ellipsis tooltip />
          <a-table-column :title="t('common.fields.status')" :width="90" align="center">
            <template #cell="{ record }">
              <DictTag :options="sysNormalDisable" :value="asDept(record).status" />
            </template>
          </a-table-column>
          <a-table-column :title="t('common.fields.createTime')" data-index="createTime" :width="170" />
          <a-table-column :title="t('common.fields.operation')" :width="200" align="center">
            <template #cell="{ record }">
              <a-space :size="4">
                <a-link v-hasPermi="['system:dept:edit']" @click="handleUpdate(asDept(record))">
                  {{ t('common.edit') }}
                </a-link>
                <a-link v-hasPermi="['system:dept:add']" @click="handleAdd(asDept(record))">
                  {{ t('common.add') }}
                </a-link>
                <a-link
                  v-hasPermi="['system:dept:remove']"
                  status="danger"
                  @click="handleDeleteRow(asDept(record))"
                >
                  {{ t('common.delete') }}
                </a-link>
              </a-space>
            </template>
          </a-table-column>
        </template>
      </a-table>
    </a-card>

    <!-- 新增/编辑部门弹窗 -->
    <a-modal
      :visible="modalOpen"
      :title="modalTitle"
      :width="600"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="modalOpen = false"
      @close="modalOpen = false"
    >
      <a-form ref="formRef" :model="form" :rules="formRules" auto-label-width>
        <a-form-item :label="t('system.dept.parentDept')">
          <a-tree-select
            v-model="form.parentId"
            :data="parentTree"
            :field-names="deptTreeFieldNames"
            :placeholder="t('system.dept.parentPlaceholder')"
            allow-clear
            allow-search
          />
        </a-form-item>
        <a-form-item field="deptName" :label="t('system.dept.deptName')">
          <a-input
            v-model.trim="form.deptName"
            :placeholder="t('common.pleaseEnter', { field: t('system.dept.deptName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="orderNum" :label="t('common.fields.displaySort')">
          <a-input-number v-model="form.orderNum" :min="0" :placeholder="t('common.pleaseEnter', { field: t('common.fields.displaySort') })" />
        </a-form-item>
        <a-form-item field="leader" :label="t('system.dept.leader')">
          <a-input
            v-model.trim="form.leader"
            :placeholder="t('common.pleaseEnter', { field: t('system.dept.leader') })"
            :max-length="20"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="phone" :label="t('system.dept.phone')">
          <a-input
            v-model.trim="form.phone"
            :placeholder="t('common.pleaseEnter', { field: t('system.dept.phone') })"
            :max-length="11"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="email" :label="t('common.fields.email')">
          <a-input
            v-model.trim="form.email"
            :placeholder="t('common.pleaseEnter', { field: t('common.fields.email') })"
            :max-length="50"
            allow-clear
          />
        </a-form-item>
        <a-form-item :label="t('system.dept.deptStatus')">
          <a-radio-group v-model="form.status">
            <a-radio v-for="item in sysNormalDisable" :key="item.dictValue" :value="item.dictValue">
              {{ item.dictLabel }}
            </a-radio>
          </a-radio-group>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref } from 'vue'
import type { FieldRule, FormInstance, TableData, TreeFieldNames } from '@arco-design/web-vue'
import type { TreeNodeKey } from '@arco-design/web-vue/es/tree/interface'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconExpand,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconShrink
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import DictTag from '@/components/DictTag/index.vue'
import {
  addDept,
  delDept,
  getDept,
  listDept,
  listDeptExcludeChild,
  updateDept
} from '@/api/system/dept'
import type { DeptQuery, SysDept } from '@/api/system/dept'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Dept' })

const { t } = useI18n()
const dicts = useDict('sys_normal_disable')
const sysNormalDisable = dicts['sys_normal_disable']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 弹窗上级部门树直接吃列表行结构（deptId/deptName） */
const deptTreeFieldNames: TreeFieldNames = {
  key: 'deptId',
  title: 'deptName',
  children: 'children'
}

/* ---------- 列表（全量非分页，平铺组装树） ---------- */
const loading = ref(false)
const rows = ref<SysDept[]>([])
const queryParams = reactive<DeptQuery>({})

/**
 * 平铺 -> 树结构兜底组装：
 * - 后端 /system/dept/list 实测返回平铺数组（children 恒为空数组，非树），
 *   仅当首项 children 非空时才视为后端已组装树并原样使用
 * - 按 parentId 关联组装；父节点不在当前集合（搜索过滤后父被排除）时提升为根，
 *   保证搜索结果可见；空 children 清理掉，避免树表渲染空展开箭头
 */
function buildDeptTree(list: SysDept[]): SysDept[] {
  if (list.length > 0 && Array.isArray(list[0].children) && list[0].children.length > 0) {
    return list
  }
  const nodeMap = new Map<number, SysDept>()
  for (const item of list) {
    item.children = []
    nodeMap.set(item.deptId, item)
  }
  const roots: SysDept[] = []
  for (const item of list) {
    const parent = item.parentId != null ? nodeMap.get(item.parentId) : undefined
    if (parent && parent !== item) {
      parent.children?.push(item)
    } else {
      roots.push(item)
    }
  }
  const cleanup = (nodes: SysDept[]): SysDept[] => {
    for (const node of nodes) {
      if (node.children && node.children.length > 0) {
        cleanup(node.children)
      } else {
        delete node.children
      }
    }
    return nodes
  }
  return cleanup(roots)
}

const deptTree = computed<SysDept[]>(() => buildDeptTree(rows.value))

async function getList(): Promise<void> {
  loading.value = true
  try {
    rows.value = (await listDept({ ...queryParams })) ?? []
    expandAll()
  } finally {
    loading.value = false
  }
}

function handleReset(): void {
  queryParams.deptName = undefined
  queryParams.status = undefined
  void getList()
}

function asDept(record: TableData): SysDept {
  return record as SysDept
}

/* ---------- 展开状态（受控） ---------- */
const expandedKeys = ref<TreeNodeKey[]>([])

function collectExpandKeys(nodes: SysDept[]): TreeNodeKey[] {
  const keys: TreeNodeKey[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      keys.push(node.deptId)
      keys.push(...collectExpandKeys(node.children))
    }
  }
  return keys
}

function expandAll(): void {
  expandedKeys.value = collectExpandKeys(deptTree.value)
}

function collapseAll(): void {
  expandedKeys.value = []
}

function onExpandedKeysChange(keys: TreeNodeKey[]): void {
  expandedKeys.value = keys
}

/* ---------- 新增/编辑弹窗 ---------- */
const formRef = ref<FormInstance>()
const modalOpen = ref(false)
const modalTitle = ref('')
const submitting = ref(false)
/** 上级部门树（编辑时用 exclude 接口排除自身子树） */
const parentTree = ref<SysDept[]>([])
const currentDeptId = ref<number>()

const form = reactive<Partial<SysDept>>({
  parentId: undefined,
  deptName: '',
  orderNum: 0,
  leader: '',
  phone: '',
  email: '',
  status: '0'
})

/** 表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  parentId: [{ required: true, message: t('common.pleaseSelect', { field: t('system.dept.parentDept') }) }],
  deptName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.dept.deptName') }) },
    { maxLength: 30, message: t('common.maxLengthTip', { max: 30 }) }
  ],
  orderNum: [{ required: true, message: t('common.pleaseEnter', { field: t('common.fields.displaySort') }) }],
  phone: [{ match: /^1[3-9]\d{9}$/, message: t('common.phoneInvalid') }],
  email: [{ match: /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/, message: t('common.emailInvalid') }]
}))

function assignForm(source: Partial<SysDept>): void {
  form.parentId = source.parentId
  form.deptName = source.deptName ?? ''
  form.orderNum = source.orderNum ?? 0
  form.leader = source.leader ?? ''
  form.phone = source.phone ?? ''
  form.email = source.email ?? ''
  form.status = source.status ?? '0'
}

/** 加载上级部门树：编辑时排除自身子树，新增时用全量列表 */
async function loadParentTree(deptId?: number): Promise<void> {
  try {
    const list = deptId != null ? await listDeptExcludeChild(deptId) : ((await listDept()) ?? [])
    parentTree.value = buildDeptTree(list)
  } catch {
    parentTree.value = []
  }
}

/** 打开新增：可传当前行作为上级（新增子部门） */
function handleAdd(row?: SysDept): void {
  currentDeptId.value = undefined
  assignForm({ parentId: row?.deptId })
  void loadParentTree()
  modalTitle.value = t('system.dept.addTitle')
  modalOpen.value = true
  void nextTick(() => formRef.value?.clearValidate())
}

async function handleUpdate(row: SysDept): Promise<void> {
  try {
    currentDeptId.value = row.deptId
    const [detail] = await Promise.all([getDept(row.deptId), loadParentTree(row.deptId)])
    assignForm(detail)
    modalTitle.value = t('system.dept.editTitle')
    modalOpen.value = true
    await nextTick()
    formRef.value?.clearValidate()
  } catch {
    // 详情加载失败：错误已由拦截器提示
  }
}

async function onSubmit(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    // 校验失败：错误信息已由表单展示
    return
  }
  submitting.value = true
  const data: Partial<SysDept> = {
    deptId: currentDeptId.value,
    parentId: form.parentId ?? 0,
    deptName: form.deptName,
    orderNum: form.orderNum,
    leader: form.leader,
    phone: form.phone,
    email: form.email,
    status: form.status
  }
  try {
    if (data.deptId != null) {
      await updateDept(data)
      Message.success(t('common.updateSuccess'))
    } else {
      delete data.deptId
      await addDept(data)
      Message.success(t('common.addSuccess'))
    }
    modalOpen.value = false
    await getList()
  } catch {
    // 提交失败（如上级部门选了自己/已存在同级同名部门）：提示已由拦截器弹出
  } finally {
    submitting.value = false
  }
}

/* ---------- 删除 ---------- */
function handleDeleteRow(row: SysDept): void {
  Modal.confirm({
    title: t('common.deleteConfirm'),
    content: t('system.dept.deleteConfirmTip', { name: row.deptName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await delDept(row.deptId)
        Message.success(t('common.deleteSuccess'))
        await getList()
      } catch {
        // 删除失败（存在下级部门/部门已分配用户）：提示已由拦截器弹出
      }
    }
  })
}

/* ---------- 初始化 ---------- */
void getList()
</script>

<style scoped>
.dept-page__search {
  margin-bottom: 12px;
}

.dept-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.dept-page__table {
  width: 100%;
}
</style>
