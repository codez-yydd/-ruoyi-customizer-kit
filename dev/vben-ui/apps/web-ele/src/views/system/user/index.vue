<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, useTemplateRef } from 'vue';
import { useRouter } from 'vue-router';

import {
  ElButton,
  ElCol,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElMessageBox,
  ElOption,
  ElRadio,
  ElRadioGroup,
  ElRow,
  ElSelect,
  ElSwitch,
  ElTable,
  ElTableColumn,
} from 'element-plus';
import { Search, Refresh, Plus, Edit, Delete, Upload, Download } from '@element-plus/icons-vue';

import {
  addUser,
  changeUserStatus,
  delUser,
  deptTreeSelect,
  downloadUserTemplate,
  exportUser,
  getUser,
  importUser,
  listUser,
  resetUserPwd,
  updateUser,
  type SysUser,
} from '#/api/system/user';
import { getConfigKey } from '#/api/system/config';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import { addDateRange, parseTime, saveBlobFile } from '#/utils/ruoyi';

defineOptions({ name: 'SystemUser' });

const router = useRouter();

// ===== 字典 =====
const { dictMap } = useDict({ sex: 'sys_user_sex', status: 'sys_normal_disable' });

// ===== 分页/查询 =====
const { queryParams, dateRange, total, handleQuery, resetQuery: resetQueryBase } = usePagination({
  userName: '',
  phonenumber: '',
  status: '',
  deptId: undefined,
});

// ===== 列表状态 =====
const loading = ref(false);
const userList = ref<SysUser[]>([]);
const ids = ref<number[]>([]);
const single = ref(true);
const multiple = ref(true);

// ===== 表格自适应高度 =====
// 让表格撑满内容区剩余空间（减去搜索栏/工具栏/分页），避免表格太矮
const tableWrapRef = useTemplateRef<HTMLElement>('tableWrapRef');
const tableHeight = ref<number>(0);

function calcTableHeight() {
  const wrap = tableWrapRef.value;
  if (!wrap) return;
  // 父容器（.ruoyi-content）高度 - 分页高度(约 56) - 内边距
  const parentH = wrap.parentElement?.clientHeight ?? 0;
  tableHeight.value = Math.max(parentH - 56, 240);
}

let resizeObserver: null | ResizeObserver = null;

// ===== 部门树 =====
const deptOptions = ref<any[]>([]);
const showDeptTree = ref(true);
const deptFilter = ref('');

const filteredDeptOptions = computed(() => deptOptions.value);

async function getDeptTree() {
  const data = await deptTreeSelect();
  deptOptions.value = data ?? [];
}

function handleNodeClick(data: any) {
  queryParams.deptId = data.id;
  getList();
}

