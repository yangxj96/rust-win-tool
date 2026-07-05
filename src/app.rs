use app_service::{ManagedService, ServiceError, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::i18n::{Language, Translations, EN, ZH};
use crate::theme::{ThemeColors, DARK, LIGHT};

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Service,
    Tools,
    Settings,
}

impl View {
    pub fn all() -> &'static [View] {
        &[View::Service, View::Tools, View::Settings]
    }

    pub fn index(&self) -> usize {
        match self {
            View::Service => 0,
            View::Tools => 1,
            View::Settings => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => View::Service,
            1 => View::Tools,
            2 => View::Settings,
            _ => View::Service,
        }
    }
}

pub enum PendingAction {
    StartService(String),
    StopService(String),
    RefreshAll,
}

pub struct App {
    current_view: View,
    managed_services: Vec<ManagedService>,
    service_statuses: std::collections::HashMap<String, String>,
    service_messages: std::collections::HashMap<String, String>,
    data_file: PathBuf,
    settings_file: PathBuf,
    language: Language,
    theme: String,
    selected_service: Option<usize>,
    settings_selected: usize,
    tools_selected: usize,
    pending_action: Option<PendingAction>,
    // 添加服务对话框
    show_add_dialog: bool,
    add_dialog_services: Vec<ServiceInfo>,
    add_dialog_filtered: Vec<usize>,
    add_dialog_selected: usize,
    add_dialog_search: String,
}

impl App {
    pub fn new() -> Self {
        let app_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust-win-tool");

        let data_file = app_dir.join("managed_services.json");
        let settings_file = app_dir.join("settings.json");

        let managed_services = app_service::load_managed_services(&data_file);
        let settings = load_settings(&settings_file);

        let selected = if managed_services.is_empty() {
            None
        } else {
            Some(0)
        };

        Self {
            current_view: View::Service,
            managed_services,
            service_statuses: std::collections::HashMap::new(),
            service_messages: std::collections::HashMap::new(),
            data_file,
            settings_file,
            language: settings.language,
            theme: settings.theme,
            selected_service: selected,
            settings_selected: 0,
            tools_selected: 0,
            pending_action: None,
            show_add_dialog: false,
            add_dialog_services: Vec::new(),
            add_dialog_filtered: Vec::new(),
            add_dialog_selected: 0,
            add_dialog_search: String::new(),
        }
    }

    // ========== 视图切换 ==========

    pub fn current_view(&self) -> &View {
        &self.current_view
    }

    pub fn prev_view(&mut self) {
        let views = View::all();
        let idx = self.current_view.index();
        let new_idx = if idx == 0 { views.len() - 1 } else { idx - 1 };
        self.current_view = View::from_index(new_idx);
    }

    pub fn next_view(&mut self) {
        let views = View::all();
        let idx = self.current_view.index();
        let new_idx = (idx + 1) % views.len();
        self.current_view = View::from_index(new_idx);
    }

    // ========== 服务列表 ==========

    pub fn managed_services(&self) -> &[ManagedService] {
        &self.managed_services
    }

    pub fn service_status(&self, name: &str) -> &str {
        self.service_statuses
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
    }

