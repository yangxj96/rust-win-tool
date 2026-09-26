//! 应用状态与功能域状态操作。

mod cleanup;
mod scripts;
mod services;
mod settings;
mod tools;

use crate::features::services::{ManagedService, ServiceDetails, ServiceInfo, StartType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::features::cleanup::{CleanupCategory, DeleteMode, Scan};
use crate::features::scripts::{CustomScript, ScriptKind};
use crate::features::tools::{
    monitor::Metrics, ports::PortEntry, processes::ProcessInfo, startup::StartupItem,
};
use crate::shared::BackendError;
use crate::ui::i18n::{Language, Translations, EN, ZH};
use crate::ui::theme::{ThemeColors, DARK, LIGHT};

pub use crate::features::tools::SystemInfo;

const TOOLS_COUNT: usize = 5;
const SCRIPTS_COUNT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// 界面主题；持久化值固定为 `dark` 和 `light`。
pub enum Theme {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 应用主窗口当前显示的一级页面。
pub enum View {
    Service,
    Tools,
    Cleanup,
    Scripts,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 服务在界面中显示的状态，包含操作期间的临时状态。
pub enum ServiceStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Refreshing,
    Unknown,
}

impl ServiceStatus {
    pub fn from_backend(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "startpending" | "start pending" => Self::Starting,
            "stoppending" | "stop pending" => Self::Stopping,
            "refreshing" => Self::Refreshing,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 当前应用操作，用于控制加载提示和控件禁用状态。
pub enum OperationState {
    Idle,
    LoadingServices,
    Refreshing,
    Starting,
    Stopping,
    LoadingSystemInfo,
    RunningScript,
    ScanningCleanup,
    CleaningCleanup,
    Error,
}

#[derive(Debug, Clone)]
/// 提交给后端执行的服务操作请求。
pub enum PendingAction {
    StartService(String),
    StopService(String),
    StartAll(Vec<String>),
    StopAll(Vec<String>),
    RefreshAll(Vec<String>),
}

#[derive(Debug, Default)]
/// “添加服务”对话框的搜索、选择和结果状态。
pub struct AddDialogState {
    show: bool,
    loading: bool,
    services: Vec<ServiceInfo>,
    filtered: Vec<usize>,
    selected: usize,
    search: SearchText,
    error: Option<String>,
}

/// 可编辑文本及可选的输入法组合文本，由添加服务对话框和服务列表筛选框共用。
#[derive(Debug, Default)]
struct SearchText {
    text: String,
    /// 输入法正在组合的文本。输入法提交结果前只显示，不并入 `text`。
    marked: String,
}

impl SearchText {
    fn full_text(&self) -> String {
        let mut full = self.text.clone();
        full.push_str(&self.marked);
        full
    }

    fn utf16_len(&self) -> usize {
        utf16_len(&self.text) + utf16_len(&self.marked)
    }

    fn marked_range(&self) -> Option<Range<usize>> {
        if self.marked.is_empty() {
            None
        } else {
            let start = utf16_len(&self.text);
            Some(start..start + utf16_len(&self.marked))
        }
    }

    fn text_range(&self, range: Range<usize>) -> String {
        let full = self.full_text();
        let start = byte_index_for_utf16(&full, range.start);
        let end = byte_index_for_utf16(&full, range.end);
        full.get(start..end).unwrap_or_default().to_string()
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        let full = self.full_text();
        let start = byte_index_for_utf16(&full, range.start);
        let end = byte_index_for_utf16(&full, range.end);
        let mut replaced = String::with_capacity(full.len() + text.len());
        replaced.push_str(&full[..start]);
        replaced.push_str(text);
        replaced.push_str(&full[end..]);
        self.text = replaced;
        self.marked.clear();
    }

    fn commit(&mut self, text: &str) {
        self.text.push_str(text);
        self.marked.clear();
    }

    fn set_marked(&mut self, text: &str) {
        self.marked = text.to_string();
    }

    fn unmark(&mut self) {
        if !self.marked.is_empty() {
            self.text.push_str(&std::mem::take(&mut self.marked));
        }
    }

    fn backspace(&mut self) {
        self.text.pop();
    }
}

/// 主服务列表采用的筛选条件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceFilter {
    All,
    Running,
    Stopped,
    Pending,
}

/// 主服务列表的排序方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceSort {
    Manual,
    Name,
    Status,
}

impl ServiceSort {
    pub fn next(self) -> Self {
        match self {
            Self::Manual => Self::Name,
            Self::Name => Self::Status,
            Self::Status => Self::Manual,
        }
    }
}

/// 进程列表的排序方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSort {
    Memory,
    Name,
    Pid,
}

/// 记录在当前会话历史中的用户操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAction {
    Start,
    Stop,
    StartAll,
    StopAll,
    Refresh,
    StartType,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub action: HistoryAction,
    pub service: String,
    pub ok: bool,
    pub message: String,
}

