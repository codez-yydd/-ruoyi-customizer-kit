<script lang="ts" setup>
import { computed, reactive, ref } from 'vue';

import { useTabs } from '@vben/hooks';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  type FormInstance,
  type FormRules,
} from 'element-plus';

import { updateUserPwdApi } from '#/api/system/profile';
import { usePasswordRule } from '#/utils/passwordRule';

defineOptions({ name: 'ResetPwd' });

const { closeCurrentTab } = useTabs();
const { infoPwdValidator } = usePasswordRule();

const formRef = ref<FormInstance>();
const saving = ref(false);
const form = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: '',
});

const rules = computed<FormRules>(() => ({
  oldPassword: [
    { required: true, message: '旧密码不能为空', trigger: 'blur' },
  ],
  newPassword: infoPwdValidator.value,
  confirmPassword: [
    { required: true, message: '确认密码不能为空', trigger: 'blur' },
    {
      validator: (_rule, value, callback) => {
        if (value !== form.newPassword) {
          callback(new Error('两次输入的密码不一致'));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
}));

async function submit() {
  await formRef.value?.validate();
  saving.value = true;
  try {
    await updateUserPwdApi(form.oldPassword, form.newPassword);
    ElMessage.success('修改成功');
    form.oldPassword = '';
    form.newPassword = '';
    form.confirmPassword = '';
    formRef.value?.clearValidate();
  } finally {
    saving.value = false;
  }
}

function close() {
  void closeCurrentTab();
}
</script>

<template>
  <ElForm
    ref="formRef"
    :model="form"
    :rules="rules"
    label-width="100px"
    style="max-width: 500px"
  >
    <ElFormItem label="旧密码" prop="oldPassword">
      <ElInput
        v-model="form.oldPassword"
        type="password"
        placeholder="请输入旧密码"
        show-password
        maxlength="20"
      />
    </ElFormItem>
    <ElFormItem label="新密码" prop="newPassword">
      <ElInput
        v-model="form.newPassword"
        type="password"
        placeholder="请输入新密码"
        show-password
        maxlength="20"
      />
    </ElFormItem>
    <ElFormItem label="确认密码" prop="confirmPassword">
      <ElInput
        v-model="form.confirmPassword"
        type="password"
        placeholder="请确认新密码"
        show-password
        maxlength="20"
      />
    </ElFormItem>
    <ElFormItem>
      <ElButton type="primary" :loading="saving" @click="submit">
        保存
      </ElButton>
      <ElButton type="danger" @click="close">关闭</ElButton>
    </ElFormItem>
  </ElForm>
</template>
