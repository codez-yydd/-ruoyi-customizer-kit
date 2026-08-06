<script setup lang="ts">
import { nextTick, onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElSelect,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTree,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete } from '@element-plus/icons-vue';

import { addRole, changeRoleStatus, dataScope, delRole, getRole, listRole, updateRole, type SysRole } from '#/api/system/role';
import { deptTreeSelect as getDeptTreeSelect } from '#/api/system/role';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import { parseTime } from '#/utils/ruoyi';

// 注意：menu 的 treeselect 已在 api/system/menu 提供，但角色编辑需菜单树勾选
import { treeselect as getMenuTreeselect, roleMenuTreeselect } from '#/api/system/menu';

defineOptions({ name: 'SystemRole' });

const { dictMap } = useDict({ status: 'sys_normal_disable' });
const { queryParams, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  roleName: '',
  roleKey: '',
  status: '',
});

const loading = ref(false);
const list = ref<SysRole[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

async function getList() {
  loading.value = true;
  try {
    const res = await listRole(queryParams);
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
  getList();
}
function handleSelectionChange(sel: SysRole[]) {
  ids.value = sel.map((i) => i.roleId);
  single.value = sel.length !== 1;
  multiple.value = !sel.length;
}

// ===== 状态切换 =====
async function handleStatusChange(row: SysRole) {
  const text = row.status === '0' ? '启用' : '停用';
  try {
    await ElMessageBox.confirm(`确认要"${text}""${row.roleName}"角色吗？`, '提示', { type: 'warning' });
    await changeRoleStatus(row.roleId, row.status!);
    ElMessage.success(text + '成功');
  } catch {
    row.status = row.status === '0' ? '1' : '0';
  }
}

// ===== 新增/编辑对话框（含菜单权限树） =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const menuTreeRef = ref<InstanceType<typeof ElTree>>();
const form = reactive<Partial<SysRole>>({});
const menuOptionsData = ref<any[]>([]);
const menuExpandAll = ref(false);
const menuCheckStrictly = ref(true);

const rules = {
  roleName: [{ required: true, message: '角色名称不能为空', trigger: 'blur' }],
  roleKey: [{ required: true, message: '权限字符不能为空', trigger: 'blur' }],
  roleSort: [{ required: true, message: '显示排序不能为空', trigger: 'blur' }],
};

function reset() {
  Object.assign(form, {
    roleId: undefined,
    roleName: '',
    roleKey: '',
    roleSort: 0,
    status: '0',
    menuIds: [],
    remark: '',
  });
  formRef.value?.resetFields();
}

/** 获取所有菜单树（新增时用） */
async function getMenuOptions() {
  menuOptionsData.value = await getMenuTreeselect();
}

/** 获取角色已有菜单并勾选（编辑时用） */
async function getRoleMenuTreeselect(roleId: number) {
  const res = await roleMenuTreeselect(roleId);
  menuOptionsData.value = res.menus ?? [];
  return res.checkedKeys ?? [];
}

async function handleAdd() {
  reset();
  await getMenuOptions();
  open.value = true;
  title.value = '添加角色';
  menuCheckStrictly.value = true;
  await nextTick();
  menuTreeRef.value?.setCheckedKeys([]);
}

async function handleUpdate(row?: SysRole) {
  reset();
  const roleId = row?.roleId ?? ids.value[0];
  if (!roleId) return;
  const res = await getRole(roleId);
  Object.assign(form, res);
  open.value = true;
  title.value = '修改角色';
  menuCheckStrictly.value = true;
  const checkedKeys = await getRoleMenuTreeselect(roleId);
  await nextTick();
  menuTreeRef.value?.setCheckedKeys(checkedKeys as any);
}

async function submitForm() {
  await formRef.value?.validate();
  // 收集勾选的菜单 id（含半选父节点）
  form.menuIds = [
    ...(menuTreeRef.value!.getCheckedKeys() as any[]),
    ...(menuTreeRef.value!.getHalfCheckedKeys() as any[]),
  ] as any;
  if (form.roleId) {
    await updateRole(form);
    ElMessage.success('修改成功');
  } else {
    await addRole(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysRole) {
  const roleIds = row.roleId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除角色编号为"${roleIds}"的数据项？`, '提示', { type: 'warning' });
    await delRole(roleIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

// ===== 数据权限对话框 =====
const openDataScope = ref(false);
const deptTreeScopeRef = ref<InstanceType<typeof ElTree>>();
const deptOptionsScope = ref<any[]>([]);
const formScope = reactive<Partial<SysRole>>({});

const dataScopeOptions = [
  { value: '1', label: '全部数据权限' },
  { value: '2', label: '自定义数据权限' },
  { value: '3', label: '本部门数据权限' },
  { value: '4', label: '本部门及以下数据权限' },
  { value: '5', label: '仅本人数据权限' },
];

async function handleDataScope(row: SysRole) {
  Object.assign(formScope, row);
  const res = await getDeptTreeSelect(row.roleId);
  deptOptionsScope.value = res.depts ?? [];
  openDataScope.value = true;
  await nextTick();
  deptTreeScopeRef.value?.setCheckedKeys(res.checkedKeys ?? []);
}

async function submitDataScope() {
  if (formScope.dataScope === '2') {
    formScope.deptIds = [
      ...(deptTreeScopeRef.value!.getCheckedKeys() as any[]),
      ...(deptTreeScopeRef.value!.getHalfCheckedKeys() as any[]),
    ] as any;
  }
  await dataScope(formScope);
  ElMessage.success('修改成功');
  openDataScope.value = false;
  getList();
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="角色名称">
        <ElInput v-model="queryParams.roleName" placeholder="请输入角色名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="权限字符">
        <ElInput v-model="queryParams.roleKey" placeholder="请输入权限字符" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect v-model="queryParams.status" placeholder="角色状态" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:role:add']" @click="handleAdd">新增</ElButton>
      <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:role:edit']" @click="handleUpdate()">修改</ElButton>
      <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:role:remove']" @click="handleDelete({} as SysRole)">删除</ElButton>
    </div>

    <ElTable v-loading="loading" :data="list" border @selection-change="handleSelectionChange">
      <ElTableColumn type="selection" width="50" align="center" />
      <ElTableColumn label="角色编号" prop="roleId" width="90" align="center" />
      <ElTableColumn label="角色名称" prop="roleName" show-overflow-tooltip />
      <ElTableColumn label="权限字符" prop="roleKey" show-overflow-tooltip />
      <ElTableColumn label="显示顺序" prop="roleSort" width="90" align="center" />
      <ElTableColumn label="状态" width="100" align="center">
        <template #default="{ row }">
          <ElSwitch v-model="row.status" active-value="0" inactive-value="1" @change="handleStatusChange(row)" />
        </template>
      </ElTableColumn>
      <ElTableColumn label="创建时间" prop="createTime" width="160" align="center">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="240" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton v-if="row.roleId !== 1" link type="primary" size="small" v-hasPermi="['system:role:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton v-if="row.roleId !== 1" link type="danger" size="small" v-hasPermi="['system:role:remove']" @click="handleDelete(row)">删除</ElButton>
          <ElButton v-if="row.roleId !== 1" link type="primary" size="small" v-hasPermi="['system:role:edit']" @click="handleDataScope(row)">数据权限</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <div class="pagination">
      <el-pagination v-model:current-page="queryParams.pageNum" v-model:page-size="queryParams.pageSize" :total="total" :page-sizes="[10, 20, 30, 50]" layout="total, sizes, prev, pager, next, jumper" background @size-change="getList" @current-change="getList" />
    </div>

    <!-- 新增/编辑角色 + 菜单权限树 -->
    <el-dialog v-model="open" :title="title" width="680px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-row>
          <el-col :span="12">
            <ElFormItem label="角色名称" prop="roleName"><ElInput v-model="form.roleName" placeholder="请输入角色名称" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="权限字符" prop="roleKey"><ElInput v-model="form.roleKey" placeholder="请输入权限字符" /></ElFormItem>
          </el-col>
        </el-row>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="角色顺序" prop="roleSort"><ElInputNumber v-model="form.roleSort" :min="0" controls-position="right" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="状态">
              <el-radio-group v-model="form.status">
                <el-radio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</el-radio>
              </el-radio-group>
            </ElFormItem>
          </el-col>
        </el-row>
        <ElFormItem label="菜单权限">
          <div class="menu-tree-wrap">
            <div class="menu-tree-toolbar">
              <el-checkbox v-model="menuExpandAll">展开/折叠</el-checkbox>
              <el-checkbox v-model="menuCheckStrictly">父子联动</el-checkbox>
            </div>
            <ElTree
              ref="menuTreeRef"
              :data="menuOptionsData"
              :props="{ label: 'label', children: 'children' }"
              show-checkbox
              node-key="id"
              :check-strictly="!menuCheckStrictly"
              :default-expand-all="menuExpandAll"
              class="menu-tree"
            />
          </div>
        </ElFormItem>
        <ElFormItem label="备注"><ElInput v-model="form.remark" type="textarea" placeholder="请输入内容" /></ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="open = false">取 消</ElButton>
      </template>
    </el-dialog>

    <!-- 数据权限对话框 -->
    <el-dialog v-model="openDataScope" title="分配数据权限" width="560px" append-to-body>
      <ElForm :model="formScope" label-width="100px">
        <ElFormItem label="角色名称"><ElInput v-model="formScope.roleName" disabled /></ElFormItem>
        <ElFormItem label="权限范围">
          <ElSelect v-model="formScope.dataScope">
            <ElOption v-for="d in dataScopeOptions" :key="d.value" :label="d.label" :value="d.value" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem v-if="formScope.dataScope === '2'" label="数据权限">
          <ElTree
            ref="deptTreeScopeRef"
            :data="deptOptionsScope"
            :props="{ label: 'label', children: 'children' }"
            show-checkbox
            node-key="id"
            check-strictly
            default-expand-all
            class="menu-tree"
          />
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitDataScope">确 定</ElButton>
        <ElButton @click="openDataScope = false">取 消</ElButton>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
@import '../_common/page.css';
.menu-tree-wrap {
  width: 100%;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  padding: 8px;
}
.menu-tree-toolbar {
  margin-bottom: 8px;
  display: flex;
  gap: 16px;
}
.menu-tree {
  max-height: 280px;
  overflow: auto;
}
</style>
