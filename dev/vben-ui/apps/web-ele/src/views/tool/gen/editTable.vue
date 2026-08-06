<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { ElMessage } from 'element-plus';
import Sortable, { type SortableEvent } from 'sortablejs';

import { optionselect as getDictOptionselect, type SysDictType } from '#/api/system/dictType';
import {
  getGenTable,
  updateGenTable,
  type GenTable,
  type GenTableColumn,
} from '#/api/tool/gen';

import BasicInfoForm from './basicInfoForm.vue';
import GenInfoForm from './genInfoForm.vue';

/**
 * 修改代码生成配置
 * 隐藏路由：/tool/gen-edit/index/:tableId
 */
defineOptions({ name: 'ToolGenEdit' });

const route = useRoute();
const router = useRouter();

const activeName = ref('columnInfo');
const tableHeight = `${document.documentElement.scrollHeight - 245}px`;
const tables = ref<GenTable[]>([]);
const columns = ref<GenTableColumn[]>([]);
const dictOptions = ref<SysDictType[]>([]);
const info = ref<Record<string, any>>({});
const basicInfoRef = ref<InstanceType<typeof BasicInfoForm>>();
const genInfoRef = ref<InstanceType<typeof GenInfoForm>>();
const dragTableRef = ref();

function getFormPromise(form: any) {
  return new Promise<boolean>((resolve) => {
    form.validate((valid: boolean) => resolve(valid));
  });
}

async function submitForm() {
  const basicForm = (basicInfoRef.value as any)?.$refs?.basicInfoForm;
  const genForm = (genInfoRef.value as any)?.$refs?.genInfoForm;
  if (!basicForm || !genForm) {
    ElMessage.error('表单未就绪，请稍后重试');
    return;
  }
  const results = await Promise.all([basicForm, genForm].map(getFormPromise));
  if (!results.every(Boolean)) {
    ElMessage.error('表单校验未通过，请重新检查提交内容');
    return;
  }
  const genTable = Object.assign({}, info.value) as GenTable;
  genTable.columns = columns.value;
  genTable.params = {
    genView: info.value.view ? '1' : '0',
    treeCode: info.value.treeCode,
    treeName: info.value.treeName,
    treeParentCode: info.value.treeParentCode,
    parentMenuId: info.value.parentMenuId,
  };
  await updateGenTable(genTable);
  ElMessage.success('修改成功');
  close();
}

function close() {
  router.push({
    path: '/tool/gen',
    query: {
      t: String(Date.now()),
      pageNum: String(route.query.pageNum || 1),
    },
  });
}

async function loadDetail() {
  const tableId = route.params.tableId as string;
  if (!tableId) return;
  // 拦截器已解包 data，直接得到 {info, rows, tables}
  const res = await getGenTable(tableId);
  columns.value = res.rows ?? [];
  info.value = res.info ?? {};
  // 字段信息页需要 info.columns 供树表配置下拉使用
  info.value.columns = columns.value;
  tables.value = res.tables ?? [];

  const dictList = await getDictOptionselect();
  dictOptions.value = dictList ?? [];
}

onMounted(async () => {
  await loadDetail();
  await nextTick();
  const element = dragTableRef.value?.$el?.querySelector(
    '.el-table__body-wrapper tbody',
  ) as HTMLElement | null;
  if (!element) return;
  Sortable.create(element, {
    handle: '.allowDrag',
    onEnd: (evt: SortableEvent) => {
      const oldIndex = evt.oldIndex ?? 0;
      const newIndex = evt.newIndex ?? 0;
      const targetRow = columns.value.splice(oldIndex, 1)[0];
      if (!targetRow) return;
      columns.value.splice(newIndex, 0, targetRow);
      columns.value.forEach((col, index) => {
        col.sort = index + 1;
      });
    },
  });
});
</script>

