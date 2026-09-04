<template>
  <AuthLayout>
    <div class="login-form">
      <h1 class="login-form__title">{{ t('login.welcomeBack') }}</h1>
      <p class="login-form__subtitle">{{ t('login.subtitle') }}</p>

      <a-form
        ref="formRef"
        :model="form"
        :rules="rules"
        layout="vertical"
        size="large"
        @submit-success="handleLogin"
      >
        <a-form-item field="username" hide-asterisk>
          <a-input v-model.trim="form.username" :placeholder="t('login.usernamePlaceholder')" allow-clear>
            <template #prefix><IconUser /></template>
          </a-input>
        </a-form-item>

        <a-form-item field="password" hide-asterisk>
          <a-input-password v-model="form.password" :placeholder="t('login.passwordPlaceholder')">
            <template #prefix><IconLock /></template>
          </a-input-password>
        </a-form-item>

        <a-form-item v-if="captchaEnabled" field="code" hide-asterisk>
          <div class="login-form__captcha">
            <a-input v-model.trim="form.code" :placeholder="t('login.captchaPlaceholder')" allow-clear>
              <template #prefix><IconSafe /></template>
            </a-input>
            <img
              class="login-form__captcha-img"
              :src="captchaImg"
              :alt="t('login.captchaTip')"
              :title="t('login.captchaTip')"
              @click="loadCaptcha"
            />
          </div>
        </a-form-item>

        <div class="login-form__options">
          <a-checkbox v-model="rememberMe">{{ t('login.rememberMe') }}</a-checkbox>
        </div>

        <a-form-item>
          <a-button
            type="primary"
            html-type="submit"
            long
            :loading="loading"
            class="login-form__submit"
          >
            {{ loading ? t('login.loggingIn') : t('login.submit') }}
          </a-button>
        </a-form-item>
      </a-form>

      <div v-if="registerEnabled" class="login-form__register">
        {{ t('login.noAccount') }}
        <router-link to="/register" class="login-form__register-link">{{ t('login.registerNow') }}</router-link>
      </div>
    </div>
  </AuthLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { IconLock, IconSafe, IconUser } from '@arco-design/web-vue/es/icon'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import AuthLayout from '@/components/AuthLayout/index.vue'
import { getCaptchaImage } from '@/api/login'
import type { LoginFormData } from '@/api/types'
import { useUserStore } from '@/stores/user'
import { usePermissionStore } from '@/stores/permission'

/**
 * 登录页：
 * - 布局复用 AuthLayout（左品牌区 + 右表单区）
 * - 验证码按后端 captchaEnabled 动态显隐；img 为裸 base64，需补前缀
 * - 回车提交一次、失败自动刷新验证码；「记住我」记忆用户名到 localStorage
 * - 注册入口仅当 VITE_APP_REGISTER === 'true' 显示（需后端同步开启注册开关）
 */
const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const permissionStore = usePermissionStore()

/** 「记住我」localStorage key（存用户名） */
const REMEMBER_KEY = 'Admin-Remember-Username'

/** 注册入口开关（默认关闭：后端 sys.account.registerUser 默认为 false） */
const registerEnabled = import.meta.env.VITE_APP_REGISTER === 'true'

/** a-form 实例所需的最小类型面 */
interface FormExpose {
  validate: () => Promise<Record<string, string> | undefined>
  resetFields: () => void
  clearValidate: (fields?: string[]) => void
}

const formRef = ref<FormExpose>()
const loading = ref(false)
const rememberMe = ref(false)

const form = reactive<LoginFormData>({
  username: '',
  password: '',
  code: '',
  uuid: ''
})

/** 表单校验规则（computed 保持语言切换后提示语联动） */
const rules = computed(() => ({
  username: [{ required: true, message: t('login.usernamePlaceholder') }],
  password: [{ required: true, message: t('login.passwordPlaceholder') }],
  code: [{ required: true, message: t('login.captchaPlaceholder') }]
}))

/** 验证码开关（后端返回 captchaEnabled=false 时隐藏验证码输入） */
const captchaEnabled = ref(false)
const captchaImg = ref('')

async function loadCaptcha(): Promise<void> {
  try {
    const result = await getCaptchaImage()
    captchaEnabled.value = result.captchaEnabled !== false
    if (captchaEnabled.value && result.img) {
      // 后端返回裸 base64，需要补 data URI 前缀
      captchaImg.value = `data:image/jpeg;base64,${result.img}`
      form.uuid = result.uuid ?? ''
    } else {
      captchaImg.value = ''
      form.uuid = ''
    }
  } catch {
    // 验证码加载失败不阻塞登录页（后端可能关闭了验证码）
    captchaEnabled.value = false
  }
}

/** 预填用户名：优先「记住我」存档，其次注册成功跳回携带的 query */
function prefillUsername(): void {
  try {
    form.username = localStorage.getItem(REMEMBER_KEY) ?? ''
    rememberMe.value = !!form.username
  } catch {
    /* 存储不可用时忽略 */
  }
  const queryUsername = typeof route.query.username === 'string' ? route.query.username : ''
  if (queryUsername) {
    form.username = queryUsername
  }
}

async function submitLogin(): Promise<void> {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }
  if (loading.value) return
  loading.value = true
  try {
    await userStore.login({
      username: form.username,
      password: form.password,
      code: form.code || undefined,
      uuid: form.uuid || undefined
    })
    // 按勾选状态记忆/清除用户名
    try {
      if (rememberMe.value) {
        localStorage.setItem(REMEMBER_KEY, form.username)
      } else {
        localStorage.removeItem(REMEMBER_KEY)
      }
    } catch {
      /* 存储不可用时忽略 */
    }
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : ''
    // 仅允许站内绝对路径，拦截 //、/\ 等协议相对写法导致的跳转
    if (redirect && redirect !== '/' && redirect.startsWith('/') && !redirect.startsWith('//')) {
      router.push(redirect)
    } else {
      router.push(permissionStore.firstMenuPath)
    }
  } catch {
    // 登录失败：刷新验证码并清空输入（后端校验码一次性有效）
    if (captchaEnabled.value) {
      form.code = ''
      await loadCaptcha()
    }
  } finally {
    loading.value = false
  }
}

function handleLogin(): void {
  void submitLogin()
}

onMounted(() => {
  prefillUsername()
  void loadCaptcha()
})
</script>

<style scoped>
.login-form__title {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-1);
}

.login-form__subtitle {
  margin: 0 0 28px;
  font-size: 14px;
  color: var(--color-text-3);
}

.login-form__captcha {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.login-form__captcha :deep(.arco-input-wrapper) {
  flex: 1;
}

.login-form__captcha-img {
  width: 110px;
  height: 40px;
  border-radius: 4px;
  cursor: pointer;
  border: 1px solid var(--color-border-2);
  flex-shrink: 0;
  object-fit: cover;
}

.login-form__options {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
  font-size: 13px;
}

.login-form__submit {
  height: 40px;
  font-size: 15px;
  border-radius: 6px;
  margin-top: 4px;
}

.login-form__register {
  text-align: center;
  font-size: 13px;
  color: var(--color-text-3);
}

.login-form__register-link {
  color: rgb(var(--primary-6));
  text-decoration: none;
}

.login-form__register-link:hover {
  text-decoration: underline;
}
</style>
