use app_service::{ManagedService, ServiceInfo};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Service,
    Settings,
    Tools,
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
}

impl App {
    pub fn new() -> Self {
        let data_file = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust-win-tool")
            .join("managed_services.json");
        
        let managed_services = app_service::load_managed_services(&data_file);
        
        Self {
            current_view: View::Service,
            managed_services,
            all_services: Vec::new(),
            service_statuses: std::collections::HashMap::new(),
            data_file,
            selected_service: None,
            show_add_dialog: false,
            show_edit_dialog: false,
            search_query: String::new(),
            edit_form: None,
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
        self.service_statuses.get(name).map(|s| s.as_str()).unwrap_or("Unknown")
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
    
    pub fn load_services(&mut self) -> Result<(), String> {
        self.all_services = app_service::list_all_services().map_err(|e| e.to_string())?;
        Ok(())
    }
    
    pub fn refresh_statuses(&mut self) {
        for svc in &self.managed_services {
            let status = app_service::get_service_status(&svc.name)
                .unwrap_or_else(|_| "Unknown".to_string());
            self.service_statuses.insert(svc.name.clone(), status);
        }
    }
    
    pub fn add_service(&mut self, name: String, display_name: String) {
        if !self.managed_services.iter().any(|s| s.name == name) {
            self.managed_services.push(ManagedService {
                name,
                display_name,
                enabled: true,
            });
            app_service::save_managed_services(&self.data_file, &self.managed_services);
        }
    }
    
    pub fn remove_service(&mut self, name: &str) {
        self.managed_services.retain(|s| s.name != name);
        self.service_statuses.remove(name);
        app_service::save_managed_services(&self.data_file, &self.managed_services);
    }
    
    pub fn update_service(&mut self, name: String, display_name: String, enabled: bool) {
        if let Some(svc) = self.managed_services.iter_mut().find(|s| s.name == name) {
            svc.display_name = display_name;
            svc.enabled = enabled;
            app_service::save_managed_services(&self.data_file, &self.managed_services);
        }
    }
    
    pub fn start_service(&mut self, name: &str) -> Result<(), String> {
        app_service::start_service(name).map_err(|e| e.to_string())?;
        self.refresh_statuses();
        Ok(())
    }
    
    pub fn stop_service(&mut self, name: &str) -> Result<(), String> {
        app_service::stop_service(name).map_err(|e| e.to_string())?;
        self.refresh_statuses();
        Ok(())
    }
}