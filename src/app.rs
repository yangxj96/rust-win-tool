use app_service::{ManagedService, ServiceError, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::i18n::{Language, Translations, EN, ZH};
use crate::theme::{ThemeColors, DARK, LIGHT};

const SETTINGS_COUNT: usize = 2;
const TOOLS_COUNT: usize = 6;
const VIEW_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
}

impl Theme {
    pub fn next(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Service,
    Tools,
    Scripts,
    Settings,
}

impl View {
    pub fn index(&self) -> usize {
        match self {
            View::Service => 0,
            View::Tools => 1,
            View::Scripts => 2,
            View::Settings => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => View::Service,
            1 => View::Tools,
            2 => View::Scripts,
            3 => View::Settings,
            _ => View::Service,
        }
    }
}

pub enum PendingAction {
    StartService(String),
    StopService(String),
    StartAll,
    StopAll,
    RefreshAll,
}

#[derive(Default)]
pub struct AddDialogState {
    pub show: bool,
    pub services: Vec<ServiceInfo>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub search: String,
}

pub struct App {
    current_view: View,
    managed_services: Vec<ManagedService>,
    service_statuses: std::collections::HashMap<String, String>,
    service_messages: std::collections::HashMap<String, String>,
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
    pending_action: Option<PendingAction>,
    add_dialog: AddDialogState,
}

pub struct SystemInfo {
    pub os: String,
    pub version: String,
    pub build: String,
    pub computer: String,
    pub user: String,
    pub cpu: String,
    pub cores: String,
    pub ram: String,
}

impl App {
    pub fn new() -> Self {
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("config");

        let _ = std::fs::create_dir_all(&app_dir);

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
            scripts_selected: 0,
            script_result: None,
            tool_detail_active: false,
            system_info: None,
            pending_action: None,
            add_dialog: AddDialogState::default(),
        }
    }

    // ========== 视图切换 ==========

    pub fn current_view(&self) -> &View {
        &self.current_view
    }

    pub fn prev_view(&mut self) {
        let idx = self.current_view.index();
        let new_idx = if idx == 0 { VIEW_COUNT - 1 } else { idx - 1 };
        self.current_view = View::from_index(new_idx);
    }

