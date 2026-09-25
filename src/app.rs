use app_service::{ManagedService, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::backend::BackendError;
use crate::cleanup::{CleanupCategory, DeleteMode, Scan};
use crate::i18n::{Language, Translations, EN, ZH};
use crate::theme::{ThemeColors, DARK, LIGHT};

pub use crate::backend::SystemInfo;

const TOOLS_COUNT: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Service,
    Tools,
    Cleanup,
    Scripts,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub enum PendingAction {
    StartService(String),
    StopService(String),
    StartAll(Vec<String>),
    StopAll(Vec<String>),
    RefreshAll(Vec<String>),
}

#[derive(Debug, Default)]
pub struct AddDialogState {
    show: bool,
    loading: bool,
    services: Vec<ServiceInfo>,
    filtered: Vec<usize>,
    selected: usize,
    search: SearchText,
    error: Option<String>,
}

/// Editable text with an optional IME composition segment. Shared by the
/// add-service dialog and the main service list filter.
#[derive(Debug, Default)]
struct SearchText {
    text: String,
    /// Text currently being composed by the platform IME. It is displayed but
    /// not committed to `text` until the IME reports a result.
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

/// Which managed services the main list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceFilter {
    All,
    Running,
    Stopped,
    Pending,
}

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

/// One cleanable location with its selection and last scan result.
#[derive(Debug, Clone, Copy)]
pub struct CleanupRow {
    pub category: CleanupCategory,
    pub selected: bool,
    pub scan: Option<Scan>,
}

/// Junk cleanup page state.
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

pub struct AppState {
    current_view: View,
    managed_services: Vec<ManagedService>,
    service_statuses: HashMap<String, ServiceStatus>,
    /// Last status confirmed by the backend, so a failed operation can fall
    /// back to it instead of showing `Unknown`.
    service_known_statuses: HashMap<String, ServiceStatus>,
    service_messages: HashMap<String, String>,
    data_file: PathBuf,
    settings_file: PathBuf,
    language: Language,
    theme: Theme,
    tools_selected: usize,
    scripts_selected: usize,
    script_result: Option<String>,
    tool_detail_active: bool,
    system_info: Option<SystemInfo>,
    system_info_loading: bool,
    operation_state: OperationState,
    add_dialog: AddDialogState,
    /// Free-text filter and status filter for the main service list.
    service_search: SearchText,
    service_filter: ServiceFilter,
    cleanup: CleanupState,
    /// Transient message shown after a refresh completes.
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
        let managed_services = app_service::load_managed_services(&data_file);
        let settings = load_settings(&settings_file);
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
            script_result: None,
            tool_detail_active: false,
            system_info: None,
            system_info_loading: false,
            operation_state: OperationState::Idle,
            add_dialog: AddDialogState::default(),
            service_search: SearchText::default(),
            service_filter: ServiceFilter::All,
            cleanup: CleanupState::default(),
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
        }
    }

    pub fn managed_services(&self) -> &[ManagedService] {
        &self.managed_services
    }

    pub fn service_status(&self, name: &str) -> ServiceStatus {
        self.service_statuses
            .get(name)
            .copied()
            .unwrap_or(ServiceStatus::Unknown)
    }

    pub fn service_message(&self, name: &str) -> Option<&str> {
        self.service_messages.get(name).map(String::as_str)
    }

    pub fn service_search(&self) -> &str {
        self.service_search.text.as_str()
    }

    pub fn service_search_marked(&self) -> &str {
        self.service_search.marked.as_str()
    }

    pub fn service_filter(&self) -> ServiceFilter {
        self.service_filter
    }

    pub fn set_service_filter(&mut self, filter: ServiceFilter) {
        self.service_filter = filter;
    }

    pub fn service_search_text_utf16_len(&self) -> usize {
        self.service_search.utf16_len()
    }

    pub fn service_search_marked_range(&self) -> Option<Range<usize>> {
        self.service_search.marked_range()
    }

    pub fn service_search_text_range(&self, range: Range<usize>) -> String {
        self.service_search.text_range(range)
    }

    pub fn service_search_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.service_search.replace_range(range, text);
    }

    pub fn service_search_commit_text(&mut self, text: &str) {
        self.service_search.commit(text);
    }

    pub fn service_search_set_marked(&mut self, text: &str) {
        self.service_search.set_marked(text);
    }

    pub fn service_search_unmark(&mut self) {
        self.service_search.unmark();
    }

    pub fn service_search_backspace(&mut self) {
        self.service_search.backspace();
    }

    /// Indices into `managed_services` that pass the current search and status
    /// filter, in list order.
    pub fn filtered_service_indices(&self) -> Vec<usize> {
        let search = self.service_search.text.to_lowercase();
        self.managed_services
            .iter()
            .enumerate()
            .filter(|(_, service)| {
                let matches_search = search.is_empty()
                    || service.name.to_lowercase().contains(&search)
                    || service.display_name.to_lowercase().contains(&search);
                matches_search
                    && self
                        .service_filter
                        .matches(self.service_status(&service.name))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn cleanup(&self) -> &CleanupState {
        &self.cleanup
    }

    pub fn toggle_cleanup_row(&mut self, index: usize) {
        if let Some(row) = self.cleanup.rows.get_mut(index) {
            row.selected = !row.selected;
        }
    }

    pub fn set_cleanup_delete_mode(&mut self, mode: DeleteMode) {
        self.cleanup.delete_mode = mode;
    }

    pub fn begin_cleanup_scan(&mut self) {
        self.cleanup.scanning = true;
        self.cleanup.message = None;
        self.operation_state = OperationState::ScanningCleanup;
    }

    pub fn apply_cleanup_scan(
        &mut self,
        scans: Vec<(CleanupCategory, Scan)>,
        recycle_bin: Option<(u64, u64)>,
    ) {
        for (category, scan) in scans {
            if let Some(row) = self
                .cleanup
                .rows
                .iter_mut()
                .find(|row| row.category == category)
            {
                row.scan = Some(scan);
            }
        }
        self.cleanup.recycle_bin = recycle_bin;
        self.cleanup.scanning = false;
        self.cleanup.cleaning = false;
        self.operation_state = OperationState::Idle;
    }

    pub fn begin_cleanup(&mut self) {
        self.cleanup.cleaning = true;
        self.cleanup.message = None;
        self.operation_state = OperationState::CleaningCleanup;
    }

    pub fn set_cleanup_message(&mut self, message: String) {
        self.cleanup.message = Some(message);
    }

    pub fn apply_cleanup_result(
        &mut self,
        scans: Vec<(CleanupCategory, Scan)>,
        recycle_bin: Option<(u64, u64)>,
        message: String,
    ) {
        for (category, scan) in scans {
            if let Some(row) = self
                .cleanup
                .rows
                .iter_mut()
                .find(|row| row.category == category)
            {
                row.scan = Some(scan);
            }
        }
        self.cleanup.recycle_bin = recycle_bin;
        self.cleanup.message = Some(message);
        self.cleanup.scanning = false;
        self.cleanup.cleaning = false;
        self.operation_state = OperationState::Idle;
    }

    pub fn selected_cleanup_categories(&self) -> Vec<CleanupCategory> {
        self.cleanup
            .rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.category)
            .collect()
    }

    /// Combined size/count of the currently selected categories.
    pub fn selected_cleanup_totals(&self) -> Scan {
        let mut total = Scan::default();
        for row in &self.cleanup.rows {
            if !row.selected {
                continue;
            }
            if let Some(scan) = row.scan {
                total.bytes += scan.bytes;
                total.files += scan.files;
            }
        }
        total
    }

    pub fn request_refresh(&mut self) -> PendingAction {
        for service in &self.managed_services {
            self.service_statuses
                .insert(service.name.clone(), ServiceStatus::Refreshing);
        }
        self.operation_state = OperationState::Refreshing;
        PendingAction::RefreshAll(self.service_names())
    }

    pub fn request_start_service(&mut self, name: &str) -> Option<PendingAction> {
        let name = self
            .managed_services
            .iter()
            .find(|service| service.name == name)?
            .name
            .clone();
        self.service_statuses
            .insert(name.clone(), ServiceStatus::Starting);
        self.operation_state = OperationState::Starting;
        Some(PendingAction::StartService(name))
    }

    pub fn request_stop_service(&mut self, name: &str) -> Option<PendingAction> {
        let name = self
            .managed_services
            .iter()
            .find(|service| service.name == name)?
            .name
            .clone();
        self.service_statuses
            .insert(name.clone(), ServiceStatus::Stopping);
        self.operation_state = OperationState::Stopping;
        Some(PendingAction::StopService(name))
    }

    pub fn request_start_all(&mut self) -> PendingAction {
        let names = self.service_names();
        for name in &names {
            self.service_statuses
                .insert(name.clone(), ServiceStatus::Starting);
        }
        self.operation_state = OperationState::Starting;
        PendingAction::StartAll(names)
    }

    pub fn request_stop_all(&mut self) -> PendingAction {
        let names = self.service_names();
        for name in &names {
            self.service_statuses
                .insert(name.clone(), ServiceStatus::Stopping);
        }
        self.operation_state = OperationState::Stopping;
        PendingAction::StopAll(names)
    }

    pub fn apply_service_success(&mut self, name: &str, status: &str) {
        let status = ServiceStatus::from_backend(status);
        self.service_statuses.insert(name.to_string(), status);
        self.service_known_statuses.insert(name.to_string(), status);
        self.service_messages.remove(name);
    }

    pub fn apply_service_error(&mut self, name: &str, error: &BackendError) {
        // Keep the last confirmed status so a failed start/stop does not make
        // the row look unknown; the error is surfaced through the message.
        let status = self
            .service_known_statuses
            .get(name)
            .copied()
            .unwrap_or(ServiceStatus::Unknown);
        self.service_statuses.insert(name.to_string(), status);
        self.service_messages
            .insert(name.to_string(), map_error(error, self.language));
        self.operation_state = OperationState::Error;
    }

    pub fn set_operation_state(&mut self, state: OperationState) {
        self.operation_state = state;
    }

    pub fn operation_state(&self) -> OperationState {
        self.operation_state
    }

    /// Transient "refreshed N services" message, if one is currently showing.
    pub fn refresh_notice(&self) -> Option<&str> {
        self.refresh_notice.as_deref()
    }

    pub fn set_refresh_notice(&mut self, message: String) {
        self.refresh_notice = Some(message);
    }

    pub fn clear_refresh_notice(&mut self) {
        self.refresh_notice = None;
    }

    pub fn remove_service(&mut self, name: &str) {
        let Some(index) = self
            .managed_services
            .iter()
            .position(|service| service.name == name)
        else {
            return;
        };

        let name = self.managed_services.remove(index).name;
        self.service_statuses.remove(&name);
        self.service_known_statuses.remove(&name);
        self.service_messages.remove(&name);
        app_service::save_managed_services(&self.data_file, &self.managed_services);
    }

    pub fn begin_add_dialog(&mut self) {
        self.add_dialog = AddDialogState {
            show: true,
            loading: true,
            ..AddDialogState::default()
        };
        self.operation_state = OperationState::LoadingServices;
    }

    pub fn set_add_dialog_services(&mut self, result: Result<Vec<ServiceInfo>, BackendError>) {
        match result {
            Ok(services) => {
                self.add_dialog.services = services;
                self.add_dialog.loading = false;
                self.add_dialog.error = None;
                self.rebuild_add_dialog_filter();
                self.operation_state = OperationState::Idle;
            }
            Err(error) => {
                self.add_dialog.loading = false;
                self.add_dialog.error = Some(map_error(&error, self.language));
                self.operation_state = OperationState::Error;
            }
        }
    }

    pub fn show_add_dialog(&self) -> bool {
        self.add_dialog.show
    }

    pub fn add_dialog_loading(&self) -> bool {
        self.add_dialog.loading
    }

    pub fn add_dialog_error(&self) -> Option<&str> {
        self.add_dialog.error.as_deref()
    }

    pub fn add_dialog_services(&self) -> &[ServiceInfo] {
        &self.add_dialog.services
    }

    pub fn add_dialog_filtered(&self) -> &[usize] {
        &self.add_dialog.filtered
    }

    pub fn add_dialog_selected(&self) -> usize {
        self.add_dialog.selected
    }

    pub fn add_dialog_search(&self) -> &str {
        self.add_dialog.search.text.as_str()
    }

    /// Text being composed by the IME, rendered after the committed search text.
    pub fn add_dialog_marked(&self) -> &str {
        self.add_dialog.search.marked.as_str()
    }

    /// Total length of the editable text in UTF-16 units, matching the
    /// platform input protocol.
    pub fn add_dialog_text_utf16_len(&self) -> usize {
        self.add_dialog.search.utf16_len()
    }

    /// The UTF-16 range occupied by the IME composition, if any.
    pub fn add_dialog_marked_range(&self) -> Option<Range<usize>> {
        self.add_dialog.search.marked_range()
    }

    pub fn add_dialog_text_range(&self, range: Range<usize>) -> String {
        self.add_dialog.search.text_range(range)
    }

    /// Replace a UTF-16 range of the editable text, clearing any composition.
    pub fn add_dialog_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.add_dialog.search.replace_range(range, text);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    /// Commit a chunk of text (typed character or IME result) at the cursor.
    pub fn add_dialog_commit_text(&mut self, text: &str) {
        self.add_dialog.search.commit(text);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    /// Begin or update an IME composition.
    pub fn add_dialog_set_marked(&mut self, text: &str) {
        self.add_dialog.search.set_marked(text);
    }

    /// Commit any pending composition.
    pub fn add_dialog_unmark(&mut self) {
        self.add_dialog.search.unmark();
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn close_add_dialog(&mut self) {
        self.add_dialog = AddDialogState::default();
        if self.operation_state == OperationState::LoadingServices {
            self.operation_state = OperationState::Idle;
        }
    }

    pub fn add_dialog_backspace(&mut self) {
        self.add_dialog.search.backspace();
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn select_add_dialog(&mut self, index: usize) {
        if index < self.add_dialog.filtered.len() {
            self.add_dialog.selected = index;
        }
    }

    pub fn confirm_add_service(&mut self) -> bool {
        let Some(&real_index) = self.add_dialog.filtered.get(self.add_dialog.selected) else {
            return false;
        };
        let service = &self.add_dialog.services[real_index];
        if self
            .managed_services
            .iter()
            .any(|item| item.name == service.name)
        {
            self.close_add_dialog();
            return false;
        }

        self.managed_services.push(ManagedService {
            name: service.name.clone(),
            display_name: service.display_name.clone(),
            enabled: true,
        });
        app_service::save_managed_services(&self.data_file, &self.managed_services);
        self.close_add_dialog();
        true
    }

    pub fn t(&self) -> &'static Translations {
        translations_for(self.language)
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// Switch directly to a language, used by the settings segmented control.
    pub fn set_language(&mut self, language: Language) {
        if self.language != language {
            self.language = language;
            save_settings(&self.settings_file, self.language, self.theme);
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn theme_colors(&self) -> &'static ThemeColors {
        match self.theme {
            Theme::Dark => &DARK,
            Theme::Light => &LIGHT,
        }
    }

    /// Switch directly to a theme, used by the settings segmented control.
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            save_settings(&self.settings_file, self.language, self.theme);
        }
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }

    pub fn select_tool(&mut self, index: usize) {
        if index < TOOLS_COUNT {
            self.tools_selected = index;
        }
    }

    pub fn scripts_selected(&self) -> usize {
        self.scripts_selected
    }

    pub fn tool_detail_active(&self) -> bool {
        self.tool_detail_active
    }

    pub fn open_tool_detail(&mut self) -> bool {
        if self.tools_selected != 0 {
            return false;
        }
        self.tool_detail_active = true;
        if self.system_info.is_none() {
            self.system_info_loading = true;
            self.operation_state = OperationState::LoadingSystemInfo;
        }
        true
    }

    pub fn close_tool_detail(&mut self) {
        self.tool_detail_active = false;
    }

    pub fn system_info(&self) -> Option<&SystemInfo> {
        self.system_info.as_ref()
    }

    pub fn system_info_loading(&self) -> bool {
        self.system_info_loading
    }

    /// Re-run the system information fetch, showing the loading state again.
    pub fn request_system_info_refresh(&mut self) {
        self.system_info = None;
        self.system_info_loading = true;
        self.operation_state = OperationState::LoadingSystemInfo;
    }

    pub fn set_system_info(&mut self, result: Result<SystemInfo, BackendError>) {
        self.system_info_loading = false;
        match result {
            Ok(info) => {
                self.system_info = Some(info);
                self.operation_state = OperationState::Idle;
            }
            Err(_) => {
                self.system_info = None;
                self.operation_state = OperationState::Error;
            }
        }
    }

    pub fn begin_script(&mut self) {
        self.script_result = None;
        self.operation_state = OperationState::RunningScript;
    }

    pub fn script_result(&self) -> Option<&str> {
        self.script_result.as_deref()
    }

    pub fn set_script_result(&mut self, result: Result<u32, BackendError>) {
        self.script_result = Some(match result {
            Ok(deleted) => self
                .t()
                .script_result_cleanup
                .replace("{}", &deleted.to_string()),
            Err(error) => map_error(&error, self.language),
        });
        self.operation_state = OperationState::Idle;
    }

    fn service_names(&self) -> Vec<String> {
        self.managed_services
            .iter()
            .map(|service| service.name.clone())
            .collect()
    }

    fn rebuild_add_dialog_filter(&mut self) {
        let search = self.add_dialog.search.text.to_lowercase();
        self.add_dialog.filtered = self
            .add_dialog
            .services
            .iter()
            .enumerate()
            .filter(|(_, service)| {
                search.is_empty()
                    || service.name.to_lowercase().contains(&search)
                    || service.display_name.to_lowercase().contains(&search)
            })
            .map(|(index, _)| index)
            .collect();
    }
}

fn translations_for(language: Language) -> &'static Translations {
    match language {
        Language::Chinese => &ZH,
        Language::English => &EN,
    }
}

fn map_error(error: &BackendError, language: Language) -> String {
    let translations = translations_for(language);
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

/// Number of UTF-16 code units, which is what the platform input protocol uses.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Convert a UTF-16 offset into a byte offset, clamping to the end of the string.
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

#[derive(Debug, Serialize, Deserialize)]
struct AppSettings {
    language: Language,
    theme: Theme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Chinese,
            theme: Theme::Dark,
        }
    }
}

fn load_settings(path: &Path) -> AppSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn save_settings(path: &Path, language: Language, theme: Theme) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&AppSettings { language, theme }) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_operation_is_idle() {
        assert_eq!(AppState::new().operation_state(), OperationState::Idle);
    }

    #[test]
    fn backend_status_is_typed() {
        assert_eq!(
            ServiceStatus::from_backend("Running"),
            ServiceStatus::Running
        );
        assert_eq!(
            ServiceStatus::from_backend("Stopped"),
            ServiceStatus::Stopped
        );
        assert_eq!(
            ServiceStatus::from_backend("unexpected"),
            ServiceStatus::Unknown
        );
    }

    #[test]
    fn only_system_tool_can_be_selected_and_opened() {
        let mut state = AppState::new();

        state.select_tool(1);

        assert_eq!(state.tools_selected(), 0);
        assert!(state.open_tool_detail());
    }

    #[test]
    fn dialog_filter_matches_name_and_display_name() {
        let mut state = AppState::new();
        state.begin_add_dialog();
        state.set_add_dialog_services(Ok(vec![
            ServiceInfo {
                name: "AlphaSvc".into(),
                display_name: "Alpha Display".into(),
                status: "Stopped".into(),
                start_type: "Automatic".into(),
                description: String::new(),
            },
            ServiceInfo {
                name: "BetaSvc".into(),
                display_name: "Beta Display".into(),
                status: "Running".into(),
                start_type: "Manual".into(),
                description: String::new(),
            },
        ]));
        state.add_dialog_commit_text("b");
        assert_eq!(state.add_dialog_filtered(), &[1]);
    }

    fn add_managed(state: &mut AppState, name: &str, display_name: &str) {
        state.managed_services.push(ManagedService {
            name: name.into(),
            display_name: display_name.into(),
            enabled: true,
        });
    }

    #[test]
    fn service_list_filters_by_search_and_status() {
        let mut state = AppState::new();
        add_managed(&mut state, "Redis", "Redis Server");
        add_managed(&mut state, "MySQL", "MySQL");
        state.apply_service_success("Redis", "Running");
        state.apply_service_success("MySQL", "Stopped");

        assert_eq!(state.filtered_service_indices(), vec![0, 1]);

        state.service_search_commit_text("red");
        assert_eq!(state.filtered_service_indices(), vec![0]);

        state.service_search_backspace();
        state.service_search_backspace();
        state.service_search_backspace();
        assert_eq!(state.filtered_service_indices(), vec![0, 1]);

        state.set_service_filter(ServiceFilter::Running);
        assert_eq!(state.filtered_service_indices(), vec![0]);
        state.set_service_filter(ServiceFilter::Stopped);
        assert_eq!(state.filtered_service_indices(), vec![1]);
        state.set_service_filter(ServiceFilter::Pending);
        assert!(state.filtered_service_indices().is_empty());
    }

    #[test]
    fn failed_operation_keeps_last_known_status() {
        let mut state = AppState::new();
        add_managed(&mut state, "Svc", "Svc");
        assert_eq!(state.service_status("Svc"), ServiceStatus::Unknown);

        state.apply_service_success("Svc", "Running");
        assert_eq!(state.service_status("Svc"), ServiceStatus::Running);

        state.apply_service_error("Svc", &BackendError::Service("access is denied".into()));
        assert_eq!(state.service_status("Svc"), ServiceStatus::Running);
        assert!(state.service_message("Svc").is_some());
    }

    #[test]
    fn dialog_selection_accepts_a_filtered_index() {
        let mut state = AppState::new();
        state.begin_add_dialog();
        state.set_add_dialog_services(Ok(vec![
            ServiceInfo {
                name: "AlphaSvc".into(),
                display_name: "Alpha Display".into(),
                status: "Stopped".into(),
                start_type: "Automatic".into(),
                description: String::new(),
            },
            ServiceInfo {
                name: "BetaSvc".into(),
                display_name: "Beta Display".into(),
                status: "Running".into(),
                start_type: "Manual".into(),
                description: String::new(),
            },
        ]));

        state.select_add_dialog(1);

        assert_eq!(state.add_dialog_selected(), 1);
    }
}
