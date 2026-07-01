<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Sunny, Moon } from "@element-plus/icons-vue";
import { useTheme } from "../composables/useTheme";

const { t, locale } = useI18n();
const { theme, setTheme } = useTheme();
const notifications = ref(true);
const language = ref(locale.value);

const languages = [
  { label: "简体中文", value: "zh-CN" },
  { label: "English", value: "en" },
];

watch(theme, (val) => {
  setTheme(val);
});

watch(language, (val) => {
  locale.value = val;
  localStorage.setItem("language", val);
});
</script>

<template>
  <div class="settings-container">
    <h2 class="text-xl font-bold mb-6">{{ t("settings.title") }}</h2>

    <el-card class="settings-card">
      <template #header>
        <div class="card-header">
          <span>{{ t("settings.appearance.title") }}</span>
        </div>
      </template>
      <el-form label-width="120px">
        <el-form-item :label="t('settings.appearance.theme')">
          <div class="theme-switch-wrapper">
            <el-switch
              v-model="theme"
              active-value="dark"
              inactive-value="light"
              inline-prompt
              :active-icon="Moon"
              :inactive-icon="Sunny"
              class="theme-switch"
            />
            <span class="theme-label">{{ theme === "dark" ? t("settings.appearance.darkMode") : t("settings.appearance.lightMode") }}</span>
          </div>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card class="settings-card">
      <template #header>
        <div class="card-header">
          <span>{{ t("settings.system.title") }}</span>
        </div>
      </template>
      <el-form label-width="120px">
        <el-form-item :label="t('settings.system.language')">
          <el-select v-model="language" class="w-full">
            <el-option
              v-for="lang in languages"
              :key="lang.value"
              :label="lang.label"
              :value="lang.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('settings.system.notification')">
          <el-switch v-model="notifications" />
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<style scoped>
.settings-container {
  padding: 20px;
  max-width: 800px;
}

.settings-card {
  margin-bottom: 20px;
}

.card-header {
  font-weight: 600;
}

.theme-switch-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
}

.theme-switch {
  --el-switch-on-color: #2c2c2c;
  --el-switch-off-color: #e6a23c;
}

.theme-label {
  font-size: 14px;
  color: var(--el-text-color-regular);
}

:deep(.el-card__header) {
  padding: 12px 20px;
  background-color: var(--el-fill-color-light);
}
</style>
