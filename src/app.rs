use app_service::{ManagedService, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::backend::BackendError;
use crate::i18n::{Language, Translations, EN, ZH};
use crate::theme::{ThemeColors, DARK, LIGHT};

pub use crate::backend::SystemInfo;

const SETTINGS_COUNT: usize = 2;
const TOOLS_COUNT: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Service,
    Tools,
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
    search: String,
    error: Option<String>,
}

pub struct AppState {
    current_view: View,
    managed_services: Vec<ManagedService>,
    service_statuses: HashMap<String, ServiceStatus>,
    service_messages: HashMap<String, String>,
    data_file: PathBuf,
    settings_file: PathBuf,
    language: Language,
    theme: Theme,
    selected_service: Option<usize>,
    settings_selected: usize,
    tools_selected: usize,
    scripts_selected: usize,
    script_result: Option<String>,
    tool_detail_active: bool,
    system_info: Option<SystemInfo>,
    system_info_loading: bool,
    operation_state: OperationState,
    add_dialog: AddDialogState,
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
        let selected_service = (!managed_services.is_empty()).then_some(0);

        Self {
            current_view: View::Service,
            managed_services,
            service_statuses: HashMap::new(),
            service_messages: HashMap::new(),
            data_file,
            settings_file,
            language: settings.language,
            theme: settings.theme,
            selected_service,
            settings_selected: 0,
            tools_selected: 0,
            scripts_selected: 0,
            script_result: None,
            tool_detail_active: false,
            system_info: None,
            system_info_loading: false,
            operation_state: OperationState::Idle,
            add_dialog: AddDialogState::default(),
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

    pub fn selected_service(&self) -> Option<usize> {
        self.selected_service
    }

    pub fn select_service(&mut self, index: usize) {
        if index < self.managed_services.len() {
            self.selected_service = Some(index);
        }
    }

    pub fn request_refresh(&mut self) -> PendingAction {
        for service in &self.managed_services {
            self.service_statuses
                .insert(service.name.clone(), ServiceStatus::Refreshing);
        }
        self.operation_state = OperationState::Refreshing;
        PendingAction::RefreshAll(self.service_names())
    }

    pub fn request_start_selected(&mut self) -> Option<PendingAction> {
        let name = self.selected_service_name()?;
        self.service_statuses
            .insert(name.clone(), ServiceStatus::Starting);
        self.operation_state = OperationState::Starting;
        Some(PendingAction::StartService(name))
    }

    pub fn request_stop_selected(&mut self) -> Option<PendingAction> {
        let name = self.selected_service_name()?;
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
        self.service_statuses
            .insert(name.to_string(), ServiceStatus::from_backend(status));
        self.service_messages.remove(name);
    }

    pub fn apply_service_error(&mut self, name: &str, error: &BackendError) {
        self.service_statuses
            .insert(name.to_string(), ServiceStatus::Unknown);
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

    pub fn remove_selected_service(&mut self) {
        let Some(index) = self.selected_service else {
            return;
        };
        if index >= self.managed_services.len() {
            return;
        }

        let name = self.managed_services.remove(index).name;
        self.service_statuses.remove(&name);
        self.service_messages.remove(&name);
        app_service::save_managed_services(&self.data_file, &self.managed_services);

        self.selected_service = if self.managed_services.is_empty() {
            None
        } else {
            Some(index.min(self.managed_services.len() - 1))
        };
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
        &self.add_dialog.search
    }

    pub fn close_add_dialog(&mut self) {
        self.add_dialog = AddDialogState::default();
        if self.operation_state == OperationState::LoadingServices {
            self.operation_state = OperationState::Idle;
        }
    }

    pub fn add_dialog_input(&mut self, character: char) {
        self.add_dialog.search.push(character);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn add_dialog_backspace(&mut self) {
        self.add_dialog.search.pop();
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
        if self.selected_service.is_none() {
            self.selected_service = Some(0);
        }
        self.close_add_dialog();
        true
    }

    pub fn t(&self) -> &'static Translations {
        translations_for(self.language)
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn cycle_language(&mut self) {
        self.language = self.language.next();
        save_settings(&self.settings_file, self.language, self.theme);
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

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
        save_settings(&self.settings_file, self.language, self.theme);
    }

    pub fn settings_selected(&self) -> usize {
        self.settings_selected
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

    pub fn select_setting(&mut self, index: usize) {
        if index < SETTINGS_COUNT {
            self.settings_selected = index;
        }
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

    fn selected_service_name(&self) -> Option<String> {
        self.selected_service
            .and_then(|index| self.managed_services.get(index))
            .map(|service| service.name.clone())
    }

    fn service_names(&self) -> Vec<String> {
        self.managed_services
            .iter()
            .map(|service| service.name.clone())
            .collect()
    }

    fn rebuild_add_dialog_filter(&mut self) {
        let search = self.add_dialog.search.to_lowercase();
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
    } else if detail.contains("notfound") || detail.contains("服务不存在") {
        translations.err_not_found.to_string()
    } else {
        translations.err_failed.to_string()
    }
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
        state.add_dialog_input('b');
        assert_eq!(state.add_dialog_filtered(), &[1]);
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
