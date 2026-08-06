<script setup lang="ts">
import { computed } from 'vue';

import { ElCol, ElDialog, ElRow, ElTag } from 'element-plus';

import type { SysJob } from '#/api/monitor/job';
import type { SysJobLog } from '#/api/monitor/jobLog';
import { useDict } from '#/composables/useDict';
import DictTag from '#/components/DictTag/index.vue';
import { parseTime } from '#/utils/ruoyi';

defineOptions({ name: 'MonitorJobDetail' });

const props = withDefaults(
  defineProps<{
    /** 是否显示弹窗 */
    visible: boolean;
    /** 任务或日志行数据 */
    row?: Partial<SysJob & SysJobLog>;
    /** job=任务详细 | log=调度日志详细 */
    type?: 'job' | 'log';
  }>(),
  {
    row: () => ({}),
    type: 'job',
  },
);

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const { dictMap } = useDict({ group: 'sys_job_group' });

const form = computed(() => props.row || {});

/** 日志执行耗时（毫秒），仅成功且有起止时间时展示 */
const costTime = computed(() => {
  if (!form.value.startTime || !form.value.endTime) return 0;
  return (
    new Date(form.value.endTime).getTime() -
    new Date(form.value.startTime).getTime()
  );
});

function handleClose() {
  emit('update:visible', false);
}

function misfirePolicyLabel(policy?: string) {
  switch (String(policy)) {
    case '0':
      return '默认策略';
    case '1':
      return '立即执行';
    case '2':
      return '执行一次';
    case '3':
      return '放弃执行';
    default:
      return policy || '-';
  }
}
</script>

<template>
  <ElDialog
    :model-value="visible"
    :title="type === 'log' ? '调度日志详细' : '任务详细'"
    width="780px"
    append-to-body
    @close="handleClose"
  >
    <div class="detail-wrap">
      <!-- 调度日志详细 -->
      <template v-if="type === 'log'">
        <div class="detail-card">
          <div class="detail-card-title">基本信息</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">日志编号</span>
                <span class="detail-value">{{ form.jobLogId }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">执行状态</span>
                <ElTag v-if="form.status == '0'" type="success" size="small">正常</ElTag>
                <ElTag v-else type="danger" size="small">失败</ElTag>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">开始时间</span>
                <span class="detail-value">{{ form.startTime || '-' }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">结束时间</span>
                <span class="detail-value">{{ form.endTime || '-' }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">记录时间</span>
                <span class="detail-value">{{ form.createTime || '-' }}</span>
              </div>
            </ElCol>
            <ElCol v-if="form.status == '0' && form.startTime && form.endTime" :span="12">
              <div class="detail-item">
                <span class="detail-label">执行耗时</span>
                <span class="detail-value">{{ costTime }} 毫秒</span>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <div class="detail-card">
          <div class="detail-card-title">任务信息</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">任务名称</span>
                <span class="detail-value">{{ form.jobName }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">任务分组</span>
                <DictTag :options="dictMap.group" :value="form.jobGroup" />
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="24">
              <div class="detail-item">
                <span class="detail-label">日志信息</span>
                <span class="detail-value">{{ form.jobMessage || '-' }}</span>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <div class="detail-card">
          <div class="detail-card-title">调用目标</div>
          <div class="code-body">
            <pre class="code-pre">{{ form.invokeTarget || '（无）' }}</pre>
          </div>
        </div>

        <div v-if="form.status == '1'" class="detail-card">
          <div class="detail-card-title error-title">异常信息</div>
          <div class="error-body">
            <div class="error-msg">{{ form.exceptionInfo || '（无）' }}</div>
          </div>
        </div>
      </template>

      <!-- 任务详细 -->
      <template v-else>
        <div class="detail-card">
          <div class="detail-card-title">任务配置</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">任务编号</span>
                <span class="detail-value">{{ form.jobId }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">任务名称</span>
                <span class="detail-value">{{ form.jobName }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">任务分组</span>
                <DictTag :options="dictMap.group" :value="form.jobGroup" />
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">执行状态</span>
                <ElTag v-if="form.status == '0'" type="success" size="small">正常</ElTag>
                <ElTag v-else type="info" size="small">暂停</ElTag>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <div class="detail-card">
          <div class="detail-card-title">调度信息</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">cron 表达式</span>
                <span class="detail-value mono">{{ form.cronExpression }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">下次执行时间</span>
                <span class="detail-value">{{ parseTime(form.nextValidTime) || '-' }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">执行策略</span>
                <ElTag size="small">{{ misfirePolicyLabel(form.misfirePolicy) }}</ElTag>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">并发执行</span>
                <ElTag v-if="form.concurrent == '0'" type="success" size="small">允许</ElTag>
                <ElTag v-else type="danger" size="small">禁止</ElTag>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <div class="detail-card">
          <div class="detail-card-title">执行方法</div>
          <div class="code-body">
            <pre class="code-pre">{{ form.invokeTarget || '（无）' }}</pre>
          </div>
        </div>

        <div class="detail-card">
          <div class="detail-card-title">元信息</div>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">创建人</span>
                <span class="detail-value">{{ form.createBy || '-' }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">创建时间</span>
                <span class="detail-value">{{ form.createTime || '-' }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow class="detail-row">
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">更新人</span>
                <span class="detail-value">{{ form.updateBy || '-' }}</span>
              </div>
            </ElCol>
            <ElCol :span="12">
              <div class="detail-item">
                <span class="detail-label">更新时间</span>
                <span class="detail-value">{{ form.updateTime || '-' }}</span>
              </div>
            </ElCol>
          </ElRow>
          <ElRow v-if="form.remark" class="detail-row">
            <ElCol :span="24">
              <div class="detail-item">
                <span class="detail-label">备注</span>
                <span class="detail-value">{{ form.remark }}</span>
              </div>
            </ElCol>
          </ElRow>
        </div>
      </template>
    </div>
  </ElDialog>
</template>

<style scoped>
.detail-wrap {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.detail-card {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
  padding: 12px 16px;
}

.detail-card-title {
  font-weight: 600;
  margin-bottom: 10px;
  color: var(--el-text-color-primary);
}

.detail-card-title.error-title {
  color: var(--el-color-danger);
}

.detail-row + .detail-row {
  margin-top: 8px;
}

.detail-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-height: 28px;
  line-height: 28px;
}

.detail-label {
  width: 90px;
  flex-shrink: 0;
  color: var(--el-text-color-secondary);
}

.detail-value {
  flex: 1;
  word-break: break-all;
}

.detail-value.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.code-body,
.error-body {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 10px 12px;
}

.code-pre,
.error-msg {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 13px;
  line-height: 1.5;
}

.error-msg {
  color: var(--el-color-danger);
}
</style>
