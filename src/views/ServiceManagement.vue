<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { ElMessage, ElMessageBox } from "element-plus";
import { Plus, Delete, VideoPlay, VideoPause, Refresh, EditPen, Warning } from "@element-plus/icons-vue";

const { t } = useI18n();

function friendlyError(err: unknown, action: string): string {
  const msg = String(err);
  if (/OpenError|CouldNotStartService|CouldNotStopService|拒绝访问|Access.?Denied|权限|administrator/i.test(msg)) {
    return t("services.error.permission", { action });
  }
  if (/not found|找不到|不存在/i.test(msg)) {
    return t("services.error.notFound");
  }
  if (/timeout|超时/i.test(msg)) {
    return t("services.error.timeout", { action });
  }
  return t("services.error.default", { action });
}

interface ServiceInfo {
  name: string;
  display_name: string;
  status: string;
  start_type: string;
  description: string;
}

interface ManagedService {
  name: string;
  display_name: string;
  enabled: boolean;
}

const managedServices = ref<ManagedService[]>([]);
const allServices = ref<ServiceInfo[]>([]);
const serviceStatuses = ref<Record<string, string>>({});
const showDialog = ref(false);
const searchQuery = ref("");
const loading = ref(false);
const tableLoading = ref(false);
const tableLoadingText = ref("");
const refreshing = ref(false);
const editDialogVisible = ref(false);
const editForm = ref({ name: "", display_name: "", enabled: true });

async function loadManaged() {
  managedServices.value = await invoke("get_managed_services");
  await refreshStatuses();
}

async function refreshStatuses() {
  refreshing.value = true;
  tableLoadingText.value = t("common.loading");
  tableLoading.value = true;
  try {
    const services = managedServices.value;
    const results = await Promise.allSettled(
      services.map((svc) => invoke<string>("get_service_status", { name: svc.name }))
    );
    for (let i = 0; i < services.length; i++) {
      const r = results[i];
      serviceStatuses.value[services[i].name] =
        r.status === "fulfilled" ? r.value : "Unknown";
    }
  } finally {
    tableLoading.value = false;
    refreshing.value = false;
  }
}

async function openAddDialog() {
  loading.value = true;
  showDialog.value = true;
  try {
    allServices.value = await invoke("list_services");
  } catch (e) {
    ElMessage.error({ message: friendlyError(e, t("common.add")), icon: Warning });
  } finally {
    loading.value = false;
  }
}

const filteredServices = computed(() => {
  const q = searchQuery.value.toLowerCase();
  const existing = new Set(managedServices.value.map((s) => s.name));
  return allServices.value.filter(
    (s) =>
      !existing.has(s.name) &&
      (s.name.toLowerCase().includes(q) || s.display_name.toLowerCase().includes(q))
  );
});

async function addService(svc: ServiceInfo) {
  managedServices.value = await invoke("add_managed_service", {
    name: svc.name,
    displayName: svc.display_name,
  });
  await refreshStatuses();
  ElMessage.success(t("services.message.added", { name: svc.display_name }));
}

async function removeService(name: string) {
  await ElMessageBox.confirm(t("services.confirm.remove"), t("common.confirm"), { type: "warning" });
  tableLoadingText.value = t("common.loading");
  tableLoading.value = true;
  try {
    managedServices.value = await invoke("remove_managed_service", { name });
    delete serviceStatuses.value[name];
    ElMessage.success(t("services.message.removed"));
  } catch (e) {
    ElMessage.error({ message: friendlyError(e, t("common.remove")), icon: Warning });
  } finally {
    tableLoading.value = false;
  }
}

function openEditDialog(svc: ManagedService) {
  editForm.value = { ...svc };
  editDialogVisible.value = true;
}

async function saveEdit() {
  managedServices.value = await invoke("update_managed_service", {
    name: editForm.value.name,
    displayName: editForm.value.display_name,
    enabled: editForm.value.enabled,
  });
  editDialogVisible.value = false;
  ElMessage.success(t("services.message.updated"));
}

async function startSvc(name: string) {
  tableLoadingText.value = t("common.loading");
  tableLoading.value = true;
  try {
    await invoke("start_svc", { name });
    ElMessage.success(t("services.message.startSent"));
    await refreshStatuses();
  } catch (e) {
    ElMessage.error({ message: friendlyError(e, t("services.start")), icon: Warning });
  } finally {
    tableLoading.value = false;
  }
}

async function stopSvc(name: string) {
  tableLoadingText.value = t("common.loading");
  tableLoading.value = true;
  try {
    await invoke("stop_svc", { name });
    ElMessage.success(t("services.message.stopSent"));
    await refreshStatuses();
  } catch (e) {
    ElMessage.error({ message: friendlyError(e, t("services.stop")), icon: Warning });
  } finally {
    tableLoading.value = false;
  }
}

function statusType(status: string) {
  if (status === "Running") return "success";
  if (status === "Stopped") return "danger";
  return "warning";
}

function statusIcon(status: string) {
  if (status === "Running") return VideoPlay;
  if (status === "Stopped") return VideoPause;
  return Refresh;
}

function statusText(status: string) {
  const key = `services.statusType.${status}`;
  const translated = t(key);
  return translated !== key ? translated : status;
}

