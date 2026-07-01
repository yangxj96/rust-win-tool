<script setup lang="ts">
import { ref } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { Setting, Monitor, Fold, Expand } from "@element-plus/icons-vue";
import { useTheme } from "../composables/useTheme";
import TitleBar from "../components/TitleBar.vue";

const { t } = useI18n();
const router = useRouter();
const route = useRoute();
const isCollapse = ref(false);
useTheme();

function handleMenuSelect(index: string) {
  router.push(index);
}
</script>

<template>
  <div class="layout">
    <TitleBar />
    <div class="layout-body">
      <aside class="sidebar" :style="{ width: isCollapse ? '64px' : '200px' }">
        <el-menu
          :default-active="route.path"
          :collapse="isCollapse"
          class="sidebar-menu"
          @select="handleMenuSelect"
        >
          <el-menu-item index="/services">
            <el-icon><Monitor /></el-icon>
            <template #title>{{ t("menu.services") }}</template>
          </el-menu-item>
          <el-menu-item index="/settings">
            <el-icon><Setting /></el-icon>
            <template #title>{{ t("menu.settings") }}</template>
          </el-menu-item>
        </el-menu>
      </aside>
      <div class="main-wrapper">
        <header class="header">
          <div class="header-left">
            <el-button :icon="isCollapse ? Expand : Fold" text @click="isCollapse = !isCollapse" />
            <el-breadcrumb separator="/">
              <el-breadcrumb-item :to="{ path: '/' }">{{ t("breadcrumb.home") }}</el-breadcrumb-item>
              <el-breadcrumb-item v-if="route.meta.title">{{ t(route.meta.title as string) }}</el-breadcrumb-item>
            </el-breadcrumb>
          </div>
        </header>
        <main class="content">
          <router-view />
        </main>
      </div>
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.layout-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  flex-shrink: 0;
  height: 100%;
  overflow: hidden;
  background-color: var(--el-bg-color);
  transition: width 0.3s;
}

.sidebar-menu {
  height: 100%;
  border-right: none;
}

.main-wrapper {
  flex: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.header {
  display: flex;
  align-items: center;
  height: 60px;
  padding: 0 20px;
  flex-shrink: 0;
  background-color: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color-light);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.content {
  flex: 1;
  overflow-y: auto;
  background-color: var(--el-bg-color-page);
}
</style>
