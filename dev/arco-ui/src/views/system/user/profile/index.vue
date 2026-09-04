<template>
  <div class="profile-page">
    <a-card :bordered="false" class="app-page-card">
      <a-tabs default-active-key="userinfo">
        <!-- 基本资料 -->
        <a-tab-pane key="userinfo" :title="t('system.profile.tabInfo')">
          <a-row :gutter="24">
            <a-col :xs="24" :md="7" class="profile-page__avatar-col">
              <div class="profile-page__avatar">
                <a-avatar :size="104">
                  <img v-if="userStore.avatarUrl" :src="userStore.avatarUrl" :alt="t('system.profile.avatarAlt')" />
                  <IconUser v-else />
                </a-avatar>
                <a-upload
                  :auto-upload="false"
                  :show-file-list="false"
                  accept="image/*"
                  @change="onAvatarChange"
                >
                  <template #upload-button>
                    <a-button type="outline" size="small" :loading="avatarUploading">
                      {{ t('system.profile.changeAvatar') }}
                    </a-button>
                  </template>
                </a-upload>
                <span class="profile-page__avatar-tip">{{ t('system.profile.avatarTip') }}</span>
              </div>
            </a-col>
            <a-col :xs="24" :md="17">
              <a-descriptions :column="{ xs: 1, md: 2 }" bordered size="medium" class="profile-page__desc">
                <a-descriptions-item :label="t('system.user.userName')">{{ profileUser?.userName ?? '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('common.fields.phonenumber')">{{ profileUser?.phonenumber || '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('common.fields.email')">{{ profileUser?.email || '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('system.profile.deptLabel')">{{ profileUser?.dept?.deptName ?? '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('system.profile.roleGroupLabel')">{{ roleGroup || '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('system.profile.postGroupLabel')">{{ postGroup || '-' }}</a-descriptions-item>
                <a-descriptions-item :label="t('system.profile.createdDate')">{{ profileUser?.createTime ?? '-' }}</a-descriptions-item>
              </a-descriptions>

              <a-divider orientation="left">{{ t('system.profile.editProfile') }}</a-divider>
              <a-form
                ref="infoFormRef"
                :model="infoForm"
                :rules="infoRules"
                auto-label-width
                class="profile-page__form"
              >
                <a-row :gutter="12">
                  <a-col :span="12">
                    <a-form-item field="nickName" :label="t('system.user.nickName')">
                      <a-input
                        v-model.trim="infoForm.nickName"
                        :placeholder="t('common.pleaseEnter', { field: t('system.user.nickName') })"
                        allow-clear
                      />
                    </a-form-item>
                  </a-col>
                  <a-col :span="12">
                    <a-form-item field="phonenumber" :label="t('common.fields.phonenumber')">
                      <a-input
                        v-model.trim="infoForm.phonenumber"
                        :placeholder="t('common.pleaseEnter', { field: t('common.fields.phonenumber') })"
                        allow-clear
                      />
                    </a-form-item>
                  </a-col>
                  <a-col :span="12">
                    <a-form-item field="email" :label="t('common.fields.email')">
                      <a-input
                        v-model.trim="infoForm.email"
                        :placeholder="t('common.pleaseEnter', { field: t('common.fields.email') })"
                        allow-clear
                      />
                    </a-form-item>
                  </a-col>
                  <a-col :span="12">
                    <a-form-item field="sex" :label="t('system.profile.sex')">
                      <a-radio-group v-model="infoForm.sex">
                        <a-radio v-for="item in sysUserSex" :key="item.dictValue" :value="item.dictValue">
                          {{ item.dictLabel }}
                        </a-radio>
                      </a-radio-group>
                    </a-form-item>
                  </a-col>
                </a-row>
                <a-form-item>
                  <a-button type="primary" :loading="infoSaving" @click="submitInfo">{{ t('common.save') }}</a-button>
                </a-form-item>
              </a-form>
            </a-col>
          </a-row>
        </a-tab-pane>

        <!-- 修改密码 -->
        <a-tab-pane key="password" :title="t('system.profile.tabPassword')">
          <a-form
            ref="pwdFormRef"
            :model="pwdForm"
            :rules="pwdRules"
            auto-label-width
            class="profile-page__form profile-page__pwd-form"
          >
            <a-form-item field="oldPassword" :label="t('system.profile.oldPassword')">
              <a-input-password
                v-model="pwdForm.oldPassword"
                :placeholder="t('common.pleaseEnter', { field: t('system.profile.oldPassword') })"
                allow-clear
              />
            </a-form-item>
            <a-form-item field="newPassword" :label="t('system.profile.newPassword')">
              <a-input-password
                v-model="pwdForm.newPassword"
                :placeholder="t('common.newPwdPlaceholder')"
                allow-clear
              />
            </a-form-item>
            <a-form-item field="confirmPassword" :label="t('system.profile.confirmPassword')">
              <a-input-password
                v-model="pwdForm.confirmPassword"
                :placeholder="t('system.profile.confirmPwdPlaceholder')"
                allow-clear
              />
            </a-form-item>
            <a-form-item>
              <a-button type="primary" :loading="pwdSaving" @click="submitPwd">{{ t('system.profile.saveChanges') }}</a-button>
            </a-form-item>
          </a-form>
        </a-tab-pane>
      </a-tabs>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import type { FieldRule, FileItem, FormInstance } from '@arco-design/web-vue'
import { IconUser } from '@arco-design/web-vue/es/icon'
import { Message, Modal } from '@arco-design/web-vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getProfile, updateProfile, updateUserPwd } from '@/api/system/user'
import { uploadAvatar } from '@/api/common'
import type { SysUser } from '@/api/types'
import { useDict } from '@/hooks/useDict'
import { usePermissionStore } from '@/stores/permission'
import { useUserStore } from '@/stores/user'

// 与动态注入的隐藏路由 name 一致（见 stores/permission.ts）
defineOptions({ name: 'Profile' })

const router = useRouter()
const { t } = useI18n()
const userStore = useUserStore()
const permissionStore = usePermissionStore()
const sysUserSex = useDict('sys_user_sex')['sys_user_sex']

const profileUser = ref<SysUser>()
const roleGroup = ref('')
const postGroup = ref('')

/* ---------- 基本资料 ---------- */
const infoFormRef = ref<FormInstance>()
const infoSaving = ref(false)
const infoForm = reactive({
  nickName: '',
  phonenumber: '',
  email: '',
  sex: ''
})

/** 校验规则（computed：随语言切换联动提示语） */
const infoRules = computed<Record<string, FieldRule[]>>(() => ({
  nickName: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.user.nickName') }) },
    { maxLength: 30, message: t('common.maxLengthTip', { max: 30 }) }
  ],
  phonenumber: [{ match: /^1[3-9]\d{9}$/, message: t('common.phoneInvalid') }],
  email: [{ match: /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/, message: t('common.emailInvalid') }]
}))

async function loadProfile(): Promise<void> {
  const res = await getProfile()
  profileUser.value = res.data
  roleGroup.value = res.roleGroup ?? ''
  postGroup.value = res.postGroup ?? ''
  infoForm.nickName = res.data?.nickName ?? ''
  infoForm.phonenumber = res.data?.phonenumber ?? ''
  infoForm.email = res.data?.email ?? ''
  infoForm.sex = res.data?.sex ?? '0'
}

async function submitInfo(): Promise<void> {
  try {
    await infoFormRef.value?.validate()
  } catch {
    return
  }
  infoSaving.value = true
  try {
    await updateProfile({
      nickName: infoForm.nickName,
      phonenumber: infoForm.phonenumber,
      email: infoForm.email,
      sex: infoForm.sex
    })
    Message.success(t('common.updateSuccess'))
    await loadProfile()
    // 同步顶栏显示的昵称等信息
    await userStore.getInfo()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    infoSaving.value = false
  }
}

/* ---------- 头像 ---------- */
const avatarUploading = ref(false)

/** 选择头像即上传（成功后刷新登录态头像） */
async function onAvatarChange(fileList: FileItem[]): Promise<void> {
  const file = fileList[fileList.length - 1]?.file
  if (!file) return
  if (file.size > 2 * 1024 * 1024) {
    Message.error(t('system.profile.avatarTooLarge'))
    return
  }
  avatarUploading.value = true
  try {
    await uploadAvatar(file)
    Message.success(t('system.profile.avatarUpdated'))
    await userStore.getInfo()
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    avatarUploading.value = false
  }
}

/* ---------- 修改密码 ---------- */
const pwdFormRef = ref<FormInstance>()
const pwdSaving = ref(false)
const pwdForm = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: ''
})

/** 校验规则（computed：随语言切换联动提示语） */
const pwdRules = computed<Record<string, FieldRule[]>>(() => ({
  oldPassword: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.profile.oldPassword') }) },
    { minLength: 5, maxLength: 20, message: t('common.passwordLengthTip') }
  ],
  newPassword: [
    { required: true, message: t('common.pleaseEnter', { field: t('system.profile.newPassword') }) },
    { minLength: 5, maxLength: 20, message: t('common.passwordLengthTip') }
  ],
  confirmPassword: [
    { required: true, message: t('system.profile.confirmPwdPlaceholder') },
    {
      validator: (value, callback) => {
        if (value !== pwdForm.newPassword) {
          callback(t('common.passwordMismatch'))
        } else {
          callback()
        }
      }
    }
  ]
}))

