<script lang="ts" setup>
import { computed, onMounted, reactive, ref, watch } from 'vue';

import { useUserStore } from '@vben/stores';

import {
  ElButton,
  ElCard,
  ElCol,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElOption,
  ElRow,
  ElSelect,
  ElTabPane,
  ElTabs,
  ElUpload,
  type FormInstance,
  type FormRules,
} from 'element-plus';

import { useDict } from '#/composables/useDict';
import {
  getProfileApi,
  updateProfileApi,
  uploadAvatarApi,
  updateUserPwdApi,
  type ProfileUser,
} from '#/api/system/profile';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'Profile' });

const userStore = useUserStore();
const { dictMap } = useDict({ sex: 'sys_user_sex' });

// ===== 个人中心详情数据 =====
const user = ref<ProfileUser>({} as ProfileUser);
const roleGroup = ref('');
const postGroup = ref('');
const activeTab = ref('userinfo');

// 头像 URL（拼 API 前缀，处理若依相对路径）
const avatarUrl = computed(() => {
  const a = user.value.avatar ?? userStore.userInfo?.avatar ?? '';
  if (!a) return '';
  return /^https?:\/\//i.test(a)
    ? a
    : `${import.meta.env.VITE_GLOB_API_URL}${a}`;
});

async function loadProfile() {
  try {
    const res = await getProfileApi();
    user.value = res.data;
    roleGroup.value = res.roleGroup;
    postGroup.value = res.postGroup;
    // 同步 userStore 里的头像/昵称（个人中心与顶栏保持一致）
    if (res.data.avatar) userStore.userInfo!.avatar = avatarUrl.value;
    if (res.data.nickName) userStore.userInfo!.realName = res.data.nickName;
  } catch {
    ElMessage.error('获取个人信息失败');
  }
}

onMounted(loadProfile);

// ===== 头像上传 =====
function handleAvatarSuccess(response: any) {
  // 若依返回 {code,msg,imgUrl}，requestClient 拦截器已解包到 imgUrl
  const imgUrl = response?.imgUrl ?? response;
  if (imgUrl) {
    user.value.avatar = imgUrl;
    userStore.userInfo!.avatar = `${import.meta.env.VITE_GLOB_API_URL}${imgUrl}`;
    ElMessage.success('头像更新成功');
  }
}

function beforeAvatarUpload(file: File) {
  const isImg = /^image\//.test(file.type);
  const isLt2M = file.size / 1024 / 1024 < 2;
  if (!isImg) {
    ElMessage.error('只能上传图片文件');
    return false;
  }
  if (!isLt2M) {
    ElMessage.error('头像图片大小不能超过 2MB');
    return false;
  }
  return true;
}

/** 自定义上传：调若依 /system/user/profile/avatar（字段名 avatarfile） */
async function handleAvatarUpload(opt: { file: File }) {
  try {
    const result = await uploadAvatarApi(opt.file);
    handleAvatarSuccess(result);
  } catch {
    ElMessage.error('上传失败');
  }
}

// ===== 基本资料表单 =====
const userinfoFormRef = ref<FormInstance>();
const userinfoForm = reactive({
  nickName: '',
  phonenumber: '',
  email: '',
  sex: '',
});

function syncUserinfoForm() {
  userinfoForm.nickName = user.value.nickName ?? '';
  userinfoForm.phonenumber = user.value.phonenumber ?? '';
  userinfoForm.email = user.value.email ?? '';
  userinfoForm.sex = user.value.sex ?? '';
}

async function submitUserinfo() {
  await userinfoFormRef.value?.validate();
  await updateProfileApi({ ...userinfoForm });
  ElMessage.success('修改成功');
  // 同步顶栏昵称
  userStore.userInfo!.realName = userinfoForm.nickName;
  await loadProfile();
}

// ===== 修改密码表单 =====
const pwdFormRef = ref<FormInstance>();
const pwdForm = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: '',
});

const pwdRules: FormRules = {
  oldPassword: [{ message: '旧密码不能为空', required: true, trigger: 'blur' }],
  newPassword: [
    { message: '新密码不能为空', required: true, trigger: 'blur' },
    { min: 6, max: 20, message: '长度在 6 到 20 个字符', trigger: 'blur' },
  ],
  confirmPassword: [
    { message: '确认密码不能为空', required: true, trigger: 'blur' },
    {
      validator: (_rule, value, cb) => {
        if (value !== pwdForm.newPassword) {
          cb(new Error('两次输入的密码不一致'));
        } else {
          cb();
        }
      },
      trigger: 'blur',
    },
  ],
};

async function submitPwd() {
  await pwdFormRef.value?.validate();
  await updateUserPwdApi(pwdForm.oldPassword, pwdForm.newPassword);
  ElMessage.success('密码修改成功，请重新登录');
  pwdForm.oldPassword = '';
  pwdForm.newPassword = '';
  pwdForm.confirmPassword = '';
}

// 当 user 数据加载后同步基本资料表单
watch(() => user.value.userId, syncUserinfoForm, { immediate: true });
</script>

