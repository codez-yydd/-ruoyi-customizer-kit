<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElRadio,
  ElRadioGroup,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTreeSelect,
} from 'element-plus';
import { Search, Refresh, Plus } from '@element-plus/icons-vue';

import { addMenu, delMenu, getMenu, listMenu, treeselect as getTreeselect, updateMenu, type SysMenu } from '#/api/system/menu';
import { useDict } from '#/composables/useDict';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'SystemMenu' });

const { dictMap } = useDict({ visible: 'sys_show_hide', status: 'sys_normal_disable' });

const loading = ref(false);
const list = ref<SysMenu[]>([]);
const queryParams = reactive({ menuName: '', visible: '' });

async function getList() {
  loading.value = true;
  try {
    list.value = await listMenu(queryParams);
  } finally {
    loading.value = false;
  }
}

function handleSearch() {
  getList();
}
function handleResetQuery() {
  queryParams.menuName = '';
  queryParams.visible = '';
  getList();
}

/**
 * 平铺转树（参考若依 handleTree）。
 *
 * 关键点：搜索等场景下后端只返回满足条件的菜单，可能不含父节点。
 * 此时若仍按 parentId===0 取根，会因父节点被过滤而整棵子树丢失（表现为“暂无数据”）。
 * 因此先把“父节点不在当前列表中”的项也视作根节点，保证搜索子菜单也能展示。
 */
function buildTree(items: SysMenu[], parentId = 0): SysMenu[] {
  const idSet = new Set(items.map((i) => i.menuId));
  const isRoot = (i: SysMenu) => i.parentId === parentId || !idSet.has(i.parentId);
  const recur = (roots: SysMenu[]): SysMenu[] =>
    roots
      .map((i) => ({ ...i, children: buildChildren(i.menuId) }))
      .sort((a, b) => (a.orderNum ?? 0) - (b.orderNum ?? 0));
  const buildChildren = (pid: number): SysMenu[] =>
    recur(items.filter((i) => i.parentId === pid));
  return recur(items.filter(isRoot));
}
const treeData = computed(() => buildTree(list.value));

// ===== 新增/编辑 =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysMenu>>({});
const menuTreeOptions = ref<any[]>([]);

const rules = {
  menuType: [{ required: true, message: '菜单类型不能为空', trigger: 'blur' }],
  menuName: [{ required: true, message: '菜单名称不能为空', trigger: 'blur' }],
  orderNum: [{ required: true, message: '显示排序不能为空', trigger: 'blur' }],
};

/** 是否显示路由地址/组件等（目录 M 和 菜单 C 显示） */
const isDirOrMenu = computed(() => form.menuType === 'M' || form.menuType === 'C');

function reset() {
  Object.assign(form, {
    menuId: undefined,
    parentId: 0,
    menuName: '',
    menuType: 'M',
    orderNum: 0,
    path: '',
    component: '',
    query: '',
    routeName: '',
    isFrame: '1',
    isCache: '0',
    visible: '0',
    status: '0',
    perms: '',
    icon: '',
  });
  formRef.value?.resetFields();
}

async function loadTreeOptions() {
  menuTreeOptions.value = await getTreeselect();
}

async function handleAdd(row?: SysMenu) {
  reset();
  await loadTreeOptions();
  if (row?.menuId) {
    form.parentId = row.menuId;
  }
  open.value = true;
  title.value = '添加菜单';
}