async function submitPwd(): Promise<void> {
  try {
    await pwdFormRef.value?.validate()
  } catch {
    return
  }
  pwdSaving.value = true
  try {
    await updateUserPwd(pwdForm.oldPassword, pwdForm.newPassword)
    Message.success(t('system.profile.pwdChanged'))
    pwdForm.oldPassword = ''
    pwdForm.newPassword = ''
    pwdForm.confirmPassword = ''
    Modal.confirm({
      title: t('common.notice'),
      content: t('system.profile.reloginConfirm'),
      hideCancel: false,
      onOk: async () => {
        try {
          await userStore.logout()
        } finally {
          permissionStore.reset()
          router.push('/login')
        }
      }
    })
  } catch {
    // 失败提示已由响应拦截器统一弹出
  } finally {
    pwdSaving.value = false
  }
}

/* ---------- 初始化 ---------- */
onMounted(() => {
  void loadProfile()
})
</script>

<style scoped>
.profile-page__avatar-col {
  display: flex;
  justify-content: center;
  margin-bottom: 12px;
}

.profile-page__avatar {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.profile-page__avatar-tip {
  font-size: 12px;
  color: var(--color-text-3);
}

.profile-page__desc {
  margin-bottom: 8px;
}

.profile-page__form {
  max-width: 720px;
  margin-top: 8px;
}

.profile-page__pwd-form {
  max-width: 480px;
}
</style>
