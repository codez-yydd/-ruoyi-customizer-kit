<script lang="ts" setup>
/**
 * 基础布局：接入若依顶部通知公告（listTop / markRead / markReadAll），
 * 替换原先的演示用假数据。
 */
import type { NotificationItem } from '@vben/layouts';

import { computed, onMounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';

import { AuthenticationLoginExpiredModal } from '@vben/common-ui';
import { useWatermark } from '@vben/hooks';
import { UserRoundPen } from '@vben/icons';
import {
  BasicLayout,
  LockScreen,
  Notification,
  UserDropdown,
} from '@vben/layouts';
import { preferences } from '@vben/preferences';
import { useAccessStore, useUserStore } from '@vben/stores';

import {
  listNoticeTop,
  markNoticeRead,
  markNoticeReadAll,
  type SysNotice,
} from '#/api/system/notice';
import { useAuthStore } from '#/store';
import LoginForm from '#/views/_core/authentication/login.vue';
import NoticeDetailView from '#/views/system/notice/DetailView.vue';

/** 扩展通知项：携带公告 ID，便于点击时标记已读并打开详情 */
type NoticeNotificationItem = NotificationItem & { noticeId: number };

const notifications = ref<NoticeNotificationItem[]>([]);
const noticeDetailRef = ref<InstanceType<typeof NoticeDetailView>>();

const userStore = useUserStore();
const authStore = useAuthStore();
const accessStore = useAccessStore();
const router = useRouter();
const { destroyWatermark, updateWatermark } = useWatermark();
const showDot = computed(() =>
  notifications.value.some((item) => !item.isRead),
);

/** 将后端公告映射为布局 Notification 组件所需结构 */
function mapNoticeToItem(notice: SysNotice): NoticeNotificationItem {
  return {
    avatar: preferences.app.defaultAvatar,
    date: notice.createTime || '',
    isRead: !!notice.isRead,
    message: notice.noticeType === '1' ? '通知' : '公告',
    noticeId: notice.noticeId,
    title: notice.noticeTitle,
  };
}

async function loadNoticeTop() {
  try {
    const res = await listNoticeTop();
    const list = res.data ?? [];
    notifications.value = list.map(mapNoticeToItem);
  } catch {
    // 未登录或无权限时静默失败，避免打断布局渲染
    notifications.value = [];
  }
}

// 用户下拉菜单：仅保留「个人中心」（退出登录由组件内置项处理）
const menus = computed(() => [
  {
    handler: () => {
      router.push('/user/profile');
    },
    icon: UserRoundPen,
    text: '个人中心',
  },
]);

const avatar = computed(() => {
  return userStore.userInfo?.avatar ?? preferences.app.defaultAvatar;
});

// 描述：优先用登录用户邮箱，其次用户名
const description = computed(
  () => userStore.userInfo?.username || userStore.userInfo?.realName || '',
);

async function handleLogout() {
  await authStore.logout(false);
}

function handleNoticeClear() {
  notifications.value = [];
}

async function handleMakeAll() {
  const ids = notifications.value.map((n) => n.noticeId).join(',');
  if (!ids) return;
  try {
    await markNoticeReadAll(ids);
  } catch {
    /* 接口失败仍本地标记，避免交互卡死 */
  }
  notifications.value = notifications.value.map((item) => ({
    ...item,
    isRead: true,
  }));
}

/** 点击单条：标记已读并打开公告详情 */
async function handleNoticeRead(item: NotificationItem) {
  const noticeItem = item as NoticeNotificationItem;
  if (!noticeItem.isRead && noticeItem.noticeId) {
    try {
      await markNoticeRead(noticeItem.noticeId);
    } catch {
      /* ignore */
    }
    noticeItem.isRead = true;
  }
  if (noticeItem.noticeId) {
    noticeDetailRef.value?.open(noticeItem.noticeId);
  }
}

function handleViewAll() {
  router.push('/system/notice');
}

watch(
  () => preferences.app.watermark,
  async (enable) => {
    if (enable) {
      await updateWatermark({
        content: `${userStore.userInfo?.username}`,
      });
    } else {
      destroyWatermark();
    }
  },
  {
    immediate: true,
  },
);

onMounted(loadNoticeTop);
</script>

<template>
  <BasicLayout @clear-preferences-and-logout="handleLogout">
    <template #user-dropdown>
      <UserDropdown
        :avatar
        :description
        :menus
        :text="userStore.userInfo?.realName"
        @logout="handleLogout"
      />
    </template>
    <template #notification>
      <Notification
        :dot="showDot"
        :notifications="notifications"
        @clear="handleNoticeClear"
        @make-all="handleMakeAll"
        @read="handleNoticeRead"
        @view-all="handleViewAll"
      />
    </template>
    <template #extra>
      <AuthenticationLoginExpiredModal
        v-model:open="accessStore.loginExpired"
        :avatar
      >
        <LoginForm />
      </AuthenticationLoginExpiredModal>
      <NoticeDetailView ref="noticeDetailRef" />
    </template>
    <template #lock-screen>
      <LockScreen :avatar @to-login="handleLogout" />
    </template>
  </BasicLayout>
</template>
