<script lang="ts" setup>
import { onMounted, reactive, ref } from 'vue';
import { useRoute } from 'vue-router';

import { IconifyIcon } from '@vben/icons';
import { useUserStore } from '@vben/stores';

import {
  ElCard,
  ElCol,
  ElMessage,
  ElRow,
  ElTabPane,
  ElTabs,
} from 'element-plus';

import {
  getProfileApi,
  type ProfileUser,
} from '#/api/system/profile';
import { parseTime } from '#/utils/ruoyi';

import UserAvatar from './userAvatar.vue';
import UserInfo from './userInfo.vue';
import ResetPwd from './resetPwd.vue';

defineOptions({ name: 'Profile' });

const route = useRoute();
const userStore = useUserStore();

const state = reactive({
  user: {} as ProfileUser,
  roleGroup: '',
  postGroup: '',
});

const activeTab = ref('userinfo');
const loading = ref(false);

/** 将若依相对头像路径拼成可访问 URL */
function resolveAvatarUrl(path?: string) {
  const avatarPath = path ?? '';
  if (!avatarPath) return '';
  return /^https?:\/\//i.test(avatarPath)
    ? avatarPath
    : `${import.meta.env.VITE_GLOB_API_URL}${avatarPath}`;
}

async function loadProfile() {
  loading.value = true;
  try {
    const res = await getProfileApi();
    state.user = res.data ?? ({} as ProfileUser);
    state.roleGroup = res.roleGroup ?? '';
    state.postGroup = res.postGroup ?? '';
    // 同步顶栏昵称/头像，保证个人中心与布局头像一致
    if (userStore.userInfo) {
      if (state.user.nickName) {
        userStore.userInfo.realName = state.user.nickName;
      }
      if (state.user.avatar) {
        userStore.userInfo.avatar = resolveAvatarUrl(state.user.avatar);
      }
    }
  } catch {
    ElMessage.error('获取个人信息失败');
  } finally {
    loading.value = false;
  }
}

/** 头像裁剪上传成功后，同步左侧展示的相对路径 */
function onAvatarSuccess(imgUrl: string) {
  state.user.avatar = imgUrl;
}

onMounted(() => {
  // 支持路由参数或 query 指定默认 Tab（如从「强制改密」入口跳入）
  const tabFromRoute =
    (route.params.activeTab as string) ||
    (route.query.tab as string) ||
    '';
  if (tabFromRoute === 'resetPwd' || tabFromRoute === 'userinfo') {
    activeTab.value = tabFromRoute;
  }
  void loadProfile();
});
</script>

<template>
  <div class="p-4" v-loading="loading">
    <ElRow :gutter="20">
      <!-- 左侧：个人信息卡片 -->
      <ElCol :span="6" :xs="24">
        <ElCard shadow="never">
          <template #header>
            <div class="card-header">个人信息</div>
          </template>
          <div>
            <div class="text-center">
              <UserAvatar
                :avatar="state.user.avatar"
                @success="onAvatarSuccess"
              />
            </div>
            <ul class="info-list">
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:user" class="info-icon" />
                  用户名称
                </span>
                <div class="info-value">{{ state.user.userName }}</div>
              </li>
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:phone" class="info-icon" />
                  手机号码
                </span>
                <div class="info-value">{{ state.user.phonenumber }}</div>
              </li>
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:mail" class="info-icon" />
                  用户邮箱
                </span>
                <div class="info-value">{{ state.user.email }}</div>
              </li>
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:network" class="info-icon" />
                  所属部门
                </span>
                <div class="info-value">
                  <template v-if="state.user.dept">
                    {{ state.user.dept.deptName }} / {{ state.postGroup }}
                  </template>
                </div>
              </li>
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:users" class="info-icon" />
                  所属角色
                </span>
                <div class="info-value">{{ state.roleGroup }}</div>
              </li>
              <li>
                <span class="info-label">
                  <IconifyIcon icon="lucide:calendar" class="info-icon" />
                  创建日期
                </span>
                <div class="info-value">
                  {{ parseTime(state.user.createTime) }}
                </div>
              </li>
            </ul>
          </div>
        </ElCard>
      </ElCol>

      <!-- 右侧：基本资料 / 修改密码 -->
      <ElCol :span="18" :xs="24">
        <ElCard shadow="never">
          <template #header>
            <div class="card-header">基本资料</div>
          </template>
          <ElTabs v-model="activeTab">
            <ElTabPane label="基本资料" name="userinfo">
              <UserInfo :user="state.user" @updated="loadProfile" />
            </ElTabPane>
            <ElTabPane label="修改密码" name="resetPwd">
              <ResetPwd />
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

.text-center {
  text-align: center;
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
  gap: 12px;
  padding: 11px 0;
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 14px;
}

.info-list li:last-child {
  border-bottom: none;
}

.info-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.info-icon {
  width: 14px;
  height: 14px;
}

.info-value {
  text-align: right;
  word-break: break-all;
  color: var(--el-text-color-primary);
}
</style>
