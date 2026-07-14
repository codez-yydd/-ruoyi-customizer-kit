<script setup lang="ts">
// 应用主框架：左侧向导式导航 + 右侧内容区。
// 导航门控：未选项目时除「首页」外其他步骤禁用；
// 已选项目后按 maxStep 解锁进度，只能进入已解锁步骤，防止越级跳转。
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'

const route = useRoute()
const router = useRouter()
const store = useProjectStore()
const { maxStep } = storeToRefs(store)

// 菜单项：step 为该步骤序号（首页为 -1，始终可点）
const menus = [
  { name: 'home', title: '首页', icon: '🏠', step: -1 },
  { name: 'detect', title: '项目识别', icon: '🔍', step: 1 },
  { name: 'config', title: '参数配置', icon: '⚙️', step: 2 },
  { name: 'preview', title: '执行预览', icon: '👁️', step: 3 },
  { name: 'execute', title: '执行改造', icon: '🛠️', step: 4 }
]

const activeName = computed(() => route.name as string)

/** 某菜单是否可点击（已解锁或始终可点） */
function isReachable(step: number) {
  return step < 0 || step <= maxStep.value
}

function go(m: { name: string; step: number }) {
  if (!isReachable(m.step)) return
  router.push({ name: m.name })
}
</script>

<template>
  <div class="rf-layout">
    <aside class="rf-sidebar">
      <div class="rf-sidebar__title">若依锻造台</div>
      <nav class="rf-sidebar__menu">
        <div
          v-for="m in menus"
          :key="m.name"
          class="rf-menu-item"
          :class="{
            active: activeName === m.name,
            disabled: !isReachable(m.step)
          }"
          @click="go(m)"
        >
          <span class="rf-menu-item__icon">{{ m.icon }}</span>
          <span class="rf-menu-item__text">{{ m.title }}</span>
          <span
            v-if="m.step > 0 && m.step <= maxStep"
            class="rf-menu-item__done"
            title="已解锁"
          >
            ✓
          </span>
        </div>
      </nav>
      <div class="rf-sidebar__footer">RuoYi Forge v0.1.0</div>
    </aside>

    <main class="rf-main">
      <div class="rf-content">
        <router-view />
      </div>
    </main>
  </div>
</template>

<style scoped>
.rf-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 18px;
  cursor: pointer;
  font-size: 14px;
  color: var(--rf-sidebar-text);
  transition: background 0.15s;
}

.rf-menu-item:hover {
  background: #263445;
  color: #fff;
}

.rf-menu-item.active {
  background: var(--rf-sidebar-active);
  color: #fff;
}

.rf-menu-item.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.rf-menu-item.disabled:hover {
  background: transparent;
  color: var(--rf-sidebar-text);
}

.rf-menu-item__icon {
  width: 18px;
  text-align: center;
}

.rf-menu-item__text {
  flex: 1;
}

.rf-menu-item__done {
  font-size: 12px;
  color: #67c23a;
}
</style>