    pub fn next_view(&mut self) {
        let idx = self.current_view.index();
        let new_idx = (idx + 1) % VIEW_COUNT;
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
                let name = self.managed_services[idx].name.clone();
                self.managed_services.remove(idx);
                self.service_statuses.remove(&name);
                self.service_messages.remove(&name);
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

    pub fn start_all_services(&mut self) {
        let msg = self.t().status_starting.to_string();
        for svc in &self.managed_services {
            self.service_statuses.insert(svc.name.clone(), msg.clone());
        }
        self.pending_action = Some(PendingAction::StartAll);
    }

    pub fn stop_all_services(&mut self) {
        let msg = self.t().status_stopping.to_string();
        for svc in &self.managed_services {
            self.service_statuses.insert(svc.name.clone(), msg.clone());
        }
        self.pending_action = Some(PendingAction::StopAll);
    }

    pub fn take_pending_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    pub fn execute_pending(&mut self, action: PendingAction) {
        let lang = self.language;
        match action {
            PendingAction::StartService(name) => {
                execute_single_service(
                    &name,
                    app_service::start_service,
                    &mut self.service_statuses,
                    &mut self.service_messages,
                    lang,
                );
            }
            PendingAction::StopService(name) => {
                execute_single_service(
                    &name,
                    app_service::stop_service,
                    &mut self.service_statuses,
                    &mut self.service_messages,
                    lang,
                );
            }
            PendingAction::StartAll => {
                let names: Vec<String> = self.managed_services.iter().map(|s| s.name.clone()).collect();
                for name in &names {
                    execute_single_service(
                        name,
                        app_service::start_service,
                        &mut self.service_statuses,
                        &mut self.service_messages,
                        lang,
                    );
                }
            }
            PendingAction::StopAll => {
                let names: Vec<String> = self.managed_services.iter().map(|s| s.name.clone()).collect();
                for name in &names {
                    execute_single_service(
                        name,
                        app_service::stop_service,
                        &mut self.service_statuses,
                        &mut self.service_messages,
                        lang,
                    );
                }
            }
            PendingAction::RefreshAll => {
                self.service_messages.clear();
                for svc in &self.managed_services {
                    let status = app_service::get_service_status(&svc.name)
                        .unwrap_or_else(|_| translations_for(lang).err_unknown.to_string());
                    self.service_statuses.insert(svc.name.clone(), status);
                }
            }
        }
    }

    // ========== 添加服务对话框 ==========

    pub fn show_add_dialog(&self) -> bool {
        self.add_dialog.show
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

    pub fn open_add_dialog(&mut self) {
        if let Ok(services) = app_service::list_all_services() {
            self.add_dialog.services = services;
            self.add_dialog.search.clear();
            self.add_dialog.selected = 0;
            self.rebuild_add_dialog_filter();
            self.add_dialog.show = true;
        }
    }

    pub fn close_add_dialog(&mut self) {
        self.add_dialog = AddDialogState::default();
    }

    pub fn add_dialog_input(&mut self, c: char) {
        self.add_dialog.search.push(c);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn add_dialog_backspace(&mut self) {
        self.add_dialog.search.pop();
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn add_dialog_select_prev(&mut self) {
        let len = self.add_dialog.filtered.len();
        if len == 0 {
            return;
        }
        self.add_dialog.selected = if self.add_dialog.selected == 0 {
            len - 1
        } else {
            self.add_dialog.selected - 1
        };
    }

    pub fn add_dialog_select_next(&mut self) {
        let len = self.add_dialog.filtered.len();
        if len == 0 {
            return;
        }
        self.add_dialog.selected = (self.add_dialog.selected + 1) % len;
    }

    pub fn confirm_add_service(&mut self) {
        if self.add_dialog.selected < self.add_dialog.filtered.len() {
            let real_idx = self.add_dialog.filtered[self.add_dialog.selected];
            let svc = &self.add_dialog.services[real_idx];
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
        let search = self.add_dialog.search.to_lowercase();
        self.add_dialog.filtered = self
            .add_dialog
            .services
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
                    SETTINGS_COUNT - 1
                } else {
                    self.settings_selected - 1
                };
            }
            View::Tools => {
                self.tools_selected = if self.tools_selected == 0 {
                    TOOLS_COUNT - 1
                } else {
                    self.tools_selected - 1
                };
            }
            View::Scripts => {
                self.scripts_selected = 0;
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
                self.settings_selected = (self.settings_selected + 1) % SETTINGS_COUNT;
            }
            View::Tools => {
                self.tools_selected = (self.tools_selected + 1) % TOOLS_COUNT;
            }
            View::Scripts => {
                self.scripts_selected = 0;
            }
        }
    }

    // ========== 翻译与设置 ==========

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
            Theme::Light => &LIGHT,
            Theme::Dark => &DARK,
        }
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
        save_settings(&self.settings_file, self.language, self.theme);
    }

    // ========== 其他 ==========

    pub fn settings_selected(&self) -> usize {
        self.settings_selected
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }

    pub fn scripts_selected(&self) -> usize {
        self.scripts_selected
    }

    pub fn tool_detail_active(&self) -> bool {
        self.tool_detail_active
    }

    pub fn system_info(&self) -> Option<&SystemInfo> {
        self.system_info.as_ref()
    }

    pub fn toggle_tool_detail(&mut self) {
        if !self.tool_detail_active {
            self.tool_detail_active = true;
            if self.system_info.is_none() {
                self.fetch_system_info();
            }
        }
    }

    pub fn close_tool_detail(&mut self) {
        self.tool_detail_active = false;
    }

