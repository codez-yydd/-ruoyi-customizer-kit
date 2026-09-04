<template>
  <AuthLayout>
    <div class="register-form">
      <h1 class="register-form__title">{{ t('register.title') }}</h1>
      <p class="register-form__subtitle">{{ t('register.subtitle') }}</p>

      <a-form
        ref="formRef"
        :model="form"
        :rules="rules"
        layout="vertical"
        size="large"
        @submit-success="handleRegister"
      >
        <a-form-item field="username" hide-asterisk>
          <a-input v-model.trim="form.username" :placeholder="t('register.usernamePlaceholder')" allow-clear>
            <template #prefix><IconUser /></template>
          </a-input>
        </a-form-item>

        <a-form-item field="password" hide-asterisk>
          <a-input-password v-model="form.password" :placeholder="t('register.passwordPlaceholder')">
            <template #prefix><IconLock /></template>
          </a-input-password>
        </a-form-item>

        <a-form-item field="confirmPassword" hide-asterisk>
          <a-input-password v-model="form.confirmPassword" :placeholder="t('register.confirmPlaceholder')">
            <template #prefix><IconLock /></template>
          </a-input-password>
        </a-form-item>

        <a-form-item v-if="captchaEnabled" field="code" hide-asterisk>
          <div class="register-form__captcha">
            <a-input v-model.trim="form.code" :placeholder="t('register.captchaPlaceholder')" allow-clear>
              <template #prefix><IconSafe /></template>
            </a-input>
            <img
              class="register-form__captcha-img"
              :src="captchaImg"
              :alt="t('register.captchaTip')"
              :title="t('register.captchaTip')"
              @click="loadCaptcha"
            />
          </div>
        </a-form-item>

        <a-form-item>
          <a-button
            type="primary"
            html-type="submit"
            long
            :loading="loading"
            class="register-form__submit"
          >
            {{ loading ? t('register.registering') : t('register.submit') }}
          </a-button>
        </a-form-item>
      </a-form>

      <div class="register-form__login">
        {{ t('register.alreadyHaveAccount') }}
        <router-link to="/login" class="register-form__login-link">{{ t('register.backToLogin') }}</router-link>
      </div>
    </div>
  </AuthLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { IconLock, IconSafe, IconUser } from '@arco-design/web-vue/es/icon'
import { Message } from '@arco-design/web-vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import AuthLayout from '@/components/AuthLayout/index.vue'
import { getCaptchaImage, register } from '@/api/login'
import type { RegisterFormData } from '@/api/types'

/**
 * 注册页：
 * - 布局复用 AuthLayout；验证码复用 /captchaImage
 * - 后端 sys.account.registerUser=false 时提交返回错误 msg（拦截器统一提示），前端仅刷新验证码
 * - 注册成功 -> 提示 -> 跳 /login 并携带 username 预填
 */
const { t } = useI18n()
const router = useRouter()

/** a-form 实例所需的最小类型面（与登录页一致） */
interface FormExpose {
  validate: () => Promise<Record<string, string> | undefined>
  validateField: (field?: string | string[]) => Promise<Record<string, string> | undefined>
  resetFields: () => void
  clearValidate: (fields?: string[]) => void
}

const formRef = ref<FormExpose>()
const loading = ref(false)

const form = reactive<RegisterFormData>({
  username: '',
  password: '',
  confirmPassword: '',
  code: '',
  uuid: ''
})

/** 确认密码一致性校验（若依后端约定 5-20 位） */
const validateConfirmPassword = (
  value: string | undefined,
  callback: (error?: string) => void
): void => {
  if (value !== form.password) {
    callback(t('register.mismatch'))
    return
  }
  callback()
}

/** 表单校验规则（computed 保持语言切换后提示语联动） */
const rules = computed(() => ({
  username: [
    { required: true, message: t('register.usernamePlaceholder') },
    { minLength: 2, maxLength: 20, message: t('register.usernameLength') }
  ],
  password: [
    { required: true, message: t('register.passwordPlaceholder') },
    { minLength: 5, maxLength: 20, message: t('register.passwordLength') }
  ],
  confirmPassword: [
    { required: true, message: t('register.confirmPlaceholder') },
    { validator: validateConfirmPassword }
  ],
  code: [{ required: true, message: t('register.captchaPlaceholder') }]
}))

/** 密码变更时同步重校验已填写的确认密码，避免「两次输入不一致」提示滞留 */
watch(
  () => form.password,
  () => {
    if (form.confirmPassword) {
      formRef.value?.validateField('confirmPassword').catch(() => {
        /* 校验失败仅展示字段错误，无需额外处理 */
      })
    }
  }
)

/** 验证码（后端关闭验证码时隐藏输入） */
const captchaEnabled = ref(false)
const captchaImg = ref('')

async function loadCaptcha(): Promise<void> {
  try {
    const result = await getCaptchaImage()
    captchaEnabled.value = result.captchaEnabled !== false
    if (captchaEnabled.value && result.img) {
      captchaImg.value = `data:image/jpeg;base64,${result.img}`
      form.uuid = result.uuid ?? ''
    } else {
      captchaImg.value = ''
      form.uuid = ''
    }
  } catch {
    captchaEnabled.value = false
  }
}

async function submitRegister(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }
  if (loading.value) return
  loading.value = true
  try {
    const result = await register({
      username: form.username,
      password: form.password,
      confirmPassword: form.confirmPassword,
      code: form.code || undefined,
      uuid: form.uuid || undefined
    })
    Message.success(result.msg || t('register.success'))
    // 跳登录页并预填用户名
    router.push({ path: '/login', query: { username: form.username } })
  } catch {
    // 失败（含注册未开启）：拦截器已提示；校验码一次性有效，刷新
    if (captchaEnabled.value) {
      form.code = ''
      await loadCaptcha()
    }
  } finally {
    loading.value = false
  }
}

function handleRegister(): void {
  void submitRegister()
}

onMounted(() => {
  void loadCaptcha()
})
</script>

<style scoped>
.register-form__title {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-1);
}

.register-form__subtitle {
  margin: 0 0 24px;
  font-size: 14px;
  color: var(--color-text-3);
}

.register-form__captcha {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.register-form__captcha :deep(.arco-input-wrapper) {
  flex: 1;
}

.register-form__captcha-img {
  width: 110px;
  height: 40px;
  border-radius: 4px;
  cursor: pointer;
  border: 1px solid var(--color-border-2);
  flex-shrink: 0;
  object-fit: cover;
}

.register-form__submit {
  height: 40px;
  font-size: 15px;
  border-radius: 6px;
  margin-top: 4px;
}

.register-form__login {
  text-align: center;
  font-size: 13px;
  color: var(--color-text-3);
}

.register-form__login-link {
  color: rgb(var(--primary-6));
  text-decoration: none;
}

.register-form__login-link:hover {
  text-decoration: underline;
}
</style>
