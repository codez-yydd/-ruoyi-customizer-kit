<script lang="ts" setup>
import type { VbenFormSchema } from '@vben/common-ui';

import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

import { AuthenticationLogin, z } from '@vben/common-ui';
import { $t } from '@vben/locales';

import { getCaptchaApi } from '#/api';
import { useAuthStore } from '#/store';

defineOptions({ name: 'Login' });

const authStore = useAuthStore();

// ===== 适配若依图形验证码 =====
const captchaImg = ref('');
const captchaUuid = ref('');
const captchaEnabled = ref(true);

/** 获取验证码图片 */
async function refreshCaptcha() {
  try {
    const result = await getCaptchaApi();
    captchaEnabled.value = result.captchaEnabled;
    if (result.captchaEnabled) {
      // 若依 img 是纯 base64，补全 data URI 前缀
      captchaImg.value = result.img.startsWith('data:')
        ? result.img
        : `data:image/jpeg;base64,${result.img}`;
      captchaUuid.value = result.uuid;
    }
  } catch {
    // 获取验证码失败时不阻塞登录（后端可能关闭验证码）
    captchaEnabled.value = false;
  }
}

onMounted(refreshCaptcha);
onBeforeUnmount(() => {
  captchaImg.value = '';
  captchaUuid.value = '';
});

const formSchema = computed((): VbenFormSchema[] => {
  const fields: VbenFormSchema[] = [
    {
      component: 'VbenInput',
      componentProps: {
        placeholder: $t('authentication.usernameTip'),
      },
      fieldName: 'username',
      label: $t('authentication.username'),
      rules: z.string().min(1, { message: $t('authentication.usernameTip') }),
    },
    {
      component: 'VbenInputPassword',
      componentProps: {
        placeholder: $t('authentication.password'),
      },
      fieldName: 'password',
      label: $t('authentication.password'),
      rules: z.string().min(1, { message: $t('authentication.passwordTip') }),
    },
  ];

  // 若依启用验证码时，追加验证码输入字段
  if (captchaEnabled.value) {
    fields.push({
      component: 'VbenInput',
      componentProps: {
        placeholder: '请输入验证码',
      },
      fieldName: 'code',
      label: '验证码',
      rules: z.string().min(1, { message: '请输入验证码' }),
    });
  }

  return fields;
});

/**
 * 提交登录：组装若依所需参数（username/password/code/uuid）
 */
async function handleSubmit(values: Record<string, any>) {
  await authStore.authLogin({
    username: values.username,
    password: values.password,
    code: values.code,
    uuid: captchaUuid.value,
  });
}
</script>

<template>
  <AuthenticationLogin
    :form-schema="formSchema"
    :loading="authStore.loginLoading"
    @submit="handleSubmit"
  >
    <!-- 若依图形验证码图片，点击刷新 -->
    <template v-if="captchaEnabled && captchaImg" #captcha-image>
      <img
        :src="captchaImg"
        alt="验证码"
        title="点击刷新验证码"
        class="captcha-img"
        @click="refreshCaptcha"
      />
    </template>
  </AuthenticationLogin>
</template>

<style scoped>
.captcha-img {
  height: 38px;
  cursor: pointer;
  border-radius: 4px;
  border: 1px solid #dcdfe6;
}
</style>
