use app_service::{ManagedService, ServiceInfo};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Service,
    Settings,
    Tools,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MsgType {
    Success,
    Error,
    Info,
}

pub struct App {
    current_view: View,
    managed_services: Vec<ManagedService>,
    all_services: Vec<ServiceInfo>,
    service_statuses: std::collections::HashMap<String, String>,
    data_file: PathBuf,
    selected_service: Option<usize>,
    show_add_dialog: bool,
    show_edit_dialog: bool,
    search_query: String,
    edit_form: Option<ManagedService>,
    status_message: String,
    status_message_type: MsgType,
    settings_selected: usize,
    tools_selected: usize,
    needs_refresh: bool,
}

impl App {
    pub fn new() -> Self {
        let data_file = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust-win-tool")
            .join("managed_services.json");

        let managed_services = app_service::load_managed_services(&data_file);

        let selected = if managed_services.is_empty() {
            None
        } else {
            Some(0)
        };

        Self {
            current_view: View::Service,
            managed_services,
            all_services: Vec::new(),
            service_statuses: std::collections::HashMap::new(),
            data_file,
            selected_service: selected,
            show_add_dialog: false,
            show_edit_dialog: false,
            search_query: String::new(),
            edit_form: None,
            status_message: String::new(),
            status_message_type: MsgType::Info,
            settings_selected: 0,
            tools_selected: 0,
            needs_refresh: true,
        }
    }

    pub fn current_view(&self) -> &View {
        &self.current_view
    }

    pub fn set_view(&mut self, view: View) {
        self.current_view = view;
    }

    pub fn managed_services(&self) -> &[ManagedService] {
        &self.managed_services
    }

    pub fn all_services(&self) -> &[ServiceInfo] {
        &self.all_services
    }

    pub fn service_status(&self, name: &str) -> &str {
        self.service_statuses
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or("Unknown")
    }

    pub fn selected_service(&self) -> Option<usize> {
        self.selected_service
    }

    pub fn set_selected_service(&mut self, index: Option<usize>) {
        self.selected_service = index;
    }

    pub fn show_add_dialog(&self) -> bool {
        self.show_add_dialog
    }

    pub fn set_show_add_dialog(&mut self, show: bool) {
        self.show_add_dialog = show;
    }

    pub fn show_edit_dialog(&self) -> bool {
        self.show_edit_dialog
    }

    pub fn set_show_edit_dialog(&mut self, show: bool) {
        self.show_edit_dialog = show;
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    pub fn edit_form(&self) -> Option<&ManagedService> {
        self.edit_form.as_ref()
    }

    pub fn set_edit_form(&mut self, form: Option<ManagedService>) {
        self.edit_form = form;
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn status_message_type(&self) -> &MsgType {
        &self.status_message_type
    }

    pub fn set_status_message(&mut self, msg: String, msg_type: MsgType) {
        self.status_message = msg;
        self.status_message_type = msg_type;
    }

    pub fn clear_status_message(&mut self) {
        self.status_message.clear();
    }

    pub fn settings_selected(&self) -> usize {
        self.settings_selected
    }

    pub fn set_settings_selected(&mut self, index: usize) {
        self.settings_selected = index;
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }

    pub fn set_tools_selected(&mut self, index: usize) {
        self.tools_selected = index;
    }

    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }

    pub fn set_needs_refresh(&mut self, needs: bool) {
        self.needs_refresh = needs;
    }

    pub fn load_services(&mut self) -> Result<(), String> {
        self.all_services = app_service::list_all_services().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn refresh_statuses(&mut self) {
        for svc in &self.managed_services {
            let status = app_service::get_service_status(&svc.name)
                .unwrap_or_else(|_| "未知".to_string());
            self.service_statuses.insert(svc.name.clone(), status);
        }
        self.set_status_message("服务状态已刷新".to_string(), MsgType::Success);
    }

    pub fn add_service(&mut self, name: String, display_name: String) {
        if !self.managed_services.iter().any(|s| s.name == name) {
            self.managed_services.push(ManagedService {
                name: name.clone(),
                display_name,
                enabled: true,
            });
            app_service::save_managed_services(&self.data_file, &self.managed_services);
            if self.selected_service.is_none() {
                self.selected_service = Some(0);
            }
            self.set_status_message(
                format!("已添加服务: {}", name),
                MsgType::Success,
            );
        }
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
                self.set_status_message(format!("已删除服务: {}", name), MsgType::Success);
            }
        } else {
            self.set_status_message("请先选择一个服务".to_string(), MsgType::Info);
        }
    }

    pub fn start_selected_service(&mut self) {
        if let Some(idx) = self.selected_service {
            if idx < self.managed_services.len() {
                let name = self.managed_services[idx].name.clone();
                match app_service::start_service(&name) {
                    Ok(()) => {
                        self.set_status_message(
                            format!("已发送启动指令: {}", name),
                            MsgType::Success,
                        );
                        let status = app_service::get_service_status(&name)
                            .unwrap_or_else(|_| "未知".to_string());
                        self.service_statuses.insert(name, status);
                    }
                    Err(e) => {
                        self.set_status_message(
                            format!("启动失败: {}", e),
                            MsgType::Error,
                        );
                    }
                }
            }
        } else {
            self.set_status_message("请先选择一个服务".to_string(), MsgType::Info);
        }
    }

    pub fn stop_selected_service(&mut self) {
        if let Some(idx) = self.selected_service {
            if idx < self.managed_services.len() {
                let name = self.managed_services[idx].name.clone();
                match app_service::stop_service(&name) {
                    Ok(()) => {
                        self.set_status_message(
                            format!("已发送停止指令: {}", name),
                            MsgType::Success,
                        );
                        let status = app_service::get_service_status(&name)
                            .unwrap_or_else(|_| "未知".to_string());
                        self.service_statuses.insert(name, status);
                    }
                    Err(e) => {
                        self.set_status_message(
                            format!("停止失败: {}", e),
                            MsgType::Error,
                        );
                    }
                }
            }
        } else {
            self.set_status_message("请先选择一个服务".to_string(), MsgType::Info);
        }
    }

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

    pub fn update_service(&mut self, name: String, display_name: String, enabled: bool) {
        if let Some(svc) = self.managed_services.iter_mut().find(|s| s.name == name) {
            svc.display_name = display_name;
            svc.enabled = enabled;
            app_service::save_managed_services(&self.data_file, &self.managed_services);
        }
    }
}
