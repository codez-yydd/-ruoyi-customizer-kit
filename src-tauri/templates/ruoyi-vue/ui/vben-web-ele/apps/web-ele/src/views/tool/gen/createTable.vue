<script setup lang="ts">
import { ref } from 'vue';

import { ElButton, ElInput, ElMessage } from 'element-plus';

import { createTable } from '#/api/tool/gen';

/**
 * 创建表弹窗
 * 直接执行建表 SQL（支持多条），仅 admin 角色可用。
 */
defineOptions({ name: 'ToolGenCreateTable' });

const emit = defineEmits<{ ok: [] }>();

const visible = ref(false);
const content = ref('');

function show() {
  content.value = '';
  visible.value = true;
}

async function handleCreateTable() {
  if (!content.value.trim()) {
    ElMessage.error('请输入建表语句');
    return;
  }
  const res = (await createTable({
    sql: content.value,
    tplWebType: 'element-plus',
  })) as { code?: number; msg?: string };
  ElMessage.success(res.msg || '创建成功');
  if (res.code === 200) {
    visible.value = false;
    emit('ok');
  }
}

defineExpose({ show });
</script>

<template>
  <el-dialog v-model="visible" title="创建表" width="800px" top="5vh" append-to-body destroy-on-close>
    <span>创建表语句(支持多个建表语句)：</span>
    <ElInput
      v-model="content"
      type="textarea"
      :rows="10"
      placeholder="请输入建表 SQL"
      style="margin-top: 8px"
    />
    <template #footer>
      <ElButton type="primary" @click="handleCreateTable">确 定</ElButton>
      <ElButton @click="visible = false">取 消</ElButton>
    </template>
  </el-dialog>
</template>
