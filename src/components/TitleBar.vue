<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Close } from "@element-plus/icons-vue";

const appWindow = getCurrentWindow();

function minimize() {
  appWindow.minimize();
}

function close() {
  appWindow.close();
}

function startDrag(event: MouseEvent) {
  if ((event.target as HTMLElement).closest('.titlebar-button')) return;
  appWindow.startDragging();
}
</script>

<template>
  <div class="titlebar" @mousedown="startDrag">
    <div class="titlebar-drag">
      <span class="titlebar-title">Rust Win Tool</span>
    </div>
    <div class="titlebar-controls">
      <button class="titlebar-button" @click="minimize">
        <el-icon><Minus /></el-icon>
      </button>
      <button class="titlebar-button close-button" @click="close">
        <el-icon><Close /></el-icon>
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 32px;
  background-color: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color-light);
  user-select: none;
  position: relative;
  z-index: 100;
}

.titlebar-drag {
  flex: 1;
  height: 100%;
  display: flex;
  align-items: center;
  padding-left: 12px;
}

.titlebar-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.titlebar-controls {
  display: flex;
  height: 100%;
}

.titlebar-button {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  color: var(--el-text-color-regular);
  cursor: pointer;
  transition: background-color 0.2s;
}

.titlebar-button:hover {
  background-color: var(--el-fill-color-light);
}

.titlebar-button.close-button:hover {
  background-color: var(--el-color-danger);
  color: var(--el-color-white);
}

.titlebar-button .el-icon {
  font-size: 14px;
}
</style>
