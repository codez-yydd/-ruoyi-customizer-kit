<script lang="ts" setup>
import type { VbenFormSchema } from '@vben/common-ui';

import { computed, h, onBeforeUnmount, onMounted, ref } from 'vue';

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

/**
 * 验证码图片渲染函数：作为 form schema 的 suffix 渲染在输入框右侧。
 *
 * 说明：AuthenticationLogin 组件本身没有 captcha-image 插槽，
 * 因此必须用表单字段的 suffix（form-field.vue 会把 suffix 渲染在
 * FormControl 右侧），否则验证码图片无处显示。
 */
// 返回 any 以匹配 VbenFormSchema.suffix 的类型签名
// （CustomRenderType 标注为 () => Component | string，但运行时作为
//  函数式组件由 VbenRenderContent 直接 h() 渲染其 VNode 返回值）
function renderCaptchaImage(): any {
  return captchaImg.value
    ? h('img', {
        src: captchaImg.value,
        alt: '验证码',
        title: '点击刷新验证码',
        class: 'captcha-img',
        onClick: refreshCaptcha,
      })
    : null;
}

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

  // 若依启用验证码时，追加验证码输入字段 + 右侧图片（suffix）
  if (captchaEnabled.value) {
    fields.push({
      component: 'VbenInput',
      componentProps: {
        placeholder: '请输入验证码',
      },
      fieldName: 'code',
      label: '验证码',
      rules: z.string().min(1, { message: '请输入验证码' }),
      // 右侧渲染验证码图片，点击刷新
      suffix: renderCaptchaImage,
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
    :show-code-login="false"
    :show-forget-password="false"
    :show-qrcode-login="false"
    :show-register="false"
    :show-third-party-login="false"
    @submit="handleSubmit"
  />
</template>

<style scoped>
:deep(.captcha-img) {
  height: 38px;
  cursor: pointer;
  border-radius: 4px;
  border: 1px solid #dcdfe6;
}
</style>
