<template>
  <div class="online-page">
    <CrudTable
      :data="list"
      :loading="loading"
      :columns="columns"
      row-key="tokenId"
      v-model:page="page"
      v-model:limit="limit"
      :total="total"
      @query="getList"
    >
      <template #search>
        <a-form :model="queryParams" layout="inline">
          <a-form-item field="userName" :label="t('monitor.online.loginName')">
            <a-input
              v-model.trim="queryParams.userName"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.online.loginName') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item field="ipaddr" :label="t('monitor.online.ipaddr')">
            <a-input
              v-model.trim="queryParams.ipaddr"
              :placeholder="t('common.pleaseEnter', { field: t('monitor.online.ipaddr') })"
              allow-clear
              style="width: 160px"
              @keyup.enter="handleQuery"
              @clear="handleQuery"
            />
          </a-form-item>
          <a-form-item>
            <a-space>
              <a-button type="primary" @click="handleQuery">
                <template #icon><IconSearch /></template>
                {{ t('common.search') }}
              </a-button>
              <a-button @click="resetQuery">
                <template #icon><IconRefresh /></template>
                {{ t('common.reset') }}
              </a-button>
            </a-space>
          </a-form-item>
        </a-form>
      </template>

      <template #cell-tokenId="{ record }">
        <span :title="asOnline(record).tokenId">{{ shortToken(asOnline(record).tokenId) }}</span>
      </template>

      <template #cell-loginTime="{ record }">
        {{ formatTime(asOnline(record).loginTime) }}
      </template>

      <template #cell-operation="{ record }">
        <a-link
          v-hasPermi="['monitor:online:forceLogout']"
          status="danger"
          @click="handleForceLogout(asOnline(record))"
        >
          {{ t('monitor.online.forceLogout') }}
        </a-link>
      </template>
    </CrudTable>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { TableData } from '@arco-design/web-vue'
import { Message, Modal } from '@arco-design/web-vue'
import { IconRefresh, IconSearch } from '@arco-design/web-vue/es/icon'
import { useI18n } from 'vue-i18n'
import CrudTable from '@/components/CrudTable/index.vue'
import type { CrudColumn } from '@/components/CrudTable/index.vue'
import { forceLogout, listOnline } from '@/api/monitor/online'
import type { OnlineQuery, OnlineUser } from '@/api/monitor/online'
import { useCrud } from '@/hooks/useCrud'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Online' })

const { t } = useI18n()

/** 列定义（computed：随语言切换联动列标题） */
const columns = computed<CrudColumn[]>(() => [
  { key: 'tokenId', label: t('monitor.online.tokenId'), width: 120, tooltip: true },
  { key: 'userName', label: t('monitor.online.loginName'), width: 120 },
  { key: 'deptName', label: t('monitor.online.dept'), width: 140, ellipsis: true, tooltip: true },
  { key: 'ipaddr', label: t('monitor.online.ipaddr'), width: 140 },
  { key: 'loginLocation', label: t('monitor.online.loginLocation'), width: 120 },
  { key: 'browser', label: t('monitor.online.browser'), minWidth: 120, ellipsis: true, tooltip: true },
  { key: 'os', label: t('monitor.online.os'), width: 120, ellipsis: true, tooltip: true },
  { key: 'loginTime', label: t('monitor.online.loginTime'), width: 170 },
  { key: 'operation', label: t('common.fields.operation'), width: 90 }
])

const { loading, list, total, page, limit, queryParams, getList, handleQuery, resetQuery } =
  useCrud<OnlineUser, OnlineQuery>({
    listApi: listOnline,
    pkField: 'tokenId'
  })

function asOnline(record: TableData): OnlineUser {
  return record as OnlineUser
}

/** 会话编号 UUID 截断展示（悬浮显示完整值） */
function shortToken(tokenId?: string): string {
  if (!tokenId) return '-'
  return tokenId.length > 12 ? `${tokenId.slice(0, 12)}...` : tokenId
}

/** epoch 毫秒 -> yyyy-MM-dd HH:mm:ss（在线用户 loginTime 为时间戳） */
function formatTime(ms?: number): string {
  if (!ms) return '-'
  const date = new Date(ms)
  const pad = (value: number): string => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/** 强退会话（带确认） */
function handleForceLogout(row: OnlineUser): void {
  Modal.confirm({
    title: t('monitor.online.forceLogoutConfirmTitle'),
    content: t('monitor.online.forceLogoutConfirm', { name: row.userName }),
    hideCancel: false,
    onOk: async () => {
      try {
        await forceLogout(row.tokenId)
        Message.success(t('monitor.online.forceLogoutSuccess'))
        await getList()
      } catch {
        // 失败提示已由响应拦截器统一弹出
      }
    }
  })
}

/* ---------- 初始化 ---------- */
void getList()
</script>