// ===== 列表查询 =====
async function getList() {
  loading.value = true;
  try {
    const params = addDateRange({ ...queryParams }, dateRange.value, 'CreateTime');
    const res = await listUser(params);
    userList.value = res.rows ?? [];
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

// ===== 选择 =====
function handleSelectionChange(selection: SysUser[]) {
  ids.value = selection.map((item) => item.userId);
  single.value = selection.length !== 1;
  multiple.value = !selection.length;
}

// ===== 状态切换 =====
async function handleStatusChange(row: SysUser) {
  const text = row.status === '0' ? '启用' : '停用';
  try {
    await ElMessageBox.confirm(`确认要"${text}""${row.userName}"用户吗？`, '提示', {
      type: 'warning',
    });
    await changeUserStatus(row.userId, row.status!);
    ElMessage.success(text + '成功');
  } catch {
    row.status = row.status === '0' ? '1' : '0';
  }
}

// ===== 新增/编辑对话框 =====
const open = ref(false);
const title = ref('');
const formRef = ref();
const form = reactive<Partial<SysUser> & { password?: string }>({});
const postOptions = ref<{ postId: number; postName: string; status: string }[]>([]);
const roleOptions = ref<{ roleId: number; roleName: string; status: string }[]>([]);
const initPassword = ref('');

const rules = {
  userName: [
    { required: true, message: '用户名称不能为空', trigger: 'blur' },
    { min: 2, max: 20, message: '用户名称长度必须介于 2 和 20 之间', trigger: 'blur' },
  ],
  nickName: [{ required: true, message: '用户昵称不能为空', trigger: 'blur' }],
  email: [{ type: 'email' as const, message: '请输入正确的邮箱地址', trigger: ['blur', 'change'] }],
  phonenumber: [
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号码', trigger: 'blur' },
  ],
};

function reset() {
  Object.assign(form, {
    userId: undefined,
    deptId: undefined,
    userName: undefined,
    nickName: undefined,
    password: undefined,
    phonenumber: undefined,
    email: undefined,
    sex: '0',
    status: '0',
    remark: undefined,
    postIds: [],
    roleIds: [],
  });
  formRef.value?.resetFields();
}

async function handleAdd() {
  reset();
  const res = await getUser();
  postOptions.value = res.posts ?? [];
  roleOptions.value = res.roles ?? [];
  form.password = initPassword.value;
  open.value = true;
  title.value = '添加用户';
}

async function handleUpdate(row?: SysUser) {
  reset();
  const userId = row?.userId ?? ids.value[0];
  const res = await getUser(userId);
  Object.assign(form, res.data);
  form.postIds = res.postIds;
  form.roleIds = res.roleIds;
  form.password = '';
  postOptions.value = res.posts ?? [];
  roleOptions.value = res.roles ?? [];
  open.value = true;
  title.value = '修改用户';
}

async function submitForm() {
  await formRef.value?.validate();
  if (form.userId) {
    await updateUser(form);
    ElMessage.success('修改成功');
  } else {
    await addUser(form);
    ElMessage.success('新增成功');
  }
  open.value = false;
  getList();
}

function cancel() {
  open.value = false;
  reset();
}

// ===== 删除 =====
async function handleDelete(row: SysUser) {
  const userIds = row.userId || ids.value;
  try {
    await ElMessageBox.confirm(`是否确认删除用户编号为"${userIds}"的数据项？`, '提示', {
      type: 'warning',
    });
    await delUser(userIds as any);
    getList();
    ElMessage.success('删除成功');
  } catch {
    // 取消
  }
}

// ===== 重置密码 =====
async function handleResetPwd(row: SysUser) {
  try {
    const { value } = await ElMessageBox.prompt(`请输入「${row.userName}」的新密码`, '重置密码', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputPattern: /.{5,20}/,
      inputErrorMessage: '密码长度 5-20 位',
    });
    await resetUserPwd(row.userId, value);
    ElMessage.success('修改成功，新密码是：' + value);
  } catch {
    // 取消
  }
}

// ===== 分配角色 =====
// 跳转到独立分配角色页（views/system/user/authRole.vue），路由通过 builtinMenus 注入。
function handleAuthRole(row: SysUser) {
  router.push(`/system/user-auth/role/${row.userId}`);
}

// ===== 导出 =====
// 若依导出：POST /system/user/export，返回 Excel 二进制流。
// 若勾选了行则按所选导出，否则按当前查询条件导出。
async function handleExport() {
  try {
    await ElMessageBox.confirm('是否确认导出所有用户数据项?', '警告', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning',
    });
  } catch {
    return; // 用户取消
  }
  // 选中了行就只导选中的（通过 userName/ids 限定），否则按查询条件导出
  const params =
    ids.value.length > 0
      ? undefined
      : addDateRange({ ...queryParams }, dateRange.value, 'CreateTime');
  const response: any = await exportUser(params);
  await saveBlobFile(response, 'user.xlsx');
  ElMessage.success('导出成功');
}

// ===== 导入 =====
const importOpen = ref(false);
const importUploading = ref(false);
const importUpdateSupport = ref(false);
const importFileRef = ref();

function handleImport() {
  importOpen.value = true;
  importUpdateSupport.value = false;
}