const HISTORY_LIMIT: usize = 100;

impl ServiceFilter {
    fn matches(self, status: ServiceStatus) -> bool {
        match self {
            Self::All => true,
            Self::Running => status == ServiceStatus::Running,
            Self::Stopped => matches!(status, ServiceStatus::Stopped | ServiceStatus::Unknown),
            Self::Pending => matches!(
                status,
                ServiceStatus::Starting | ServiceStatus::Stopping | ServiceStatus::Refreshing
            ),
        }
    }
}

/// 一个可清理位置及其勾选状态、最近一次扫描结果。
#[derive(Debug, Clone, Copy)]
pub struct CleanupRow {
    pub category: CleanupCategory,
    pub selected: bool,
    pub scan: Option<Scan>,
}

/// 添加脚本对话框状态：名称、解释器类型和多行脚本内容。
#[derive(Default)]
struct ScriptDialogState {
    name: SearchText,
    content: SearchText,
    kind: ScriptKind,
    error: Option<String>,
}

/// 启动项页面状态。
#[derive(Default)]
pub struct StartupState {
    pub items: Vec<StartupItem>,
    pub loading: bool,
    pub error: Option<String>,
}

/// 网络诊断页面状态。
#[derive(Default)]
pub struct NetState {
    host: SearchText,
    port: u16,
    result: Option<String>,
    loading: bool,
}

/// 实时监视器状态：最新采样值和供图表绘制的短期历史数据。
#[derive(Default)]
pub struct MonitorState {
    pub active: bool,
    pub latest: Option<Metrics>,
    pub cpu_history: Vec<f32>,
    pub memory_history: Vec<f32>,
}

/// 端口占用查询对话框状态。
#[derive(Default)]
struct PortLookupState {
    open: bool,
    query: SearchText,
    port: Option<u16>,
    loading: bool,
    error: Option<String>,
    results: Vec<PortEntry>,
    searched: bool,
}

const MONITOR_HISTORY: usize = 60;

/// 垃圾清理页面状态。
pub struct CleanupState {
    pub scanning: bool,
    pub cleaning: bool,
    pub rows: Vec<CleanupRow>,
    pub recycle_bin: Option<(u64, u64)>,
    pub delete_mode: DeleteMode,
    pub message: Option<String>,
}

impl Default for CleanupState {
    fn default() -> Self {
        Self {
            scanning: false,
            cleaning: false,
            rows: CleanupCategory::all()
                .iter()
                .map(|&category| CleanupRow {
                    category,
                    selected: !category.requires_admin(),
                    scan: None,
                })
                .collect(),
            recycle_bin: None,
            delete_mode: DeleteMode::Recycle,
            message: None,
        }
    }
}

/// 由 GPUI 主窗口共享、并负责持久化数据的应用模型。
///
/// 文件路径和序列化值与控制台版本保持兼容；异步操作结果通过此模型在界面
/// 线程中应用。
pub struct AppState {
    current_view: View,
    managed_services: Vec<ManagedService>,
    service_statuses: HashMap<String, ServiceStatus>,
    /// 后端最近一次确认的状态。操作失败时回退到此值，避免错误显示为“未知”。
    service_known_statuses: HashMap<String, ServiceStatus>,
    service_messages: HashMap<String, String>,
    data_file: PathBuf,
    settings_file: PathBuf,
    language: Language,
    theme: Theme,
    tools_selected: usize,
    scripts_selected: usize,
    script_output: Option<String>,
    script_running: bool,
    scripts_file: PathBuf,
    custom_scripts: Vec<CustomScript>,
    /// “添加脚本”对话框中正在输入的命令。
    script_dialog: Option<ScriptDialogState>,
    tool_detail_active: bool,
    system_info: Option<SystemInfo>,
    system_info_loading: bool,
    operation_state: OperationState,
    add_dialog: AddDialogState,
    /// 主服务列表的文本筛选和状态筛选条件。
    service_search: SearchText,
    service_filter: ServiceFilter,
    service_sort: ServiceSort,
    history: Vec<HistoryEntry>,
    cleanup: CleanupState,
    /// 当前详情浮层展示的服务。
    service_detail_name: Option<String>,
    service_detail: Option<ServiceDetails>,
    service_detail_loading: bool,
    service_detail_error: Option<String>,
    /// 进程管理器状态。
    processes: Vec<ProcessInfo>,
    processes_loading: bool,
    processes_error: Option<String>,
    process_sort: ProcessSort,
    port_lookup: PortLookupState,
    monitor: MonitorState,
    network: NetState,
    startup: StartupState,
    /// 刷新完成后短暂显示的提示信息。
    refresh_notice: Option<String>,
}
impl AppState {
    pub fn new() -> Self {
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("config");
        let _ = std::fs::create_dir_all(&app_dir);

        let data_file = app_dir.join("managed_services.json");
        let settings_file = app_dir.join("settings.json");
        let scripts_file = app_dir.join("scripts.json");
        let managed_services = crate::features::services::load_managed_services(&data_file);
        let custom_scripts = crate::features::scripts::load(&scripts_file);
        let settings = settings::load_settings(&settings_file);
        Self {
            current_view: View::Service,
            managed_services,
            service_statuses: HashMap::new(),
            service_known_statuses: HashMap::new(),
            service_messages: HashMap::new(),
            data_file,
            settings_file,
            language: settings.language,
            theme: settings.theme,
            tools_selected: 0,
            scripts_selected: 0,
            script_output: None,
            script_running: false,
            scripts_file,
            custom_scripts,
            script_dialog: None,
            tool_detail_active: false,
            system_info: None,
            system_info_loading: false,
            operation_state: OperationState::Idle,
            add_dialog: AddDialogState::default(),
            service_search: SearchText::default(),
            service_filter: ServiceFilter::All,
            service_sort: ServiceSort::Manual,
            history: Vec::new(),
            cleanup: CleanupState::default(),
            service_detail_name: None,
            service_detail: None,
            service_detail_loading: false,
            service_detail_error: None,
            processes: Vec::new(),
            processes_loading: false,
            processes_error: None,
            process_sort: ProcessSort::Memory,
            port_lookup: PortLookupState::default(),
            monitor: MonitorState::default(),
            network: NetState {
                host: SearchText::default(),
                port: 80,
                result: None,
                loading: false,
            },
            startup: StartupState::default(),
            refresh_notice: None,
        }
    }

