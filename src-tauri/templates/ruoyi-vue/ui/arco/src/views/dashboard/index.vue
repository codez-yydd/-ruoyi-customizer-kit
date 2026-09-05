<template>
  <div class="dashboard">
    <!-- 欢迎横幅：问候 + 昵称 + 日期 + 角色 chips（主色渐变浅底） -->
    <div class="dashboard__banner">
      <div class="dashboard__banner-glow" aria-hidden="true"></div>
      <div class="dashboard__banner-main">
        <h2 class="dashboard__title">
          {{ t('dashboard.welcomeTitle', { greeting, name: userStore.nickName || userStore.name || t('dashboard.fallbackUser') }) }}
        </h2>
        <p class="dashboard__meta">{{ todayText }} · {{ t('dashboard.welcomeUse', { title: appStore.displayTitle }) }}</p>
        <p class="dashboard__motto">{{ t('dashboard.motto') }}</p>
      </div>
      <div class="dashboard__banner-side">
        <span class="dashboard__banner-label">{{ t('dashboard.currentRole') }}</span>
        <div class="dashboard__roles">
          <a-tag v-for="role in displayRoles" :key="role" color="arcoblue" class="dashboard__role-tag">
            {{ role }}
          </a-tag>
          <span v-if="displayRoles.length === 0" class="dashboard__roles-empty">{{ t('dashboard.noRoles') }}</span>
        </div>
      </div>
    </div>

    <!-- 快捷入口：按权限过滤，全部无权限时不渲染 -->
    <template v-if="visibleEntries.length > 0">
      <div class="dashboard__section-title">{{ t('dashboard.quickEntries') }}</div>
      <a-row :gutter="12" class="dashboard__entries">
        <a-col v-for="entry in visibleEntries" :key="entry.path" :xs="24" :sm="12" :lg="6">
          <div class="dashboard__entry app-page-card" @click="goEntry(entry.path)">
            <span class="dashboard__entry-icon" :class="`dashboard__entry-icon--${entry.tone}`">
              <AppIcon :name="entry.icon" />
            </span>
            <div class="dashboard__entry-body">
              <div class="dashboard__entry-title">{{ t(`dashboard.${entry.titleKey}`) }}</div>
              <div class="dashboard__entry-desc">{{ t(`dashboard.${entry.descKey}`) }}</div>
            </div>
          </div>
        </a-col>
      </a-row>
    </template>

    <!-- 系统信息（全部真实数据：用户/角色/权限来自 user store，技术栈为工程事实） -->
    <div class="dashboard__about app-page-card">
      <div class="dashboard__about-title">{{ t('dashboard.systemInfo') }}</div>
      <a-descriptions :column="2" class="dashboard__descriptions">
        <a-descriptions-item :label="t('dashboard.labelCurrentUser')">
          {{ userStore.nickName || userStore.name || '—' }}
        </a-descriptions-item>
        <a-descriptions-item :label="t('dashboard.labelCurrentRole')">
          {{ displayRoles.join(roleSeparator) || '—' }}
        </a-descriptions-item>
        <a-descriptions-item :label="t('dashboard.labelPermissions')">
          {{ t('dashboard.permissionCount', { count: userStore.permissions.length }) }}
        </a-descriptions-item>
        <a-descriptions-item :label="t('dashboard.labelLoginStatus')">
          <span class="dashboard__online">
            <span class="dashboard__online-dot"></span>
            {{ t('dashboard.online') }}
          </span>
        </a-descriptions-item>
        <a-descriptions-item :label="t('dashboard.labelFrontendStack')">
          Vue 3 / TypeScript / Vite / Pinia / Arco Design Vue
        </a-descriptions-item>
        <a-descriptions-item :label="t('dashboard.labelBackendStack')">
          {{BACKEND_STACK}}
        </a-descriptions-item>
      </a-descriptions>
      <p class="dashboard__about-desc">{{ t('dashboard.aboutDesc') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Message } from '@arco-design/web-vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import AppIcon from '@/components/AppIcon/index.vue'
import { checkPermi } from '@/utils/permission'
import { useUserStore } from '@/stores/user'
import { useAppStore } from '@/stores/app'
import type { MessageSchema } from '@/locales'

// 组件名与路由 name 一致，供 keep-alive include 匹配缓存
defineOptions({ name: 'Dashboard' })

/** 首页：欢迎横幅 + 按权限过滤的快捷入口 + 系统信息卡 */
const { t } = useI18n()
const router = useRouter()
const userStore = useUserStore()
const appStore = useAppStore()

/** 按当前时段问候 */
const greeting = computed<string>(() => {
  const hour = new Date().getHours()
  if (hour < 6) return t('dashboard.greetingDawn')
  if (hour < 9) return t('dashboard.greetingMorning')
  if (hour < 12) return t('dashboard.greetingForenoon')
  if (hour < 14) return t('dashboard.greetingNoon')
  if (hour < 18) return t('dashboard.greetingAfternoon')
  return t('dashboard.greetingEvening')
})

/** 星期键（完整 key 元组，索引访问得到可校验的字面量联合类型） */
const WEEK_KEYS = [
  'dashboard.week0',
  'dashboard.week1',
  'dashboard.week2',
  'dashboard.week3',
  'dashboard.week4',
  'dashboard.week5',
  'dashboard.week6'
] as const

/** 当前日期（含星期，格式与星期名随语言切换） */
const todayText = computed<string>(() => {
  const now = new Date()
  const pad = (n: number): string => String(n).padStart(2, '0')
  return t('dashboard.dateFormat', {
    year: String(now.getFullYear()),
    month: pad(now.getMonth() + 1),
    day: pad(now.getDate()),
    week: t(WEEK_KEYS[now.getDay()])
  })
})

/** 横幅角色 chips（getInfo 返回的 roleKey 列表） */
const displayRoles = computed<string[]>(() => userStore.roles)

/** 角色列表连接符（中文顿号 / 英文逗号，随语言切换） */
const roleSeparator = computed<string>(() => t('dashboard.roleSeparator'))

/** 快捷入口与所需查看权限（tone 为浅彩底图标容器色系；文案 key 随语言切换） */
interface DashEntry {
  path: string
  icon: string
  tone: 'blue' | 'green' | 'purple' | 'orange'
  permi: string
  titleKey: keyof MessageSchema['dashboard']
  descKey: keyof MessageSchema['dashboard']
}

const entries = [
  { titleKey: 'entryUserTitle', descKey: 'entryUserDesc', path: '/system/user', icon: 'user', tone: 'blue', permi: 'system:user:list' },
  { titleKey: 'entryRoleTitle', descKey: 'entryRoleDesc', path: '/system/role', icon: 'peoples', tone: 'green', permi: 'system:role:list' },
  { titleKey: 'entryMenuTitle', descKey: 'entryMenuDesc', path: '/system/menu', icon: 'tree', tone: 'purple', permi: 'system:menu:list' },
  { titleKey: 'entryOperlogTitle', descKey: 'entryOperlogDesc', path: '/monitor/operlog', icon: 'log', tone: 'orange', permi: 'monitor:operlog:list' }
] satisfies DashEntry[]

/** 无权限的入口不渲染 */
const visibleEntries = computed(() => entries.filter((entry) => checkPermi([entry.permi])))

function goEntry(path: string): void {
  // 动态菜单可能尚未包含目标路由，router.push 失败时容错提示
  router.push(path).catch(() => {
    Message.warning(t('dashboard.menuNotReady'))
  })
}
</script>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* ---------- 欢迎横幅 ---------- */
.dashboard__banner {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  overflow: hidden;
  padding: 24px 28px;
  border-radius: 8px;
  background:
    radial-gradient(circle at 92% 18%, rgba(var(--primary-6), 0.1), transparent 50%),
    linear-gradient(120deg, rgba(var(--primary-6), 0.14), rgba(var(--purple-6), 0.07));
}

/* 横幅右上装饰光晕（纯 CSS） */
.dashboard__banner-glow {
  position: absolute;
  top: -70px;
  right: -50px;
  width: 240px;
  height: 240px;
  border-radius: 50%;
  background: rgba(var(--primary-6), 0.1);
  filter: blur(40px);
  pointer-events: none;
}

.dashboard__banner-main {
  min-width: 0;
}

.dashboard__title {
  margin: 0 0 8px;
  font-size: 20px;
  font-weight: 600;
  color: var(--color-text-1);
}

.dashboard__meta {
  margin: 0 0 4px;
  font-size: 13px;
  color: var(--color-text-3);
}

.dashboard__motto {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-2);
}

