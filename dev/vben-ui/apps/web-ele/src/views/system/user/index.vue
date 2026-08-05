<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
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
  getUser,
  listUser,
  resetUserPwd,
  updateUser,
  type SysUser,
} from '#/api/system/user';
import { useDict } from '#/composables/useDict';
import { usePagination } from '#/composables/usePagination';
import { addDateRange, parseTime } from '#/utils/ruoyi';

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
function handleAuthRole(row: SysUser) {
  router.push(`/system/user-auth/role/${row.userId}`);
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
        <ElButton type="info" plain :icon="Upload" v-hasPermi="['system:user:import']">导入</ElButton>
        <ElButton type="warning" plain :icon="Download" v-hasPermi="['system:user:export']">导出</ElButton>
      </div>

      <!-- 表格 -->
      <ElTable v-loading="loading" :data="userList" border @selection-change="handleSelectionChange">
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
              <el-tree-select v-model="form.deptId" :data="deptOptions" :props="{ label: 'label', children: 'children' }" check-strictly placeholder="请选择归属部门" style="width: 100%" />
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
  </div>
</template>

<style scoped>
.ruoyi-page {
  display: flex;
  gap: 12px;
  padding: 12px;
  height: 100%;
}
.dept-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--el-bg-color);
  border-radius: 4px;
  padding: 8px;
  display: flex;
  flex-direction: column;
}
.dept-sidebar__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 8px;
  font-weight: 600;
}
.dept-filter {
  margin: 8px 0;
}
.dept-tree {
  flex: 1;
  overflow: auto;
}
.ruoyi-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.search-form {
  background: var(--el-bg-color);
  padding: 12px 12px 0;
  border-radius: 4px;
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.pagination {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0;
}
</style>