// el-upload http-request 自定义上传：选中文件后立即上传（弹框内拖拽/点击即触发）。
// options.file 是原生 File；必须返回 false / Promise 以阻止 el-upload 默认请求。
async function handleImportSubmit(options: { file: File }) {
  const file = options.file;
  if (!/\.(xlsx|xls)$/i.test(file.name)) {
    ElMessage.error('仅允许导入 xls、xlsx 格式文件');
    return false;
  }
  importUploading.value = true;
  try {
    // requestClient.upload 走全局拦截器解包，若依导入成功返回 {code:200,msg}
    await importUser(file, importUpdateSupport.value);
    ElMessage.success('导入成功');
    importOpen.value = false;
    getList();
  } finally {
    importUploading.value = false;
  }
  return false;
}

async function handleImportTemplate() {
  const response: any = await downloadUserTemplate();
  await saveBlobFile(response, 'user_template.xlsx');
}

function handleImportClose() {
  importOpen.value = false;
  // 重置 el-upload 状态（清空已选文件列表）
  importFileRef.value?.clearFiles?.();
}

// ===== 列显隐 =====
const columns = reactive({
  userId: true,
  userName: true,
  nickName: true,
  deptName: true,
  phonenumber: true,
  status: true,
  createTime: true,
});

onMounted(() => {
  getList();
  getDeptTree();
  // 取系统参数「sys.user.initPassword」作为新增用户默认密码（与若依原版一致）。
  // getConfigKey 返回 {code:200,data:"明文密码"}，全局拦截器解包后得到字符串。
  getConfigKey('sys.user.initPassword').then((pwd: any) => {
    initPassword.value = typeof pwd === 'string' ? pwd : '';
  });
  // 表格高度自适应
  nextTick(() => {
    calcTableHeight();
    resizeObserver = new ResizeObserver(() => calcTableHeight());
    if (tableWrapRef.value?.parentElement) {
      resizeObserver.observe(tableWrapRef.value.parentElement);
    }
  });
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
});
</script>