<template>
  <div class="ruoyi-page">
    <el-card shadow="never">
      <el-tabs v-model="activeName">
        <el-tab-pane label="基本信息" name="basic">
          <BasicInfoForm ref="basicInfoRef" :info="info" />
        </el-tab-pane>
        <el-tab-pane label="字段信息" name="columnInfo">
          <el-table
            ref="dragTableRef"
            :data="columns"
            row-key="columnId"
            :max-height="tableHeight"
          >
            <el-table-column label="序号" type="index" min-width="5%" class-name="allowDrag" />
            <el-table-column
              label="字段列名"
              prop="columnName"
              min-width="10%"
              show-overflow-tooltip
              class-name="allowDrag"
            />
            <el-table-column label="字段描述" min-width="10%">
              <template #default="{ row }">
                <el-input v-model="row.columnComment" />
              </template>
            </el-table-column>
            <el-table-column
              label="物理类型"
              prop="columnType"
              min-width="10%"
              show-overflow-tooltip
            />
            <el-table-column label="Java类型" min-width="11%">
              <template #default="{ row }">
                <el-select v-model="row.javaType">
                  <el-option label="Long" value="Long" />
                  <el-option label="String" value="String" />
                  <el-option label="Integer" value="Integer" />
                  <el-option label="Double" value="Double" />
                  <el-option label="BigDecimal" value="BigDecimal" />
                  <el-option label="Date" value="Date" />
                  <el-option label="Boolean" value="Boolean" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="java属性" min-width="10%">
              <template #default="{ row }">
                <el-input v-model="row.javaField" />
              </template>
            </el-table-column>
            <el-table-column label="插入" min-width="5%">
              <template #default="{ row }">
                <el-checkbox v-model="row.isInsert" true-value="1" false-value="0" />
              </template>
            </el-table-column>
            <el-table-column label="编辑" min-width="5%">
              <template #default="{ row }">
                <el-checkbox v-model="row.isEdit" true-value="1" false-value="0" />
              </template>
            </el-table-column>
            <el-table-column label="列表" min-width="5%">
              <template #default="{ row }">
                <el-checkbox v-model="row.isList" true-value="1" false-value="0" />
              </template>
            </el-table-column>
            <el-table-column label="查询" min-width="5%">
              <template #default="{ row }">
                <el-checkbox v-model="row.isQuery" true-value="1" false-value="0" />
              </template>
            </el-table-column>
            <el-table-column label="查询方式" min-width="10%">
              <template #default="{ row }">
                <el-select v-model="row.queryType">
                  <el-option label="=" value="EQ" />
                  <el-option label="!=" value="NE" />
                  <el-option label=">" value="GT" />
                  <el-option label=">=" value="GTE" />
                  <el-option label="<" value="LT" />
                  <el-option label="<=" value="LTE" />
                  <el-option label="LIKE" value="LIKE" />
                  <el-option label="BETWEEN" value="BETWEEN" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="必填" min-width="5%">
              <template #default="{ row }">
                <el-checkbox v-model="row.isRequired" true-value="1" false-value="0" />
              </template>
            </el-table-column>
            <el-table-column label="显示类型" min-width="12%">
              <template #default="{ row }">
                <el-select v-model="row.htmlType">
                  <el-option label="文本框" value="input" />
                  <el-option label="文本域" value="textarea" />
                  <el-option label="下拉框" value="select" />
                  <el-option label="单选框" value="radio" />
                  <el-option label="复选框" value="checkbox" />
                  <el-option label="日期控件" value="datetime" />
                  <el-option label="图片上传" value="imageUpload" />
                  <el-option label="文件上传" value="fileUpload" />
                  <el-option label="富文本控件" value="editor" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="字典类型" min-width="12%">
              <template #default="{ row }">
                <el-select v-model="row.dictType" clearable filterable placeholder="请选择">
                  <el-option
                    v-for="dict in dictOptions"
                    :key="dict.dictType"
                    :label="dict.dictName"
                    :value="dict.dictType"
                  >
                    <span style="float: left">{{ dict.dictName }}</span>
                    <span style="float: right; color: #8492a6; font-size: 13px">
                      {{ dict.dictType }}
                    </span>
                  </el-option>
                </el-select>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
        <el-tab-pane label="生成信息" name="genInfo">
          <GenInfoForm ref="genInfoRef" :info="info" :tables="tables" />
        </el-tab-pane>
      </el-tabs>
      <div style="margin-top: 16px; text-align: center">
        <el-button type="primary" @click="submitForm">提交</el-button>
        <el-button @click="close">返回</el-button>
      </div>
    </el-card>
  </div>
</template>
