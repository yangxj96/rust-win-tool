use app_core::{get_app_info, AppConfig};
use app_service::{ManagedService, ServiceInfo};
use app_utils::{current_timestamp, format_timestamp, generate_id};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

struct AppState {
    managed: Mutex<Vec<ManagedService>>,
    data_file: Mutex<PathBuf>,
}

fn data_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    dir.join("managed_services.json")
}

#[tauri::command]
fn greet(name: &str) -> String {
    let info = get_app_info();
    let now = current_timestamp();
    format!(
        "Hello, {}! Welcome to {} v{}. ({})",
        name,
        info.app_name,
        info.version,
        format_timestamp(now)
    )
}

#[tauri::command]
fn get_config() -> AppConfig {
    get_app_info()
}

#[tauri::command]
fn generate_uuid() -> String {
    generate_id()
}

#[tauri::command]
fn list_services() -> Result<Vec<ServiceInfo>, String> {
    app_service::list_all_services().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_service_status(name: String) -> Result<String, String> {
    app_service::get_service_status(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_svc(name: String) -> Result<(), String> {
    app_service::start_service(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_svc(name: String) -> Result<(), String> {
    app_service::stop_service(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_managed_services(state: State<AppState>) -> Vec<ManagedService> {
    state.managed.lock().unwrap().clone()
}

#[tauri::command]
fn add_managed_service(state: State<AppState>, name: String, display_name: String) -> Vec<ManagedService> {
    let mut services = state.managed.lock().unwrap();
    if !services.iter().any(|s| s.name == name) {
        services.push(ManagedService {
            name,
            display_name,
            enabled: true,
        });
        let path = state.data_file.lock().unwrap();
        app_service::save_managed_services(&path, &services);
    }
    services.clone()
}

#[tauri::command]
fn update_managed_service(state: State<AppState>, name: String, display_name: String, enabled: bool) -> Vec<ManagedService> {
    let mut services = state.managed.lock().unwrap();
    if let Some(svc) = services.iter_mut().find(|s| s.name == name) {
        svc.display_name = display_name;
        svc.enabled = enabled;
        let path = state.data_file.lock().unwrap();
        app_service::save_managed_services(&path, &services);
    }
    services.clone()
}

#[tauri::command]
fn remove_managed_service(state: State<AppState>, name: String) -> Vec<ManagedService> {
    let mut services = state.managed.lock().unwrap();
    services.retain(|s| s.name != name);
    let path = state.data_file.lock().unwrap();
    app_service::save_managed_services(&path, &services);
    services.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let path = data_path(app.handle());
            let managed = app_service::load_managed_services(&path);
            app.manage(AppState {
                managed: Mutex::new(managed),
                data_file: Mutex::new(path),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_config,
            generate_uuid,
            list_services,
            get_service_status,
            start_svc,
            stop_svc,
            get_managed_services,
            add_managed_service,
            update_managed_service,
            remove_managed_service,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