function startTypeText(startType: string) {
  const key = `services.startTypeValue.${startType}`;
  const translated = t(key);
  return translated !== key ? translated : startType;
}

onMounted(loadManaged);
</script>

<template>
  <div class="service-page">
    <el-card shadow="hover" class="service-card">
      <template #header>
        <div class="card-header">
          <div class="card-header-left">
            <span class="card-title">{{ t("services.title") }}</span>
            <el-tag type="info" size="small" class="service-count">
              {{ t("services.totalCount", { count: managedServices.length }) }}
            </el-tag>
          </div>
          <div class="card-header-right">
            <el-button :icon="Refresh" @click="refreshStatuses" :loading="refreshing" :disabled="tableLoading">
              {{ t("services.refreshStatus") }}
            </el-button>
            <el-button type="primary" :icon="Plus" @click="openAddDialog" :disabled="tableLoading">
              {{ t("services.addService") }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty
        v-if="managedServices.length === 0"
        :description="t('services.emptyText')"
        :image-size="120"
      >
        <el-button type="primary" :icon="Plus" @click="openAddDialog">{{ t("services.addService") }}</el-button>
      </el-empty>

      <el-table
        v-else
        :data="managedServices"
        stripe
        highlight-current-row
        class="service-table"
        v-loading="tableLoading"
        :element-loading-text="tableLoadingText"
      >
        <el-table-column prop="name" :label="t('services.serviceName')" min-width="180">
          <template #default="{ row }">
            <span class="service-name">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="display_name" :label="t('services.displayName')" min-width="200" />
        <el-table-column :label="t('services.status')" width="140" align="center">
          <template #default="{ row }">
            <el-tag
              :type="statusType(serviceStatuses[row.name] || '')"
              :icon="statusIcon(serviceStatuses[row.name] || '')"
              effect="dark"
              size="default"
            >
              {{ serviceStatuses[row.name] ? statusText(serviceStatuses[row.name]) : t("common.loading") }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('services.action')" width="180" fixed="right" align="center">
          <template #default="{ row }">
            <div class="action-buttons">
              <el-tooltip :content="t('services.start')" placement="top">
                <el-button
                  type="success"
                  :icon="VideoPlay"
                  size="small"
                  circle
                  :disabled="serviceStatuses[row.name] === 'Running'"
                  @click="startSvc(row.name)"
                />
              </el-tooltip>
              <el-tooltip :content="t('services.stop')" placement="top">
                <el-button
                  type="warning"
                  :icon="VideoPause"
                  size="small"
                  circle
                  :disabled="serviceStatuses[row.name] === 'Stopped'"
                  @click="stopSvc(row.name)"
                />
              </el-tooltip>
              <el-tooltip :content="t('common.edit')" placement="top">
                <el-button
                  type="info"
                  :icon="EditPen"
                  size="small"
                  circle
                  @click="openEditDialog(row)"
                />
              </el-tooltip>
              <el-tooltip :content="t('common.remove')" placement="top">
                <el-button
                  type="danger"
                  :icon="Delete"
                  size="small"
                  circle
                  @click="removeService(row.name)"
                />
              </el-tooltip>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="showDialog" :title="t('services.addDialog.title')" width="720px" top="6vh" destroy-on-close>
      <div class="dialog-search">
        <el-input
          v-model="searchQuery"
          :placeholder="t('services.addDialog.searchPlaceholder')"
          clearable
          :prefix-icon="Refresh"
          size="large"
        />
      </div>
      <el-table
        :data="filteredServices"
        height="420px"
        v-loading="loading"
        @row-click="addService"
        highlight-current-row
        class="add-service-table"
      >
        <el-table-column prop="name" :label="t('services.serviceName')" min-width="180">
          <template #default="{ row }">
            <span class="service-name">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="display_name" :label="t('services.displayName')" min-width="220" />
        <el-table-column prop="status" :label="t('services.status')" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small" effect="plain">
              {{ statusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="start_type" :label="t('services.startType')" width="120" align="center">
          <template #default="{ row }">
            {{ startTypeText(row.start_type) }}
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>

    <el-dialog v-model="editDialogVisible" :title="t('services.editDialog.title')" width="440px" destroy-on-close>
      <el-form label-width="90px" class="edit-form">
        <el-form-item :label="t('services.editDialog.serviceName')">
          <el-input :model-value="editForm.name" disabled />
        </el-form-item>
        <el-form-item :label="t('services.editDialog.displayName')">
          <el-input v-model="editForm.display_name" />
        </el-form-item>
        <el-form-item :label="t('services.editDialog.enableMonitor')">
          <el-switch v-model="editForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">{{ t("common.cancel") }}</el-button>
        <el-button type="primary" @click="saveEdit">{{ t("common.save") }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.service-page {
  padding: 4px;
}

.service-card {
  border-radius: 8px;
}

.service-card :deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.card-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.service-count {
  vertical-align: middle;
}

.service-table {
  width: 100%;
}

.service-name {
  font-family: monospace;
  font-weight: 500;
  color: var(--el-color-primary);
}

.action-buttons {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.dialog-search {
  margin-bottom: 16px;
}

.add-service-table {
  cursor: pointer;
  border-radius: 6px;
}

.edit-form {
  padding: 10px 20px 0 0;
}
</style>