async function handleUpdate(row?: SysMenu) {
  reset();
  await loadTreeOptions();
  // 编辑时查详情
  const res = await getMenu((row as SysMenu).menuId);
  Object.assign(form, res.data);
  open.value = true;
  title.value = '修改菜单';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.menuId) {
    await updateMenu(form);
    ElMessage.success('修改成功');
  } else {
    await addMenu(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

async function handleDelete(row: SysMenu) {
  try {
    await ElMessageBox.confirm(`是否确认删除名称为"${row.menuName}"的菜单项？`, '提示', { type: 'warning' });
    await delMenu(row.menuId);
    getList();
    ElMessage.success('删除成功');
  } catch {
    /* 取消 */
  }
}

onMounted(getList);
</script>

<template>
  <div class="ruoyi-page">
    <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
      <ElFormItem label="菜单名称">
        <ElInput v-model="queryParams.menuName" placeholder="请输入菜单名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
      </ElFormItem>
      <ElFormItem label="状态">
        <ElSelect v-model="queryParams.visible" placeholder="菜单状态" clearable style="width: 200px">
          <ElOption v-for="d in dictMap.visible" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
        </ElSelect>
      </ElFormItem>
      <ElFormItem>
        <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
        <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
      </ElFormItem>
    </ElForm>

    <div class="toolbar">
      <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:menu:add']" @click="handleAdd()">新增</ElButton>
    </div>

    <ElTable v-loading="loading" :data="treeData" row-key="menuId" border>
      <ElTableColumn label="菜单名称" prop="menuName" width="200" />
      <ElTableColumn label="图标" prop="icon" width="80" align="center" />
      <ElTableColumn label="排序" prop="orderNum" width="80" align="center" />
      <ElTableColumn label="权限标识" prop="perms" show-overflow-tooltip />
      <ElTableColumn label="组件路径" prop="component" show-overflow-tooltip />
      <ElTableColumn label="状态" width="80" align="center">
        <template #default="{ row }"><DictTag :options="dictMap.status" :value="row.status" /></template>
      </ElTableColumn>
      <ElTableColumn label="创建时间" prop="createTime" width="160" align="center">
        <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
      </ElTableColumn>
      <ElTableColumn label="操作" width="220" align="center" fixed="right">
        <template #default="{ row }">
          <ElButton link type="primary" size="small" v-hasPermi="['system:menu:edit']" @click="handleUpdate(row)">修改</ElButton>
          <ElButton link type="primary" size="small" v-hasPermi="['system:menu:add']" @click="handleAdd(row)">新增</ElButton>
          <ElButton link type="danger" size="small" v-hasPermi="['system:menu:remove']" @click="handleDelete(row)">删除</ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <el-dialog v-model="open" :title="title" width="700px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-row>
          <el-col :span="24">
            <ElFormItem label="上级菜单" prop="parentId">
              <ElTreeSelect
                v-model="form.parentId"
                :data="[{ id: 0, label: '主类目', children: menuTreeOptions }]"
                :props="{ label: 'label', value: 'id', children: 'children' }"
                check-strictly
                value-key="id"
                placeholder="选择上级菜单"
                style="width: 100%"
              />
            </ElFormItem>
          </el-col>
        </el-row>
        <el-row>
          <el-col :span="24">
            <ElFormItem label="菜单类型" prop="menuType">
              <ElRadioGroup v-model="form.menuType">
                <ElRadio value="M">目录</ElRadio>
                <ElRadio value="C">菜单</ElRadio>
                <ElRadio value="F">按钮</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </el-col>
        </el-row>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="菜单名称" prop="menuName"><ElInput v-model="form.menuName" placeholder="请输入菜单名称" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="显示排序" prop="orderNum"><ElInputNumber v-model="form.orderNum" :min="0" controls-position="right" /></ElFormItem>
          </el-col>
        </el-row>
        <el-row v-if="isDirOrMenu">
          <el-col :span="12">
            <ElFormItem label="路由地址" prop="path"><ElInput v-model="form.path" placeholder="请输入路由地址" /></ElFormItem>
          </el-col>
          <el-col v-if="form.menuType === 'C'" :span="12">
            <ElFormItem label="路由名称" prop="routeName"><ElInput v-model="form.routeName" placeholder="请输入路由名称（英文）" /></ElFormItem>
          </el-col>
        </el-row>
        <el-row v-if="form.menuType === 'C'">
          <el-col :span="12">
            <ElFormItem label="组件路径" prop="component"><ElInput v-model="form.component" placeholder="如 system/user/index" /></ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="路由参数"><ElInput v-model="form.query" placeholder='如 {"id": 1}' /></ElFormItem>
          </el-col>
        </el-row>
        <el-row v-if="isDirOrMenu">
          <el-col :span="12">
            <ElFormItem label="是否外链">
              <ElRadioGroup v-model="form.isFrame">
                <ElRadio value="0">是</ElRadio>
                <ElRadio value="1">否</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </el-col>
          <el-col :span="12">
            <ElFormItem label="是否缓存">
              <ElRadioGroup v-model="form.isCache">
                <ElRadio value="0">缓存</ElRadio>
                <ElRadio value="1">不缓存</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </el-col>
        </el-row>
        <el-row>
          <el-col :span="12">
            <ElFormItem label="权限标识"><ElInput v-model="form.perms" placeholder="如 system:user:list" /></ElFormItem>
          </el-col>
          <el-col v-if="isDirOrMenu" :span="12">
            <ElFormItem label="显示状态">
              <ElRadioGroup v-model="form.visible">
                <ElRadio v-for="d in dictMap.visible" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </el-col>
        </el-row>
        <el-row v-if="isDirOrMenu">
          <el-col :span="12">
            <ElFormItem label="菜单状态">
              <ElRadioGroup v-model="form.status">
                <ElRadio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </el-col>
        </el-row>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="open = false">取 消</ElButton>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
@import '../_common/page.css';
</style>