<template>
  <div class="ruoyi-page">
    <!-- 部门树侧边栏 -->
    <div v-show="showDeptTree" class="dept-sidebar">
      <div class="dept-sidebar__head">
        <span>组织机构</span>
        <ElButton link size="small" @click="getDeptTree">刷新</ElButton>
      </div>
      <ElInput
        v-model="deptFilter"
        placeholder="请输入部门名称"
        size="small"
        clearable
        class="dept-filter"
      />
      <div class="dept-tree">
        <el-tree
          :data="filteredDeptOptions"
          :props="{ label: 'label', children: 'children' }"
          :filter-node-method="(value: string, data: any) => !value || data.label.includes(value)"
          node-key="id"
          highlight-current
          default-expand-all
          @node-click="handleNodeClick"
        />
      </div>
    </div>

    <div class="ruoyi-content">
      <!-- 搜索栏 -->
      <ElForm :inline="true" :model="queryParams" size="small" class="search-form">
        <ElFormItem label="用户名称">
          <ElInput v-model="queryParams.userName" placeholder="请输入用户名称" clearable style="width: 200px" @keyup.enter="handleSearch" />
        </ElFormItem>
        <ElFormItem label="手机号码">
          <ElInput v-model="queryParams.phonenumber" placeholder="请输入手机号码" clearable style="width: 200px" @keyup.enter="handleSearch" />
        </ElFormItem>
        <ElFormItem label="状态">
          <ElSelect v-model="queryParams.status" placeholder="用户状态" clearable style="width: 200px">
            <ElOption v-for="d in dictMap.status" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="创建时间">
          <el-date-picker v-model="dateRange" style="width: 240px" value-format="YYYY-MM-DD" type="daterange" range-separator="-" start-placeholder="开始日期" end-placeholder="结束日期" />
        </ElFormItem>
        <ElFormItem>
          <ElButton type="primary" :icon="Search" @click="handleSearch">搜索</ElButton>
          <ElButton :icon="Refresh" @click="handleResetQuery">重置</ElButton>
        </ElFormItem>
      </ElForm>

      <!-- 工具栏 -->
      <div class="toolbar">
        <ElButton type="primary" plain :icon="Plus" v-hasPermi="['system:user:add']" @click="handleAdd">新增</ElButton>
        <ElButton type="success" plain :icon="Edit" :disabled="single" v-hasPermi="['system:user:edit']" @click="handleUpdate(undefined)">修改</ElButton>
        <ElButton type="danger" plain :icon="Delete" :disabled="multiple" v-hasPermi="['system:user:remove']" @click="handleDelete({} as SysUser)">删除</ElButton>
        <ElButton type="info" plain :icon="Upload" v-hasPermi="['system:user:import']" @click="handleImport">导入</ElButton>
        <ElButton type="warning" plain :icon="Download" v-hasPermi="['system:user:export']" @click="handleExport">导出</ElButton>
      </div>

      <!-- 表格 -->
      <div ref="tableWrapRef" class="table-wrap">
        <ElTable v-loading="loading" :data="userList" :height="tableHeight" border @selection-change="handleSelectionChange">
        <ElTableColumn type="selection" width="50" align="center" />
        <ElTableColumn v-if="columns.userId" label="用户编号" align="center" prop="userId" width="100" />
        <ElTableColumn v-if="columns.userName" label="用户名称" align="center" prop="userName" show-overflow-tooltip />
        <ElTableColumn v-if="columns.nickName" label="用户昵称" align="center" prop="nickName" show-overflow-tooltip />
        <ElTableColumn v-if="columns.deptName" label="部门" align="center" show-overflow-tooltip>
          <template #default="{ row }">{{ row.dept?.deptName }}</template>
        </ElTableColumn>
        <ElTableColumn v-if="columns.phonenumber" label="手机号码" align="center" prop="phonenumber" width="120" />
        <ElTableColumn v-if="columns.status" label="状态" align="center" width="80">
          <template #default="{ row }">
            <ElSwitch v-model="row.status" active-value="0" inactive-value="1" @change="handleStatusChange(row)" />
          </template>
        </ElTableColumn>
        <ElTableColumn v-if="columns.createTime" label="创建时间" align="center" prop="createTime" width="160">
          <template #default="{ row }">{{ parseTime(row.createTime) }}</template>
        </ElTableColumn>
        <ElTableColumn label="操作" align="center" width="200" fixed="right">
          <template #default="{ row }">
            <template v-if="row.userId !== 1">
              <ElButton link type="primary" size="small" v-hasPermi="['system:user:edit']" @click="handleUpdate(row)">修改</ElButton>
              <ElButton link type="danger" size="small" v-hasPermi="['system:user:remove']" @click="handleDelete(row)">删除</ElButton>
              <el-dropdown size="small" @command="(cmd: string) => cmd === 'reset' ? handleResetPwd(row) : handleAuthRole(row)">
                <ElButton link type="primary" size="small">更多</ElButton>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="reset" v-hasPermi="['system:user:resetPwd']">重置密码</el-dropdown-item>
                    <el-dropdown-item command="auth" v-hasPermi="['system:user:edit']">分配角色</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </template>
          </template>
        </ElTableColumn>
      </ElTable>
      </div>

      <!-- 分页 -->
      <div class="pagination">
        <el-pagination
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

    <!-- 新增/修改对话框 -->
    <el-dialog v-model="open" :title="title" width="680px" append-to-body>
      <ElForm ref="formRef" :model="form" :rules="rules" label-width="80px">
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="用户昵称" prop="nickName">
              <ElInput v-model="form.nickName" placeholder="请输入用户昵称" maxlength="30" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="归属部门" prop="deptId">
              <el-tree-select v-model="form.deptId" :data="deptOptions" :props="{ value: 'id', label: 'label', children: 'children', disabled: 'disabled' }" check-strictly node-key="id" placeholder="请选择归属部门" style="width: 100%" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="手机号码" prop="phonenumber">
              <ElInput v-model="form.phonenumber" placeholder="请输入手机号码" maxlength="11" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="邮箱" prop="email">
              <ElInput v-model="form.email" placeholder="请输入邮箱" maxlength="50" />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow>
          <ElCol :span="12">
            <ElFormItem v-if="!form.userId" label="用户名称" prop="userName">
              <ElInput v-model="form.userName" placeholder="请输入用户名称" maxlength="30" />
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem v-if="!form.userId" label="用户密码" prop="password">
              <ElInput v-model="form.password" placeholder="请输入用户密码" type="password" maxlength="20" show-password />
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="用户性别">
              <ElSelect v-model="form.sex" placeholder="请选择性别">
                <ElOption v-for="d in dictMap.sex" :key="d.dictValue" :label="d.dictLabel" :value="d.dictValue" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="状态">
              <ElRadioGroup v-model="form.status">
                <ElRadio v-for="d in dictMap.status" :key="d.dictValue" :value="d.dictValue">{{ d.dictLabel }}</ElRadio>
              </ElRadioGroup>
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow>
          <ElCol :span="12">
            <ElFormItem label="岗位">
              <ElSelect v-model="form.postIds" multiple placeholder="请选择岗位">
                <ElOption v-for="p in postOptions" :key="p.postId" :label="p.postName" :value="p.postId" :disabled="p.status === '1'" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
          <ElCol :span="12">
            <ElFormItem label="角色">
              <ElSelect v-model="form.roleIds" multiple placeholder="请选择角色">
                <ElOption v-for="r in roleOptions" :key="r.roleId" :label="r.roleName" :value="r.roleId" :disabled="r.status === '1'" />
              </ElSelect>
            </ElFormItem>
          </ElCol>
        </ElRow>
        <ElRow>
          <ElCol :span="24">
            <ElFormItem label="备注">
              <ElInput v-model="form.remark" type="textarea" placeholder="请输入内容" />
            </ElFormItem>
          </ElCol>
        </ElRow>
      </ElForm>
      <template #footer>
        <ElButton type="primary" @click="submitForm">确 定</ElButton>
        <ElButton @click="cancel">取 消</ElButton>
      </template>
    </el-dialog>

    <!-- 导入对话框 -->
    <el-dialog v-model="importOpen" title="用户导入" width="420px" append-to-body @close="handleImportClose">
      <el-upload
        ref="importFileRef"
        :limit="1"
        accept=".xlsx, .xls"
        :auto-upload="true"
        :show-file-list="true"
        :http-request="handleImportSubmit"
        drag
      >
        <el-icon class="el-icon--upload"><Upload /></el-icon>
        <div class="el-upload__text">
          将文件拖到此处，或<em>点击上传</em>
        </div>
        <template #tip>
          <div class="el-upload__tip text-center">
            <span>仅允许导入 xls、xlsx 格式文件。</span>
            <el-link type="primary" :underline="false" style="font-size: 12px; vertical-align: baseline" @click="handleImportTemplate">下载模板</el-link>
          </div>
        </template>
      </el-upload>

      <div style="text-align: center" class="mt-2">
        <el-checkbox v-model="importUpdateSupport">
          是否更新已经存在的用户数据
        </el-checkbox>
      </div>
    </el-dialog>
  </div>
</template>

<style scoped>
.ruoyi-page {
  display: flex;
  gap: 12px;
  padding: 12px;
  height: 100%;
  /* el-dialog 即使关闭也会作为子元素留在 flex 流里，会挤占内容区宽度，
     这里强制它脱离文档流，避免破坏 flex 布局 */
}
/* 将 el-dialog 移出 flex 布局流（关闭态占位元素不应占空间） */
.ruoyi-page :deep(el-dialog) {
  position: absolute;
  width: 0;
  height: 0;
  overflow: hidden;
  visibility: hidden;
}
.dept-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
}
.dept-sidebar__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 4px 8px;
  font-weight: 600;
  border-bottom: 1px solid var(--el-border-color-lighter);
  margin-bottom: 8px;
}
.dept-filter {
  margin-bottom: 8px;
}
.dept-tree {
  flex: 1;
  overflow: auto;
}
.ruoyi-content {
  flex: 1 1 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.search-form {
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  padding: 16px 16px 0;
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.table-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
}
.pagination {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0 4px;
}
</style>