    pub fn fetch_system_info(&mut self) {
        use std::process::Command;

        let ps_script = r#"
            [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
            $os = Get-CimInstance Win32_OperatingSystem
            $cpu = Get-CimInstance Win32_Processor
            $sys = Get-CimInstance Win32_ComputerSystem
            @{
                OS = $os.Caption
                Version = $os.Version
                Build = $os.BuildNumber
                Computer = $env:COMPUTERNAME
                User = $env:USERNAME
                CPU = $cpu.Name
                Cores = $cpu.NumberOfCores
                RAM = [math]::Round($sys.TotalPhysicalMemory / 1GB, 2)
            } | ConvertTo-Json -Compress
        "#;

        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", ps_script]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd.output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout).to_string();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    self.system_info = Some(SystemInfo {
                        os: json["OS"].as_str().unwrap_or("N/A").to_string(),
                        version: json["Version"].as_str().unwrap_or("N/A").to_string(),
                        build: json["Build"].as_str().unwrap_or("N/A").to_string(),
                        computer: json["Computer"].as_str().unwrap_or("N/A").to_string(),
                        user: json["User"].as_str().unwrap_or("N/A").to_string(),
                        cpu: json["CPU"].as_str().unwrap_or("N/A").to_string(),
                        cores: json["Cores"].as_str().unwrap_or("N/A").to_string(),
                        ram: json["RAM"].as_str().unwrap_or("N/A").to_string(),
                    });
                }
            }
            _ => {
                self.system_info = Some(SystemInfo {
                    os: "N/A".to_string(),
                    version: "N/A".to_string(),
                    build: "N/A".to_string(),
                    computer: "N/A".to_string(),
                    user: "N/A".to_string(),
                    cpu: "N/A".to_string(),
                    cores: "N/A".to_string(),
                    ram: "N/A".to_string(),
                });
            }
        }
    }

    pub fn script_result(&self) -> Option<&str> {
        self.script_result.as_deref()
    }

    pub fn execute_script_for_selected(&mut self) {
        use crate::ui::scripts::{SCRIPTS, ScriptAction};
        if self.scripts_selected < SCRIPTS.len() {
            let script = &SCRIPTS[self.scripts_selected];
            let lang = self.language;
            let result = match script.action {
                ScriptAction::ResetNavicat => crate::ui::scripts::reset_navicat(lang),
            };
            self.script_result = Some(match result {
                Ok(msg) => msg,
                Err(e) => e,
            });
        }
    }
}

fn translations_for(lang: Language) -> &'static Translations {
    match lang {
        Language::Chinese => &ZH,
        Language::English => &EN,
    }
}

fn execute_single_service(
    name: &str,
    action_fn: fn(&str) -> Result<(), ServiceError>,
    statuses: &mut std::collections::HashMap<String, String>,
    messages: &mut std::collections::HashMap<String, String>,
    lang: Language,
) {
    match action_fn(name) {
        Ok(()) => {
            messages.remove(name);
        }
        Err(e) => {
            messages.insert(name.to_string(), map_error(&e, lang));
        }
    }
    let status = app_service::get_service_status(name)
        .unwrap_or_else(|_| translations_for(lang).err_unknown.to_string());
    statuses.insert(name.to_string(), status);
}

fn map_error(e: &ServiceError, lang: Language) -> String {
    let t = translations_for(lang);
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
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(path: &Path, language: Language, theme: Theme) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let settings = AppSettings { language, theme };
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_index_roundtrip() {
        for i in 0..4 {
            let view = View::from_index(i);
            assert_eq!(view.index(), i);
        }
    }

    #[test]
    fn view_from_index_out_of_bounds() {
        assert_eq!(View::from_index(99), View::Service);
    }

    #[test]
    fn prev_view_wraps() {
        let mut app = App::new();
        app.current_view = View::Service;
        app.prev_view();
        assert_eq!(app.current_view, View::Settings);
    }

    #[test]
    fn next_view_wraps() {
        let mut app = App::new();
        app.current_view = View::Settings;
        app.next_view();
        assert_eq!(app.current_view, View::Service);
    }

    #[test]
    fn theme_roundtrip() {
        assert_eq!(Theme::Dark.next(), Theme::Light);
        assert_eq!(Theme::Light.next(), Theme::Dark);
    }

    #[test]
    fn settings_selection_wraps() {
        let mut app = App::new();
        app.current_view = View::Settings;
        app.settings_selected = 0;
        app.select_prev();
        assert_eq!(app.settings_selected, SETTINGS_COUNT - 1);
        app.select_next();
        assert_eq!(app.settings_selected, 0);
    }

    #[test]
    fn tools_selection_wraps() {
        let mut app = App::new();
        app.current_view = View::Tools;
        app.tools_selected = 0;
        app.select_prev();
        assert_eq!(app.tools_selected, TOOLS_COUNT - 1);
        app.select_next();
        assert_eq!(app.tools_selected, 0);
    }

    #[test]
    fn add_dialog_default_is_closed() {
        let dialog = AddDialogState::default();
        assert!(!dialog.show);
        assert!(dialog.services.is_empty());
        assert!(dialog.filtered.is_empty());
        assert_eq!(dialog.selected, 0);
        assert!(dialog.search.is_empty());
    }

    #[test]
    fn settings_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("rust-win-tool-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");

        save_settings(&path, Language::English, Theme::Light);
        let loaded = load_settings(&path);
        assert_eq!(loaded.language, Language::English);
        assert_eq!(loaded.theme, Theme::Light);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