    pub fn service_message(&self, name: &str) -> &str {
        self.service_messages.get(name).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn selected_service(&self) -> Option<usize> {
        self.selected_service
    }

    pub fn refresh_statuses(&mut self) {
        let msg = self.t().status_refreshing.to_string();
        for svc in &self.managed_services {
            self.service_statuses
                .insert(svc.name.clone(), msg.clone());
        }
        self.pending_action = Some(PendingAction::RefreshAll);
    }

    pub fn remove_selected_service(&mut self) {
        if let Some(idx) = self.selected_service {
            if idx < self.managed_services.len() {
                let name = self.managed_services[idx].display_name.clone();
                self.managed_services.remove(idx);
                self.service_statuses.remove(&name);
                app_service::save_managed_services(&self.data_file, &self.managed_services);
                if self.managed_services.is_empty() {
                    self.selected_service = None;
                } else if idx >= self.managed_services.len() {
                    self.selected_service = Some(self.managed_services.len() - 1);
                }
            }
        }
    }

    pub fn start_selected_service(&mut self) {
        if let Some(idx) = self.selected_service {
            if idx < self.managed_services.len() {
                let name = self.managed_services[idx].name.clone();
                let msg = self.t().status_starting.to_string();
                self.service_statuses.insert(name.clone(), msg);
                self.pending_action = Some(PendingAction::StartService(name));
            }
        }
    }

    pub fn stop_selected_service(&mut self) {
        if let Some(idx) = self.selected_service {
            if idx < self.managed_services.len() {
                let name = self.managed_services[idx].name.clone();
                let msg = self.t().status_stopping.to_string();
                self.service_statuses.insert(name.clone(), msg);
                self.pending_action = Some(PendingAction::StopService(name));
            }
        }
    }

    pub fn take_pending_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    pub fn execute_pending(&mut self, action: PendingAction) {
        let lang = self.language;
        match action {
            PendingAction::StartService(name) => {
                match app_service::start_service(&name) {
                    Ok(()) => { self.service_messages.remove(&name); }
                    Err(e) => {
                        self.service_messages.insert(name.clone(), map_error(&e, lang));
                    }
                }
                let status = app_service::get_service_status(&name)
                    .unwrap_or_else(|_| "未知".to_string());
                self.service_statuses.insert(name, status);
            }
            PendingAction::StopService(name) => {
                match app_service::stop_service(&name) {
                    Ok(()) => { self.service_messages.remove(&name); }
                    Err(e) => {
                        self.service_messages.insert(name.clone(), map_error(&e, lang));
                    }
                }
                let status = app_service::get_service_status(&name)
                    .unwrap_or_else(|_| "未知".to_string());
                self.service_statuses.insert(name, status);
            }
            PendingAction::RefreshAll => {
                self.service_messages.clear();
                for svc in &self.managed_services {
                    let status = app_service::get_service_status(&svc.name)
                        .unwrap_or_else(|_| "未知".to_string());
                    self.service_statuses.insert(svc.name.clone(), status);
                }
            }
        }
    }

    // ========== 添加服务对话框 ==========

    pub fn show_add_dialog(&self) -> bool {
        self.show_add_dialog
    }

    pub fn add_dialog_services(&self) -> &[ServiceInfo] {
        &self.add_dialog_services
    }

    pub fn add_dialog_filtered(&self) -> &[usize] {
        &self.add_dialog_filtered
    }

    pub fn add_dialog_selected(&self) -> usize {
        self.add_dialog_selected
    }

    pub fn add_dialog_search(&self) -> &str {
        &self.add_dialog_search
    }

    pub fn open_add_dialog(&mut self) {
        if let Ok(services) = app_service::list_all_services() {
            self.add_dialog_services = services;
            self.add_dialog_search.clear();
            self.add_dialog_selected = 0;
            self.rebuild_add_dialog_filter();
            self.show_add_dialog = true;
        }
    }

    pub fn close_add_dialog(&mut self) {
        self.show_add_dialog = false;
        self.add_dialog_services.clear();
        self.add_dialog_filtered.clear();
        self.add_dialog_search.clear();
        self.add_dialog_selected = 0;
    }

    pub fn add_dialog_input(&mut self, c: char) {
        self.add_dialog_search.push(c);
        self.add_dialog_selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn add_dialog_backspace(&mut self) {
        self.add_dialog_search.pop();
        self.add_dialog_selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn add_dialog_select_prev(&mut self) {
        let len = self.add_dialog_filtered.len();
        if len == 0 {
            return;
        }
        self.add_dialog_selected = if self.add_dialog_selected == 0 {
            len - 1
        } else {
            self.add_dialog_selected - 1
        };
    }

    pub fn add_dialog_select_next(&mut self) {
        let len = self.add_dialog_filtered.len();
        if len == 0 {
            return;
        }
        self.add_dialog_selected = (self.add_dialog_selected + 1) % len;
    }

    pub fn confirm_add_service(&mut self) {
        if self.add_dialog_selected < self.add_dialog_filtered.len() {
            let real_idx = self.add_dialog_filtered[self.add_dialog_selected];
            let svc = &self.add_dialog_services[real_idx];
            let name = svc.name.clone();

            if !self.managed_services.iter().any(|s| s.name == name) {
                self.managed_services.push(ManagedService {
                    name,
                    display_name: svc.display_name.clone(),
                    enabled: true,
                });
                app_service::save_managed_services(&self.data_file, &self.managed_services);
                if self.selected_service.is_none() {
                    self.selected_service = Some(0);
                }
            }
            self.close_add_dialog();
        }
    }

    fn rebuild_add_dialog_filter(&mut self) {
        let search = self.add_dialog_search.to_lowercase();
        self.add_dialog_filtered = self
            .add_dialog_services
            .iter()
            .enumerate()
            .filter(|(_, svc)| {
                search.is_empty()
                    || svc.name.to_lowercase().contains(&search)
                    || svc.display_name.to_lowercase().contains(&search)
            })
            .map(|(i, _)| i)
            .collect();
    }

    // ========== 选择导航 ==========

    pub fn select_prev(&mut self) {
        match self.current_view {
            View::Service => {
                let len = self.managed_services.len();
                if len == 0 {
                    return;
                }
                let new_idx = match self.selected_service {
                    Some(idx) => {
                        if idx == 0 {
                            len - 1
                        } else {
                            idx - 1
                        }
                    }
                    None => 0,
                };
                self.selected_service = Some(new_idx);
            }
            View::Settings => {
                self.settings_selected = if self.settings_selected == 0 {
                    2
                } else {
                    self.settings_selected - 1
                };
            }
            View::Tools => {
                self.tools_selected = if self.tools_selected == 0 {
                    5
                } else {
                    self.tools_selected - 1
                };
            }
        }
    }

    pub fn select_next(&mut self) {
        match self.current_view {
            View::Service => {
                let len = self.managed_services.len();
                if len == 0 {
                    return;
                }
                let new_idx = match self.selected_service {
                    Some(idx) => (idx + 1) % len,
                    None => 0,
                };
                self.selected_service = Some(new_idx);
            }
            View::Settings => {
                self.settings_selected = (self.settings_selected + 1) % 3;
            }
            View::Tools => {
                self.tools_selected = (self.tools_selected + 1) % 6;
            }
        }
    }

    // ========== 翻译与设置 ==========

    pub fn t(&self) -> &'static Translations {
        match self.language {
            Language::Chinese => &ZH,
            Language::English => &EN,
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn cycle_language(&mut self) {
        self.language = self.language.next();
        save_settings(&self.settings_file, &self.language, &self.theme);
    }

    pub fn theme(&self) -> &str {
        &self.theme
    }

    pub fn theme_colors(&self) -> &'static ThemeColors {
        match self.theme.as_str() {
            "light" => &LIGHT,
            _ => &DARK,
        }
    }

    pub fn cycle_theme(&mut self) {
        self.theme = match self.theme.as_str() {
            "light" => "dark".to_string(),
            _ => "light".to_string(),
        };
        save_settings(&self.settings_file, &self.language, &self.theme);
    }

    // ========== 其他 ==========

    pub fn settings_selected(&self) -> usize {
        self.settings_selected
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }
}

fn map_error(e: &ServiceError, lang: Language) -> String {
    let t = match lang {
        Language::Chinese => &ZH,
        Language::English => &EN,
    };
    let detail = match e {
        ServiceError::StartFailed(d)
        | ServiceError::StopFailed(d)
        | ServiceError::CommandFailed(d)
        | ServiceError::ParseFailed(d) => d.as_str(),
        ServiceError::NotFound(_) => return t.err_not_found.to_string(),
    };
    let lower = detail.to_lowercase();
    if lower.contains("access is denied") || lower.contains("access denied") {
        t.err_permission.to_string()
    } else if lower.contains("already running") || lower.contains("already started") {
        t.err_running.to_string()
    } else if lower.contains("not started") || lower.contains("has not been started") {
        t.err_not_started.to_string()
    } else if lower.contains("cannot be stopped") || lower.contains("can not be stopped") {
        t.err_cannot_stop.to_string()
    } else if lower.contains("timeout") {
        t.err_timeout.to_string()
    } else {
        t.err_failed.to_string()
    }
}

#[derive(Serialize, Deserialize)]
struct AppSettings {
    language: Language,
    theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Chinese,
            theme: "dark".to_string(),
        }
    }
}

fn load_settings(path: &PathBuf) -> AppSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(path: &PathBuf, language: &Language, theme: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let settings = AppSettings {
        language: *language,
        theme: theme.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(path, json);
    }
}
