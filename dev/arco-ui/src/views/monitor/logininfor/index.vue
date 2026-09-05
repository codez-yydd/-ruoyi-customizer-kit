<template>
  <div class="logininfor-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      selectable
      row-key="infoId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
      @selection-change="setSelection"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="userName" :label="t('monitor.logininfor.userName')">
            <a-input
              v-model.trim="queryParams.userName"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.logininfor.userName') })"
              allow-clear
              style="width: 150px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="ipaddr" :label="t('monitor.logininfor.ipaddr')">
            <a-input
              v-model.trim="queryParams.ipaddr"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.logininfor.ipaddr') })"
              allow-clear
              style="width: 150px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="status" :label="t('common.fields.status')">
            <a-select
              v-model="queryParams.status"
              :options="statusOptions"
              :placeholder="t('monitor.logininfor.statusPlaceholder')"
              allow-clear
              style="width: 120px"
              @change="handleQuery"
            />
          </a-form-item>
          <a-form-item :label="t('monitor.logininfor.loginTime')">
            <a-range-picker v-model="dateRange" style="width: 240px" />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="handleReset">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #toolbar>
        <a-button
          v-hasPermi="['monitor:logininfor:remove', 'system:logininfor:remove']"
          :disabled="multiple"
          status="danger"
          @click="handleDelete()"
        >
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
        <a-button v-hasPermi="['monitor:logininfor:remove', 'system:logininfor:remove']" status="danger" @click="handleClean">
          <template #icon><IconDelete /></template>
          {{ t('common.clean') }}
        </a-button>
        <a-button
          v-hasPermi="['monitor:logininfor:unlock', 'system:logininfor:unlock']"
          @click="openUnlock()"
        >
          <template #icon><IconUnlock /></template>
          {{ t('monitor.logininfor.unlock') }}
        </a-button>
        <a-button
          v-hasPermi="['monitor:logininfor:export', 'system:logininfor:export']"
          :loading="exportLoading"
          @click="handleExportClick"
        >
          <template #icon><IconDownload /></template>
          {{ t('common.export') }}
        </a-button>
      </template>

      <template #cell-status="{ record }">
        <DictTag :options="sysCommonStatus" :value="asLog(record).status" />
      </template>

      <template #cell-operation="{ record }">
        <a-space :size="4">
          <a-link
            v-hasPermi="['monitor:logininfor:unlock', 'system:logininfor:unlock']"
            @click="handleUnlockUser(asLog(record).userName)"
          >
            {{ t('monitor.logininfor.unlock') }}
          </a-link>
          <a-link
            v-hasPermi="['monitor:logininfor:remove', 'system:logininfor:remove']"
            status="danger"
            @click="handleDelete(asLog(record).infoId)"
          >
            {{ t('common.delete') }}
          </a-link>
        </a-space>
      </template>
    </CrudTable>

    <!-- 解锁账号弹窗（按若依：搜索区解锁按钮弹输入框） -->
    <a-modal
      :visible="unlockModal.open"
      :title="t('monitor.logininfor.unlockTitle')"
      :width="420"
      :mask-closable="false"
      :ok-loading="unlockLoading"
      @ok="submitUnlock"
      @cancel="unlockModal.open = false"
      @close="unlockModal.open = false"
    >
      <a-form :model="unlockModal" layout="vertical">
        <a-form-item
          :label="t('monitor.logininfor.userName')"
          required
          :help="t('monitor.logininfor.unlockHelp')"
        >
          <a-input
            v-model.trim="unlockModal.userName"
            :placeholder="t('monitor.logininfor.unlockPlaceholder')"
            allow-clear
            @keyup.enter="submitUnlock"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import {
  IconDelete,
  IconDownload,
  IconRefresh,
  IconSearch,
  IconUnlock
} from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import DictTag from '@/components/DictTag/index.vue'
import {
  cleanLogininfor,
  delLogininfor,
  exportLogininfor,
  listLogininfor,
  unlockLogininfor
} from '@/api/monitor/logininfor'
import type { LogininforQuery, SysLogininfor } from '@/api/monitor/logininfor'
import { useCrud } from '@/hooks/useCrud'
import { useDict } from '@/hooks/useDict'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Logininfor' })

const { t } = useI18n()

const dicts = useDict('sys_common_status')
const sysCommonStatus = dicts['sys_common_status']