.dashboard__banner-side {
  flex-shrink: 0;
  text-align: right;
}

.dashboard__banner-label {
  display: block;
  margin-bottom: 8px;
  font-size: 12px;
  color: var(--color-text-3);
}

.dashboard__roles {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}

.dashboard__role-tag {
  margin: 0;
}

.dashboard__roles-empty {
  font-size: 13px;
  color: var(--color-text-3);
}

/* ---------- 快捷入口 ---------- */
.dashboard__section-title {
  margin-top: 4px;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-1);
}

.dashboard__entries :deep(.arco-col) {
  margin-bottom: 12px;
}

.dashboard__entry {
  display: flex;
  align-items: center;
  gap: 14px;
  cursor: pointer;
  border: 1px solid transparent;
  transition:
    box-shadow 0.2s,
    border-color 0.2s,
    transform 0.2s;
}

.dashboard__entry:hover {
  /* 暗色主题下 box-shadow 不可见，补描边反馈（亮色下为浅灰细描边，观感一致） */
  border-color: var(--color-fill-3);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.08);
  transform: translateY(-2px);
}

/* 浅彩底圆角图标容器（每项不同色系：底色 12% 透明 + 同系深色图标） */
.dashboard__entry-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 10px;
  font-size: 22px;
  flex-shrink: 0;
}

.dashboard__entry-icon--blue {
  background-color: rgba(var(--primary-6), 0.12);
  color: rgb(var(--primary-6));
}

.dashboard__entry-icon--green {
  background-color: rgba(var(--green-6), 0.12);
  color: rgb(var(--green-6));
}

.dashboard__entry-icon--purple {
  background-color: rgba(var(--purple-6), 0.12);
  color: rgb(var(--purple-6));
}

.dashboard__entry-icon--orange {
  background-color: rgba(var(--orange-6), 0.14);
  color: rgb(var(--orange-6));
}

.dashboard__entry-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.dashboard__entry-desc {
  margin-top: 2px;
  font-size: 12px;
  color: var(--color-text-3);
}

/* ---------- 系统信息 ---------- */
.dashboard__about {
  padding: 20px 24px;
}

.dashboard__about-title {
  margin-bottom: 12px;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-1);
}

.dashboard__online {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: rgb(var(--green-6));
}

.dashboard__online-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: rgb(var(--green-6));
}

.dashboard__about-desc {
  margin: 12px 0 0;
  font-size: 13px;
  line-height: 1.7;
  color: var(--color-text-2);
}
</style>
