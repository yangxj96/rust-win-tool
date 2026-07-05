use app_service::{ManagedService, ServiceInfo};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Service,
    Settings,
    Tools,
}

impl View {
    pub fn all() -> &'static [View] {
        &[View::Service, View::Settings, View::Tools]
    }

    pub fn index(&self) -> usize {
        match self {
            View::Service => 0,
            View::Settings => 1,
            View::Tools => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => View::Service,
            1 => View::Settings,
            2 => View::Tools,
            _ => View::Service,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            View::Service => "服务管理",
            View::Settings => "设置",
            View::Tools => "系统工具",
        }
    }
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
    status_message: String,
    status_message_type: MsgType,
    settings_selected: usize,
    tools_selected: usize,
    // 添加服务对话框
    show_add_dialog: bool,
    add_dialog_services: Vec<ServiceInfo>,
    add_dialog_filtered: Vec<usize>,
    add_dialog_selected: usize,
    add_dialog_search: String,
    // 执行状态
    is_executing: bool,
    executing_action: String,
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
            status_message: String::new(),
            status_message_type: MsgType::Info,
            settings_selected: 0,
            tools_selected: 0,
            show_add_dialog: false,
            add_dialog_services: Vec::new(),
            add_dialog_filtered: Vec::new(),
            add_dialog_selected: 0,
            add_dialog_search: String::new(),
            is_executing: false,
            executing_action: String::new(),
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

    pub fn selected_service(&self) -> Option<usize> {
        self.selected_service
    }

    pub fn refresh_statuses(&mut self) {
        self.set_executing("刷新状态");
        self.set_status_message("正在刷新服务状态...".to_string(), MsgType::Info);
        let mut success_count = 0;
        let mut fail_count = 0;
        for svc in &self.managed_services {
            match app_service::get_service_status(&svc.name) {
                Ok(status) => {
                    self.service_statuses.insert(svc.name.clone(), status);
                    success_count += 1;
                }
                Err(_) => {
                    self.service_statuses.insert(svc.name.clone(), "未知".to_string());
                    fail_count += 1;
                }
            }
        }
        self.clear_executing();
        if fail_count == 0 {
            self.set_status_message(
                format!("已刷新 {} 个服务状态", success_count),
                MsgType::Success,
            );
        } else {
            self.set_status_message(
                format!("刷新完成: {} 成功, {} 失败", success_count, fail_count),
                MsgType::Error,
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
                let display_name = self.managed_services[idx].display_name.clone();
                self.set_executing(&format!("启动 {}", display_name));
                self.set_status_message(
                    format!("正在启动 \"{}\"...", display_name),
                    MsgType::Info,
                );
                match app_service::start_service(&name) {
                    Ok(()) => {
                        let status = app_service::get_service_status(&name)
                            .unwrap_or_else(|_| "未知".to_string());
                        let current_status = Self::status_to_chinese(&status).to_string();
                        self.service_statuses.insert(name, status);
                        self.clear_executing();
                        self.set_status_message(
                            format!("服务 \"{}\" 已启动，当前状态: {}", display_name, current_status),
                            MsgType::Success,
                        );
                    }
                    Err(e) => {
                        self.clear_executing();
                        self.set_status_message(
                            format!("启动 \"{}\" 失败: {}", display_name, e),
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
                let display_name = self.managed_services[idx].display_name.clone();
                self.set_executing(&format!("停止 {}", display_name));
                self.set_status_message(
                    format!("正在停止 \"{}\"...", display_name),
                    MsgType::Info,
                );
                match app_service::stop_service(&name) {
                    Ok(()) => {
                        let status = app_service::get_service_status(&name)
                            .unwrap_or_else(|_| "未知".to_string());
                        let current_status = Self::status_to_chinese(&status).to_string();
                        self.service_statuses.insert(name, status);
                        self.clear_executing();
                        self.set_status_message(
                            format!("服务 \"{}\" 已停止，当前状态: {}", display_name, current_status),
                            MsgType::Success,
                        );
                    }
                    Err(e) => {
                        self.clear_executing();
                        self.set_status_message(
                            format!("停止 \"{}\" 失败: {}", display_name, e),
                            MsgType::Error,
                        );
                    }
                }
            }
        } else {
            self.set_status_message("请先选择一个服务".to_string(), MsgType::Info);
        }
    }

    fn status_to_chinese(status: &str) -> &str {
        match status {
            "Running" => "运行中",
            "Stopped" => "已停止",
            "Paused" => "已暂停",
            "StartPending" => "启动中",
            "StopPending" => "停止中",
            _ => status,
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
        match app_service::list_all_services() {
            Ok(services) => {
                self.add_dialog_services = services;
                self.add_dialog_search.clear();
                self.add_dialog_selected = 0;
                self.rebuild_add_dialog_filter();
                self.show_add_dialog = true;
            }
            Err(e) => {
                self.set_status_message(format!("加载服务列表失败: {}", e), MsgType::Error);
            }
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
            let display_name = svc.display_name.clone();

            if self.managed_services.iter().any(|s| s.name == name) {
                self.set_status_message(
                    format!("服务 \"{}\" 已在管理列表中", display_name),
                    MsgType::Info,
                );
            } else {
                self.managed_services.push(ManagedService {
                    name: name.clone(),
                    display_name: display_name.clone(),
                    enabled: true,
                });
                app_service::save_managed_services(&self.data_file, &self.managed_services);
                if self.selected_service.is_none() {
                    self.selected_service = Some(0);
                }
                self.set_status_message(
                    format!("已添加服务: {}", display_name),
                    MsgType::Success,
                );
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

    // ========== 状态消息 ==========

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

    // ========== 执行状态 ==========

    pub fn is_executing(&self) -> bool {
        self.is_executing
    }

    pub fn executing_action(&self) -> &str {
        &self.executing_action
    }

    pub fn set_executing(&mut self, action: &str) {
        self.is_executing = true;
        self.executing_action = action.to_string();
    }

    pub fn clear_executing(&mut self) {
        self.is_executing = false;
        self.executing_action.clear();
    }

    // ========== 其他 ==========

    pub fn settings_selected(&self) -> usize {
        self.settings_selected
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }
}