    pub fn current_view(&self) -> View {
        self.current_view
    }

    pub fn set_view(&mut self, view: View) {
        self.current_view = view;
        if view != View::Tools {
            self.tool_detail_active = false;
            self.release_tool_data();
        }
        if view != View::Scripts {
            // 输出浮层属于脚本页面。用户切换到其他页面时将其关闭，避免浮层残留在其他
            // 页面上。
            self.script_output = None;
        }
    }

    /// 清除工具页数据，避免用户离开页面后仍继续采样。
    fn release_tool_data(&mut self) {
        self.monitor = MonitorState::default();
        self.system_info = None;
        self.system_info_loading = false;
        self.processes.clear();
        self.processes_loading = false;
        self.processes_error = None;
        self.network.result = None;
        self.network.loading = false;
        self.startup.items.clear();
        self.startup.loading = false;
        self.startup.error = None;
        self.port_lookup = PortLookupState::default();
    }

    pub fn set_operation_state(&mut self, state: OperationState) {
        self.operation_state = state;
    }

    pub fn operation_state(&self) -> OperationState {
        self.operation_state
    }

    /// 当前正在显示的“已刷新 N 个服务”临时提示（如果存在）。
    pub fn refresh_notice(&self) -> Option<&str> {
        self.refresh_notice.as_deref()
    }

    pub fn set_refresh_notice(&mut self, message: String) {
        self.refresh_notice = Some(message);
    }

    pub fn clear_refresh_notice(&mut self) {
        self.refresh_notice = None;
    }
}

fn map_error(error: &BackendError, language: Language) -> String {
    let translations = settings::translations_for(language);
    let detail = error.to_string().to_lowercase();
    if detail.contains("access is denied")
        || detail.contains("access denied")
        || detail.contains("权限")
    {
        translations.err_permission.to_string()
    } else if detail.contains("already running") || detail.contains("already started") {
        translations.err_running.to_string()
    } else if detail.contains("not started") || detail.contains("has not been started") {
        translations.err_not_started.to_string()
    } else if detail.contains("cannot be stopped") || detail.contains("can not be stopped") {
        translations.err_cannot_stop.to_string()
    } else if detail.contains("timeout") {
        translations.err_timeout.to_string()
    } else if detail.contains("notfound")
        || detail.contains("not found")
        || detail.contains("does not exist")
        || detail.contains("服务不存在")
    {
        translations.err_not_found.to_string()
    } else {
        translations.err_failed.to_string()
    }
}

/// 计算 UTF-16 代码单元数量，与平台输入协议使用的长度单位一致。

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// 将 UTF-16 偏移量转换为字节偏移量，并将超出范围的值限制在字符串末尾。
fn byte_index_for_utf16(text: &str, target: usize) -> usize {
    let mut count = 0;
    for (byte_index, character) in text.char_indices() {
        if count >= target {
            return byte_index;
        }
        count += character.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests;