const statusOptions = computed(() =>
  sysCommonStatus.value.map((item) => ({ label: item.dictLabel, value: item.dictValue }))
)

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'infoId', label: t('monitor.logininfor.infoId'), width: 90 },
  { key: 'userName', label: t('monitor.logininfor.userName'), width: 120 },
  { key: 'ipaddr', label: t('monitor.logininfor.ipaddr'), width: 140 },
  { key: 'loginLocation', label: t('monitor.logininfor.loginLocation'), width: 120 },
  { key: 'browser', label: t('monitor.logininfor.browser'), minWidth: 130, ellipsis: true, tooltip: true },
  { key: 'os', label: t('monitor.logininfor.os'), width: 120, ellipsis: true, tooltip: true },
  { key: 'msg', label: t('monitor.logininfor.msg'), minWidth: 160, ellipsis: true, tooltip: true },
  { key: 'status', label: t('monitor.logininfor.loginStatus'), width: 90 },
  { key: 'loginTime', label: t('monitor.logininfor.loginTime'), width: 165 },
  { key: 'operation', label: t('common.fields.operation'), width: 110 }
])

/* ---------- 查询/导出 ---------- */
const dateRange = ref<[string, string] | undefined>()

function mergeDateRange(query: LogininforQuery): LogininforQuery {
  const next = { ...query }
  delete next.params
  const range = dateRange.value
  if (range && range.length === 2 && range[0] && range[1]) {
    next.params = { beginTime: range[0], endTime: range[1] }
  }
  return next
}

const crud = useCrud<SysLogininfor, LogininforQuery>({
  listApi: (query) => listLogininfor(mergeDateRange(query)),
  deleteApi: delLogininfor,
  exportUrl: '/monitor/logininfor/export',
  exportName: `${t('monitor.logininfor.exportFileName')}.xlsx`,
  pkField: 'infoId'
})

const {
  loading,
  exportLoading,
  list,
  total,
  page,
  limit,
  getList,
  handleQuery,
  resetQuery,
  setSelection,
  multiple,
  handleDelete
} = crud

const queryParams = crud.queryParams

function asLog(record: TableData): SysLogininfor {
  return record as SysLogininfor
}

function handleReset(): void {
  dateRange.value = undefined
  resetQuery()
}

async function handleExportClick(): Promise<void> {
  if (exportLoading.value) return
  exportLoading.value = true
  try {
    await exportLogininfor(
      mergeDateRange({ ...queryParams, pageNum: page.value, pageSize: limit.value })
    )
  } catch {
    // 导出失败已由 download.ts/拦截器提示
  } finally {
    exportLoading.value = false
  }
}

/** 清空全部登录日志（单独确认文案，防误操作） */
function handleClean(): void {
  Modal.confirm({
    title: t('common.cleanConfirm'),
    content: t('common.cleanAllConfirm', { field: t('monitor.logininfor.name') }),
    hideCancel: false,
    onOk: async () => {
      try {
        await cleanLogininfor()
        Message.success(t('common.cleanSuccess'))
        await getList()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/* ---------- 账号解锁 ---------- */
const unlockModal = reactive<{ open: boolean; userName: string }>({
  open: false,
  userName: ''
})
const unlockLoading = ref(false)

function openUnlock(userName?: string): void {
  unlockModal.userName = userName ?? ''
  unlockModal.open = true
}

/** 行内解锁（带确认） */
function handleUnlockUser(userName?: string): void {
  if (!userName) {
    Message.warning(t('monitor.logininfor.noUserName'))
    return
  }
  Modal.confirm({
    title: t('monitor.logininfor.unlockConfirmTitle'),
    content: t('monitor.logininfor.unlockConfirm', { name: userName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await unlockLogininfor(userName)
        Message.success(t('monitor.logininfor.unlockSuccess', { name: userName }))
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

async function submitUnlock(): Promise<void> {
  if (!unlockModal.userName) {
    Message.warning(t('monitor.logininfor.unlockPlaceholder'))
    return
  }
  unlockLoading.value = true
  try {
    await unlockLogininfor(unlockModal.userName)
    Message.success(t('monitor.logininfor.unlockSuccess', { name: unlockModal.userName }))
    unlockModal.open = false
  } catch {
    // 失败提示已由响应拦截器统一弹出（含账号不存在的后端提示）
  } finally {
    unlockLoading.value = false
  }
}

/* ---------- 初始化 ---------- */
void getList()
</script>
