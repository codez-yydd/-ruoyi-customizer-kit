<template>
  <div class="menu-page">
    <!-- 搜索区 -->
    <a-card :bordered="false" class="app-page-card menu-page__search">
      <a-form :model="queryParams" layout="inline">
        <a-form-item field="menuName" :label="t('system.menu.menuName')">
          <a-input
            v-model.trim="queryParams.menuName"
            :placeholder="t('common.pleaseEnter', { field: t('system.menu.menuName') })"
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
            :placeholder="t('system.menu.statusPlaceholder')"
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
      <div class="menu-page__toolbar">
        <a-space :size="8">
          <a-button v-hasPermi="['system:menu:add']" type="primary" @click="handleAdd()">
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
        class="menu-page__table"
        :data="menuTree"
        :loading="loading"
        :pagination="false"
        row-key="menuId"
        v-model:expanded-keys="expandedKeys"
      >
        <template #columns>
          <a-table-column :title="t('system.menu.menuName')" :width="280">
            <template #cell="{ record }">
              <a-space :size="8">
                <AppIcon :name="asMenu(record).icon" />
                <span>{{ asMenu(record).menuName }}</span>
              </a-space>
            </template>
          </a-table-column>
          <a-table-column :title="t('common.fields.sort')" data-index="orderNum" :width="70" align="center" />
          <a-table-column :title="t('common.fields.permKey')" data-index="perms" :width="200" ellipsis tooltip />
          <a-table-column :title="t('system.menu.component')" data-index="component" :width="200" ellipsis tooltip />
          <a-table-column :title="t('common.fields.status')" :width="90" align="center">
            <template #cell="{ record }">
              <DictTag :options="sysNormalDisable" :value="asMenu(record).status" />
            </template>
          </a-table-column>
          <a-table-column :title="t('system.menu.visibleColumn')" :width="90" align="center">
            <template #cell="{ record }">
              <DictTag :options="sysShowHide" :value="asMenu(record).visible" />
            </template>
          </a-table-column>
          <a-table-column :title="t('common.fields.createTime')" data-index="createTime" :width="170" />
          <a-table-column :title="t('common.fields.operation')" :width="220" align="center">
            <template #cell="{ record }">
              <a-space :size="4">
                <a-link v-hasPermi="['system:menu:edit']" @click="handleUpdate(asMenu(record))">
                  {{ t('common.edit') }}
                </a-link>
                <a-link v-hasPermi="['system:menu:add']" @click="handleAdd(asMenu(record))">
                  {{ t('common.add') }}
                </a-link>
                <a-link
                  v-hasPermi="['system:menu:remove']"
                  status="danger"
                  @click="handleDeleteRow(asMenu(record))"
                >
                  {{ t('common.delete') }}
                </a-link>
              </a-space>
            </template>
          </a-table-column>
        </template>
      </a-table>
    </a-card>

    <!-- 新增/编辑菜单弹窗 -->
    <a-modal
      :visible="modalOpen"
      :title="modalTitle"
      :width="720"
      :mask-closable="false"
      :ok-loading="submitting"
      @ok="onSubmit"
      @cancel="modalOpen = false"
      @close="modalOpen = false"
    >
      <a-form ref="formRef" :model="form" :rules="formRules" auto-label-width>
        <a-form-item :label="t('system.menu.parentMenu')">
          <a-tree-select
            v-model="form.parentId"
            :data="parentTree"
            :field-names="treeFieldNames"
            :placeholder="t('system.menu.parentPlaceholder')"
            allow-clear
            allow-search
          />
        </a-form-item>
        <a-form-item :label="t('system.menu.menuType')">
          <a-radio-group v-model="form.menuType">
            <a-radio value="M">{{ t('system.menu.typeDir') }}</a-radio>
            <a-radio value="C">{{ t('system.menu.typeMenu') }}</a-radio>
            <a-radio value="F">{{ t('system.menu.typeButton') }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item v-if="form.menuType !== 'F'" :label="t('system.menu.icon')">
          <IconSelect v-model="form.icon" />
        </a-form-item>
        <a-form-item field="menuName" :label="t('system.menu.menuName')">
          <a-input
            v-model.trim="form.menuName"
            :placeholder="t('common.pleaseEnter', { field: t('system.menu.menuName') })"
            allow-clear
          />
        </a-form-item>
        <a-form-item field="orderNum" :label="t('common.fields.displaySort')">
          <a-input-number v-model="form.orderNum" :min="0" :placeholder="t('common.pleaseEnter', { field: t('common.fields.displaySort') })" />
        </a-form-item>
        <template v-if="form.menuType !== 'F'">
          <a-form-item :label="t('system.menu.isFrame')">
            <a-radio-group v-model="form.isFrame">
              <a-radio value="0">{{ t('common.yes') }}</a-radio>
              <a-radio value="1">{{ t('common.no') }}</a-radio>
            </a-radio-group>
          </a-form-item>
          <a-form-item field="path" :label="t('system.menu.path')">
            <a-input
              v-model.trim="form.path"
              :placeholder="form.isFrame === '0' ? t('system.menu.pathFramePlaceholder') : t('common.pleaseEnter', { field: t('system.menu.path') })"
              allow-clear
            />
          </a-form-item>
        </template>
        <a-form-item v-if="form.menuType === 'C' && form.isFrame === '1'" field="component" :label="t('system.menu.component')">
          <a-input v-model.trim="form.component" :placeholder="t('system.menu.componentPlaceholder')" allow-clear />
        </a-form-item>
        <a-form-item v-if="form.menuType === 'C' && form.isFrame === '1'" field="query" :label="t('system.menu.query')">
          <a-input
            v-model.trim="form.query"
            :placeholder="t('system.menu.queryPlaceholder', { example: '{&quot;id&quot;: 1}' })"
            allow-clear
          />
        </a-form-item>
        <a-form-item v-if="form.menuType !== 'M'" field="perms" :label="t('common.fields.permKey')">
          <a-input v-model.trim="form.perms" :placeholder="t('system.menu.permsPlaceholder')" allow-clear />
        </a-form-item>
        <a-form-item v-if="form.menuType === 'C'" :label="t('system.menu.isCache')">
          <a-radio-group v-model="form.isCache">
            <a-radio value="0">{{ t('system.menu.cached') }}</a-radio>
            <a-radio value="1">{{ t('system.menu.notCached') }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item v-if="form.menuType !== 'F'" :label="t('system.menu.showStatus')">
          <a-radio-group v-model="form.visible">
            <a-radio value="0">{{ t('system.menu.show') }}</a-radio>
            <a-radio value="1">{{ t('system.menu.hide') }}</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item :label="t('system.menu.menuStatus')">
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
import { computed, markRaw, nextTick, reactive, ref, shallowRef } from 'vue'
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
import AppIcon from '@/components/AppIcon/index.vue'
import IconSelect from '@/components/IconSelect/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import { addMenu, delMenu, getMenu, listMenu, updateMenu } from '@/api/system/menu'
import type { MenuQuery, SysMenu } from '@/api/system/menu'
import { menuTreeselect } from '@/api/system/role'
import type { MenuTreeNode } from '@/api/system/role'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Menu' })

const { t } = useI18n()
const dicts = useDict('sys_normal_disable', 'sys_show_hide')
const sysNormalDisable = dicts['sys_normal_disable']
const sysShowHide = dicts['sys_show_hide']

const statusOptions = computed(() =>
  sysNormalDisable.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

const treeFieldNames: TreeFieldNames = { key: 'id', title: 'label', children: 'children' }

/* ---------- 列表（全量非分页，平铺组装树） ---------- */
const loading = ref(false)
const rows = ref<SysMenu[]>([])
const queryParams = reactive<MenuQuery>({})

/**
 * 平铺 -> 树结构兜底组装：
 * - 后端 /system/menu/list 实测返回平铺数组（children 恒为空数组，非树），
 *   仅当首项 children 非空时才视为后端已组装树；此时递归浅拷贝，不 mutate 入参
 * - 按 parentId 关联组装；父节点不在当前集合（按名称搜索后父被排除）时提升为根，
 *   保证搜索结果可见；空 children 清理掉，避免树表渲染空展开箭头
 * - 每个节点浅拷贝后 markRaw，禁止写回原 list 项的 children，避免深层响应式脏检查
 */
function cloneMenuNode(item: SysMenu): SysMenu {
  const node: SysMenu = { ...item }
  if (Array.isArray(item.children) && item.children.length > 0) {
    node.children = item.children.map(cloneMenuNode)
  } else {
    delete node.children
  }
  return markRaw(node)
}

function buildMenuTree(list: SysMenu[]): SysMenu[] {
  if (list.length > 0 && Array.isArray(list[0].children) && list[0].children.length > 0) {
    return list.map(cloneMenuNode)
  }
  const nodeMap = new Map<number, SysMenu>()
  for (const item of list) {
    const node: SysMenu = { ...item }
    node.children = []
    nodeMap.set(item.menuId, markRaw(node))
  }
  const roots: SysMenu[] = []
  for (const item of list) {
    const node = nodeMap.get(item.menuId)!
    const parent = item.parentId != null ? nodeMap.get(item.parentId) : undefined
    if (parent && parent !== node) {
      parent.children?.push(node)
    } else {
      roots.push(node)
    }
  }
  const cleanup = (nodes: SysMenu[]): SysMenu[] => {
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

const menuTree = shallowRef<SysMenu[]>([])

async function getList(): Promise<void> {
  loading.value = true
  try {
    const list = (await listMenu({ ...queryParams })) ?? []
    rows.value = list
    menuTree.value = buildMenuTree(list)
    expandFirstLevel()
  } finally {
    loading.value = false
  }
}

function handleReset(): void {
  queryParams.menuName = undefined
  queryParams.status = undefined
  void getList()
}

function asMenu(record: TableData): SysMenu {
  return record as SysMenu
}

/* ---------- 展开状态（受控） ---------- */
const expandedKeys = shallowRef<TreeNodeKey[]>([])

/** 默认只展目录层，避免 F 按钮行一次性铺开导致卡顿 */
function collectFirstLevelExpandKeys(nodes: SysMenu[]): TreeNodeKey[] {
  const keys: TreeNodeKey[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      keys.push(node.menuId)
    }
  }
  return keys
}

function expandFirstLevel(): void {
  expandedKeys.value = collectFirstLevelExpandKeys(menuTree.value)
}

/** 收集全部含子级的 menuId 作为展开键 */
function collectExpandKeys(nodes: SysMenu[]): TreeNodeKey[] {
  const keys: TreeNodeKey[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      keys.push(node.menuId)
      keys.push(...collectExpandKeys(node.children))
    }
  }
  return keys
}

function expandAll(): void {
  expandedKeys.value = collectExpandKeys(menuTree.value)
}

function collapseAll(): void {
  expandedKeys.value = []
}

/* ---------- 新增/编辑弹窗 ---------- */
const formRef = ref<FormInstance>()
const modalOpen = ref(false)
const modalTitle = ref('')
const submitting = ref(false)
const parentTree = ref<MenuTreeNode[]>([])
/** 编辑前的原父级 id（用于判断修改时是否改变了上级） */
const originalForm = ref<Partial<SysMenu>>({})

const form = reactive<Partial<SysMenu>>({
  parentId: undefined,
  menuType: 'M',
  icon: undefined,
  menuName: '',
  orderNum: 0,
  isFrame: '1',
  path: '',
  component: '',
  query: '',
  perms: '',
  isCache: '0',
  visible: '0',
  status: '0'
})

/** 路由地址校验：必填；外链时必须以 http(s):// 开头 */
const pathRules = computed<FieldRule[]>(() => [
  { required: true, message: t('common.pleaseEnter', { field: t('system.menu.path') }) },
  {
    validator: (value: string | undefined, callback: (error?: string) => void): void => {
      if (form.isFrame === '0' && value && !/^https?:\/\//.test(value)) {
        callback(t('system.menu.pathInvalid'))
        return
      }
      callback()
    }
  }
])

/** 表单校验规则（computed：随语言切换联动提示语） */
const formRules = computed<Record<string, FieldRule[]>>(() => ({
  menuName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.menu.menuName') }) },
    { maxLength: 50, message: t('common.maxLengthTip', { max: 50 }) }
  ],
  orderNum: [{ required: true, message: t('common.pleaseEnter', { field: t('common.fields.displaySort') }) }],
  ...(form.menuType !== 'F' ? { path: pathRules.value } : {}),
  ...(form.menuType === 'C' && form.isFrame === '1'
    ? { component: [{ required: true, message: t('common.pleaseEnter', { field: t('system.menu.component') }) }] }
    : {})
}))

/** 重置弹窗表单（openModal 传初值后逐字段覆盖，避免残留上一次状态） */
function assignForm(source: Partial<SysMenu>): void {
  form.parentId = source.parentId
  form.menuType = source.menuType ?? 'M'
  form.icon = source.icon
  form.menuName = source.menuName ?? ''
  form.orderNum = source.orderNum ?? 0
  form.isFrame = source.isFrame ?? '1'
  form.path = source.path ?? ''
  form.component = source.component ?? ''
  form.query = source.query ?? ''
  form.perms = source.perms ?? ''
  form.isCache = source.isCache ?? '0'
  form.visible = source.visible ?? '0'
  form.status = source.status ?? '0'
}

async function openModal(title: string, init: Partial<SysMenu>): Promise<void> {
  try {
    parentTree.value = (await menuTreeselect()) ?? []
  } catch {
    // 上级菜单树加载失败时仍可打开弹窗（根菜单场景），错误已由拦截器提示
    parentTree.value = []
  }
  assignForm(init)
  originalForm.value = { ...init }
  modalTitle.value = title
  modalOpen.value = true
  await nextTick()
  formRef.value?.clearValidate()
}

/** 打开新增：可传当前行作为上级（新增子菜单），菜单类型按父级推导（目录->菜单->按钮） */
function handleAdd(row?: SysMenu): void {
  const parentId = row?.menuId
  let menuType = 'M'
  if (row) {
    menuType = row.menuType === 'M' ? 'C' : 'F'
  }
  void openModal(t('system.menu.addTitle'), {
    parentId,
    menuType,
    orderNum: 0,
    isFrame: '1',
    isCache: '0',
    visible: '0',
    status: '0'
  })
}

async function handleUpdate(row: SysMenu): Promise<void> {
  try {
    const detail = await getMenu(row.menuId)
    await openModal(t('system.menu.editTitle'), { ...detail })
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
  // 提交字段裁剪：去掉页面态冗余（children/parentName/createTime 等）
  const data: Partial<SysMenu> = {
    menuId: originalForm.value.menuId,
    parentId: form.parentId ?? 0,
    menuType: form.menuType,
    icon: form.icon,
    menuName: form.menuName,
    orderNum: form.orderNum,
    isFrame: form.isFrame,
    path: form.menuType === 'F' ? '' : form.path,
    component: form.menuType === 'C' && form.isFrame === '1' ? form.component : '',
    query: form.menuType === 'C' && form.isFrame === '1' ? form.query : '',
    perms: form.perms,
    isCache: form.isCache,
    visible: form.menuType === 'F' ? '0' : form.visible,
    status: form.status
  }
  try {
    if (data.menuId != null) {
      await updateMenu(data)
      Message.success(t('common.updateSuccess'))
    } else {
      delete data.menuId
      await addMenu(data)
      Message.success(t('common.addSuccess'))
    }
    modalOpen.value = false
    await getList()
  } catch {
    // 提交失败：错误提示已由响应拦截器统一弹出，弹窗保持打开
  } finally {
    submitting.value = false
  }
}

/* ---------- 删除 ---------- */
function handleDeleteRow(row: SysMenu): void {
  Modal.confirm({
    title: t('common.deleteConfirm'),
    content: t('system.menu.deleteConfirmTip', { name: row.menuName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await delMenu(row.menuId)
        Message.success(t('common.deleteSuccess'))
        await getList()
      } catch {
        // 删除失败（如存在子菜单/菜单已分配）：错误提示已由拦截器弹出
      }
    }
  })
}

/* ---------- 初始化 ---------- */
void getList()
</script>

<style scoped>
.menu-page__search {
  margin-bottom: 12px;
}

.menu-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.menu-page__table {
  width: 100%;
}
</style>
