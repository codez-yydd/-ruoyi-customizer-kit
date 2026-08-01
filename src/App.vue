<script setup lang="ts">
// 应用主框架：左侧向导式导航 + 右侧内容区。
// 导航门控：未选项目时除「首页」外其他步骤禁用；
// 已选项目后按 maxStep 解锁进度，只能进入已解锁步骤，防止越级跳转。
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useProjectStore } from '@/stores/project'
import { Box, Check, HomeFilled, Search, Setting, Tools, View } from '@element-plus/icons-vue'

const route = useRoute()
const router = useRouter()
const store = useProjectStore()
const { maxStep } = storeToRefs(store)

// 菜单项：step 为该步骤序号（首页为 -1，始终可点）
const menus = [
  { name: 'home', title: '首页', icon: HomeFilled, step: -1 },
  { name: 'detect', title: '项目识别', icon: Search, step: 1 },
  { name: 'config', title: '参数配置', icon: Setting, step: 2 },
  { name: 'preview', title: '执行预览', icon: View, step: 3 },
  { name: 'execute', title: '执行改造', icon: Tools, step: 4 }
]

// 侧边栏底部进度条的文字（maxStep: 0-5）
const stepNames = ['未开始', '项目识别', '参数配置', '执行预览', '执行改造', '完成']

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
      <div class="rf-brand">
        <div class="rf-brand__logo">
          <el-icon :size="22"><Box /></el-icon>
        </div>
        <div class="rf-brand__text">
          <div class="rf-brand__name">若依锻造台</div>
          <div class="rf-brand__sub">RuoYi Forge</div>
        </div>
      </div>

      <nav class="rf-sidebar__menu">
        <div
          v-for="m in menus"
          :key="m.name"
          class="rf-menu-item"
          :class="{ active: activeName === m.name, disabled: !isReachable(m.step) }"
          @click="go(m)"
        >
          <el-icon class="rf-menu-item__icon"><component :is="m.icon" /></el-icon>
          <span class="rf-menu-item__text">{{ m.title }}</span>
          <el-icon
            v-if="m.step > 0 && m.step <= maxStep"
            class="rf-menu-item__done"
            :size="13"
            title="已解锁"
          >
            <Check />
          </el-icon>
        </div>
      </nav>

      <div class="rf-sidebar__progress">
        <div class="rf-sidebar__progress-label">
          <span>向导进度</span>
          <span class="rf-sidebar__progress-step">{{ stepNames[Math.min(maxStep, 5)] }}</span>
        </div>
        <div class="rf-sidebar__dots">
          <span
            v-for="s in 5"
            :key="s"
            class="rf-sidebar__dot"
            :class="{ done: s <= maxStep }"
          ></span>
        </div>
      </div>

      <div class="rf-sidebar__footer">RuoYi Forge v0.1.0</div>
    </aside>

    <main class="rf-main">
      <div class="rf-content">
        <router-view v-slot="{ Component }">
          <transition name="rf-fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </div>
    </main>
  </div>
</template>

<style scoped>
.rf-menu-item {
  display: flex;
  align-items: center;
  gap: 11px;
  margin: 2px 10px;
  padding: 10px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  color: var(--rf-sidebar-text);
  transition:
    background 0.18s ease,
    color 0.18s ease;
  position: relative;
}

.rf-menu-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #e5eaf0;
}

.rf-menu-item.active {
  background: linear-gradient(90deg, rgba(64, 158, 255, 0.28), rgba(64, 158, 255, 0.1));
  color: #fff;
}

.rf-menu-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 20%;
  bottom: 20%;
  width: 3px;
  border-radius: 2px;
  background: var(--rf-sidebar-active);
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
  font-size: 17px;
  color: #7d8ca0;
  transition: color 0.18s ease;
}

.rf-menu-item:hover .rf-menu-item__icon {
  color: #b8c4d3;
}

.rf-menu-item.active .rf-menu-item__icon {
  color: #fff;
}

.rf-menu-item__text {
  flex: 1;
}

.rf-menu-item__done {
  color: var(--rf-sidebar-active);
}

.rf-sidebar__progress {
  padding: 12px 18px 6px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.rf-sidebar__progress-label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: #7a8794;
  margin-bottom: 8px;
}

.rf-sidebar__progress-step {
  color: #a8c7ea;
  font-weight: 600;
}

.rf-sidebar__dots {
  display: flex;
  gap: 6px;
}

.rf-sidebar__dot {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.12);
  transition: background 0.25s ease;
}

.rf-sidebar__dot.done {
  background: var(--rf-sidebar-active);
}
</style>