<template>
  <div class="p-4">
    <ElRow :gutter="20">
      <!-- 左侧：个人信息卡片 -->
      <ElCol :span="8" :xs="24">
        <ElCard>
          <template #header>
            <div class="card-header">个人信息</div>
          </template>
          <div>
            <div class="text-center">
              <ElUpload
                :show-file-list="false"
                :before-upload="beforeAvatarUpload"
                :http-request="handleAvatarUpload"
                action="#"
              >
                <img
                  v-if="avatarUrl"
                  :src="avatarUrl"
                  alt="头像"
                  class="user-avatar"
                  title="点击更换头像"
                />
                <div v-else class="user-avatar user-avatar-placeholder">
                  {{ user.userName?.charAt(0)?.toUpperCase() }}
                </div>
              </ElUpload>
            </div>
            <ul class="info-list">
              <li>
                <span>用户名称</span>
                <div class="info-value">{{ user.userName }}</div>
              </li>
              <li>
                <span>手机号码</span>
                <div class="info-value">{{ user.phonenumber }}</div>
              </li>
              <li>
                <span>用户邮箱</span>
                <div class="info-value">{{ user.email }}</div>
              </li>
              <li>
                <span>所属部门</span>
                <div class="info-value">
                  {{ user.dept?.deptName }} / {{ postGroup }}
                </div>
              </li>
              <li>
                <span>所属角色</span>
                <div class="info-value">{{ roleGroup }}</div>
              </li>
              <li>
                <span>创建日期</span>
                <div class="info-value">
                  {{ parseTime(user.createTime) }}
                </div>
              </li>
            </ul>
          </div>
        </ElCard>
      </ElCol>

      <!-- 右侧：基本资料 / 修改密码 -->
      <ElCol :span="16" :xs="24">
        <ElCard>
          <template #header>
            <div class="card-header">基本资料</div>
          </template>
          <ElTabs v-model="activeTab">
            <ElTabPane label="基本资料" name="userinfo">
              <ElForm
                ref="userinfoFormRef"
                :model="userinfoForm"
                label-width="90px"
                style="max-width: 500px"
              >
                <ElFormItem label="用户昵称" prop="nickName">
                  <ElInput v-model="userinfoForm.nickName" />
                </ElFormItem>
                <ElFormItem label="手机号码" prop="phonenumber">
                  <ElInput v-model="userinfoForm.phonenumber" />
                </ElFormItem>
                <ElFormItem label="邮箱" prop="email">
                  <ElInput v-model="userinfoForm.email" />
                </ElFormItem>
                <ElFormItem label="性别" prop="sex">
                  <ElSelect v-model="userinfoForm.sex" placeholder="请选择">
                    <ElOption
                      v-for="d in dictMap.sex"
                      :key="d.dictValue"
                      :label="d.dictLabel"
                      :value="d.dictValue"
                    />
                  </ElSelect>
                </ElFormItem>
                <ElFormItem>
                  <ElButton type="primary" @click="submitUserinfo">
                    保存
                  </ElButton>
                  <ElButton @click="syncUserinfoForm">重置</ElButton>
                </ElFormItem>
              </ElForm>
            </ElTabPane>

            <ElTabPane label="修改密码" name="resetPwd">
              <ElForm
                ref="pwdFormRef"
                :model="pwdForm"
                :rules="pwdRules"
                label-width="100px"
                style="max-width: 500px"
              >
                <ElFormItem label="旧密码" prop="oldPassword">
                  <ElInput
                    v-model="pwdForm.oldPassword"
                    type="password"
                    placeholder="请输入旧密码"
                    show-password
                  />
                </ElFormItem>
                <ElFormItem label="新密码" prop="newPassword">
                  <ElInput
                    v-model="pwdForm.newPassword"
                    type="password"
                    placeholder="请输入新密码"
                    show-password
                  />
                </ElFormItem>
                <ElFormItem label="确认密码" prop="confirmPassword">
                  <ElInput
                    v-model="pwdForm.confirmPassword"
                    type="password"
                    placeholder="请确认新密码"
                    show-password
                  />
                </ElFormItem>
                <ElFormItem>
                  <ElButton type="primary" @click="submitPwd">保存</ElButton>
                  <ElButton
                    @click="
                      () => {
                        pwdFormRef?.resetFields();
                      }
                    "
                  >
                    重置
                  </ElButton>
                </ElFormItem>
              </ElForm>
            </ElTabPane>
          </ElTabs>
        </ElCard>
      </ElCol>
    </ElRow>
  </div>
</template>

<style scoped>
.card-header {
  font-weight: 600;
}

.user-avatar {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  object-fit: cover;
  cursor: pointer;
  border: 1px solid var(--el-border-color);
}

.user-avatar-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 48px;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}

.info-list {
  list-style: none;
  margin: 24px 0 0;
  padding: 0;
}

.info-list li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 11px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 14px;
}

.info-list li:last-child {
  border-bottom: none;
}

.info-list span {
  color: var(--el-text-color-secondary);
}

.info-value {
  text-align: right;
}
</style>
