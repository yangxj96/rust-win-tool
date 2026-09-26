//! 服务控制和托管服务文件操作。

use std::path::Path;

use crate::features::services::{self, ManagedService, ServiceError, ServiceInfo};
use crate::shared::BackendError;

/// 通过 Windows 服务控制管理器枚举服务。
pub fn list_services() -> Result<Vec<ServiceInfo>, BackendError> {
    services::list_all_services().map_err(service_error)
}

/// 根据服务内部名称查询当前状态。
pub fn service_status(name: &str) -> Result<String, BackendError> {
    services::get_service_status(name).map_err(service_error)
}

/// 启动服务；权限不足或服务状态不允许时返回错误。
pub fn start_service(name: &str) -> Result<(), BackendError> {
    services::start_service(name).map_err(service_error)
}

/// 停止服务；权限不足或依赖关系不允许时返回错误。
pub fn stop_service(name: &str) -> Result<(), BackendError> {
    services::stop_service(name).map_err(service_error)
}

/// 读取服务配置和运行时信息，供服务详情对话框展示。
pub fn service_details(name: &str) -> Result<services::ServiceDetails, BackendError> {
    services::get_service_details(name).map_err(service_error)
}

/// 通过服务控制管理器修改服务启动类型。
pub fn set_service_start_type(
    name: &str,
    start_type: services::StartType,
) -> Result<(), BackendError> {
    services::set_service_start_type(name, start_type).map_err(service_error)
}

/// 按现有 JSON 结构写出指定的托管服务列表。
pub fn export_managed_services(
    path: &Path,
    managed_services: &[ManagedService],
) -> Result<(), BackendError> {
    let json = serde_json::to_string_pretty(managed_services)
        .map_err(|error| BackendError::Parse(error.to_string()))?;
    std::fs::write(path, json).map_err(|error| BackendError::Command(error.to_string()))
}

/// 从导出文件读取托管服务；调用方负责用结果替换当前列表。
pub fn import_managed_services(path: &Path) -> Result<Vec<ManagedService>, BackendError> {
    let text =
        std::fs::read_to_string(path).map_err(|error| BackendError::Command(error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| BackendError::Parse(error.to_string()))
}

#[derive(Debug, Clone)]
/// 同步服务操作请求；批量操作按输入顺序处理。
pub enum ServiceOperation {
    Start(String),
    Stop(String),
    StartAll(Vec<String>),
    StopAll(Vec<String>),
    RefreshAll(Vec<String>),
}

#[derive(Debug)]
/// 批量操作中单个服务对应的状态或错误。
pub struct ServiceOperationResult {
    pub name: String,
    pub status: Result<String, BackendError>,
}

/// 执行单项或批量服务操作，并返回每个服务的最终状态。
///
/// 此函数同步调用 Windows API，必须从后台任务调用，不能阻塞 GPUI 事件循环。
pub fn execute_service_operation(operation: ServiceOperation) -> Vec<ServiceOperationResult> {
    match operation {
        ServiceOperation::Start(name) => vec![execute_one(&name, start_service)],
        ServiceOperation::Stop(name) => vec![execute_one(&name, stop_service)],
        ServiceOperation::StartAll(names) => names
            .iter()
            .map(|name| execute_one(name, start_service))
            .collect(),
        ServiceOperation::StopAll(names) => names
            .iter()
            .map(|name| execute_one(name, stop_service))
            .collect(),
        ServiceOperation::RefreshAll(names) => names
            .iter()
            .map(|name| ServiceOperationResult {
                name: name.clone(),
                status: service_status(name),
            })
            .collect(),
    }
}

fn execute_one(name: &str, action: fn(&str) -> Result<(), BackendError>) -> ServiceOperationResult {
    let action_result = action(name);
    let status = match action_result {
        Ok(()) => service_status(name),
        Err(error) => Err(error),
    };
    ServiceOperationResult {
        name: name.to_string(),
        status,
    }
}

fn service_error(error: ServiceError) -> BackendError {
    BackendError::Service(error.to_string())
}
