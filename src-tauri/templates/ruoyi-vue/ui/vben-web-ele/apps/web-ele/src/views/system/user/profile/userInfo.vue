<script lang="ts" setup>
import { reactive, ref, watch } from 'vue';

import { useTabs } from '@vben/hooks';
import { useUserStore } from '@vben/stores';

import {
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElMessage,
  ElRadio,
  ElRadioGroup,
  type FormInstance,
  type FormRules,
} from 'element-plus';

import {
  updateProfileApi,
  type ProfileUser,
} from '#/api/system/profile';

defineOptions({ name: 'UserInfoForm' });

const props = defineProps<{
  user: ProfileUser;
}>();

const emit = defineEmits<{
  (e: 'updated'): void;
}>();

const userStore = useUserStore();
const { closeCurrentTab } = useTabs();

const formRef = ref<FormInstance>();
const saving = ref(false);
const form = reactive({
  nickName: '',
  phonenumber: '',
  email: '',
  sex: '0',
});

const rules: FormRules = {
  nickName: [
    { required: true, message: '用户昵称不能为空', trigger: 'blur' },
  ],
  email: [
    { required: true, message: '邮箱地址不能为空', trigger: 'blur' },
    {
      type: 'email',
      message: '请输入正确的邮箱地址',
      trigger: ['blur', 'change'],
    },
  ],
  phonenumber: [
    { required: true, message: '手机号码不能为空', trigger: 'blur' },
    {
      pattern: /^1[3-9]\d{9}$/,
      message: '请输入正确的手机号码',
      trigger: 'blur',
    },
  ],
};

watch(
  () => props.user,
  (user) => {
    // userId/userName 任一有值即视为已加载（避免空对象覆盖表单）
    if (user?.userId == null && !user?.userName) return;
    form.nickName = user.nickName ?? '';
    form.phonenumber = user.phonenumber ?? '';
    form.email = user.email ?? '';
    // 若依 sex 可能是数字或字符串，统一成单选绑定的字符串
    form.sex = user.sex == null || user.sex === '' ? '0' : String(user.sex);
  },
  { immediate: true, deep: true },
);

async function submit() {
  await formRef.value?.validate();
  saving.value = true;
  try {
    await updateProfileApi({ ...form });
    ElMessage.success('修改成功');
    // 同步左侧卡片与顶栏昵称
    props.user.nickName = form.nickName;
    props.user.phonenumber = form.phonenumber;
    props.user.email = form.email;
    props.user.sex = form.sex;
    if (userStore.userInfo) {
      userStore.userInfo.realName = form.nickName;
    }
    emit('updated');
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
    label-width="90px"
    style="max-width: 500px"
  >
    <ElFormItem label="用户昵称" prop="nickName">
      <ElInput v-model="form.nickName" maxlength="30" />
    </ElFormItem>
    <ElFormItem label="手机号码" prop="phonenumber">
      <ElInput v-model="form.phonenumber" maxlength="11" />
    </ElFormItem>
    <ElFormItem label="邮箱" prop="email">
      <ElInput v-model="form.email" maxlength="50" />
    </ElFormItem>
    <ElFormItem label="性别">
      <ElRadioGroup v-model="form.sex">
        <ElRadio value="0">男</ElRadio>
        <ElRadio value="1">女</ElRadio>
      </ElRadioGroup>
    </ElFormItem>
    <ElFormItem>
      <ElButton type="primary" :loading="saving" @click="submit">
        保存
      </ElButton>
      <ElButton type="danger" @click="close">关闭</ElButton>
    </ElFormItem>
  </ElForm>
</template>
