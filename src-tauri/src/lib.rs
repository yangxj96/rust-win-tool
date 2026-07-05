use app_core::AppConfig;
use app_service::ServiceInfo;

pub fn get_config() -> AppConfig {
    app_core::get_app_info()
}

pub fn list_services() -> Result<Vec<ServiceInfo>, String> {
    app_service::list_all_services().map_err(|e| e.to_string())
}

pub fn get_service_status(name: &str) -> Result<String, String> {
    app_service::get_service_status(name).map_err(|e| e.to_string())
}

pub fn start_service(name: &str) -> Result<(), String> {
    app_service::start_service(name).map_err(|e| e.to_string())
}

pub fn stop_service(name: &str) -> Result<(), String> {
    app_service::stop_service(name).map_err(|e| e.to_string())
}