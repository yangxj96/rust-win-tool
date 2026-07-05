use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Language::Chinese => "简体中文",
            Language::English => "English",
        }
    }

    pub fn next(&self) -> Language {
        match self {
            Language::Chinese => Language::English,
            Language::English => Language::Chinese,
        }
    }
}

pub struct Translations {
    // 标题栏
    pub app_title: &'static str,
    pub tab_service: &'static str,
    pub tab_tools: &'static str,
    pub tab_scripts: &'static str,
    pub tab_settings: &'static str,

    // 状态栏
    pub hint_switch: &'static str,
    pub hint_select: &'static str,
    pub hint_add: &'static str,
    pub hint_delete: &'static str,
    pub hint_start: &'static str,
    pub hint_stop: &'static str,
    pub hint_start_all: &'static str,
    pub hint_stop_all: &'static str,
    pub hint_refresh: &'static str,
    pub hint_confirm: &'static str,
    pub hint_quit: &'static str,

    // 服务视图
    pub svc_header: &'static str,
    pub svc_selected: &'static str,
    pub svc_none: &'static str,
    pub svc_empty: &'static str,
    pub col_name: &'static str,
    pub col_display: &'static str,
    pub col_status: &'static str,
    pub col_message: &'static str,
    pub status_running: &'static str,
    pub status_stopped: &'static str,
    pub status_starting: &'static str,
    pub status_stopping: &'static str,
    pub status_refreshing: &'static str,

    // 工具视图
    pub tools_header: &'static str,
    pub tool_sysinfo: &'static str,
    pub tool_sysinfo_desc: &'static str,
    pub tool_diskclean: &'static str,
    pub tool_diskclean_desc: &'static str,
    pub tool_network: &'static str,
    pub tool_network_desc: &'static str,
    pub tool_process: &'static str,
    pub tool_process_desc: &'static str,
    pub tool_registry: &'static str,
    pub tool_registry_desc: &'static str,
    pub tool_eventlog: &'static str,
    pub tool_eventlog_desc: &'static str,

    // 脚本视图
    pub scripts_header: &'static str,
    pub script_reset_navicat: &'static str,
    pub script_reset_navicat_desc: &'static str,

    // 设置视图
    pub settings_header: &'static str,
    pub setting_language: &'static str,
    pub setting_theme: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,

    // 对话框
    pub dialog_add_title: &'static str,
    pub dialog_search: &'static str,
    pub dialog_empty: &'static str,
    pub dialog_hint_add: &'static str,
    pub dialog_hint_cancel: &'static str,

    // 错误消息
    pub err_permission: &'static str,
    pub err_running: &'static str,
    pub err_not_started: &'static str,
    pub err_cannot_stop: &'static str,
    pub err_timeout: &'static str,
    pub err_not_found: &'static str,
    pub err_failed: &'static str,
}

pub const ZH: Translations = Translations {
    app_title: "Rust 系统工具",
    tab_service: " 服务管理 ",
    tab_tools: " 系统工具 ",
    tab_scripts: " 执行脚本 ",
    tab_settings: " 设置 ",

    hint_switch: "切换",
    hint_select: "选择",
    hint_add: "添加",
    hint_delete: "删除",
    hint_start: "启动",
    hint_stop: "停止",
    hint_start_all: "启动全部",
    hint_stop_all: "停止全部",
    hint_refresh: "刷新",
    hint_confirm: "确认",
    hint_quit: "退出",

    svc_header: "服务管理",
    svc_selected: "选中",
    svc_none: "未选择",
    svc_empty: "暂无管理的服务，按 a 添加服务",
    col_name: "服务名称",
    col_display: "显示名称",
    col_status: "状态",
    col_message: "消息",
    status_running: "运行中",
    status_stopped: "已停止",
    status_starting: "启动中",
    status_stopping: "停止中",
    status_refreshing: "刷新中",

    tools_header: "系统工具",
    tool_sysinfo: "系统信息",
    tool_sysinfo_desc: "查看系统版本、CPU、内存等信息",
    tool_diskclean: "磁盘清理",
    tool_diskclean_desc: "清理临时文件和系统垃圾",
    tool_network: "网络诊断",
    tool_network_desc: "检测网络连接和DNS配置",
    tool_process: "进程管理",
    tool_process_desc: "查看和管理系统进程",
    tool_registry: "注册表编辑器",
    tool_registry_desc: "编辑 Windows 注册表",
    tool_eventlog: "事件查看器",
    tool_eventlog_desc: "查看系统事件日志",

    scripts_header: "执行脚本",
    script_reset_navicat: "重置Navicat时间",
    script_reset_navicat_desc: "清理Navicat注册信息和CLSID残留",

    settings_header: "设置",
    setting_language: "语言选择",
    setting_theme: "主题切换",
    theme_dark: "深色模式",
    theme_light: "浅色模式",

    dialog_add_title: "添加服务",
    dialog_search: "搜索",
    dialog_empty: "没有匹配的服务",
    dialog_hint_add: "添加",
    dialog_hint_cancel: "取消",

    err_permission: "权限不足",
    err_running: "服务已在运行",
    err_not_started: "服务未启动",
    err_cannot_stop: "服务不可停止",
    err_timeout: "操作超时",
    err_not_found: "服务不存在",
    err_failed: "操作失败",
};

pub const EN: Translations = Translations {
    app_title: "Rust Win Tool",
    tab_service: " Services ",
    tab_tools: " Tools ",
    tab_scripts: " Scripts ",
    tab_settings: " Settings ",

    hint_switch: "Switch",
    hint_select: "Select",
    hint_add: "Add",
    hint_delete: "Delete",
    hint_start: "Start",
    hint_stop: "Stop",
    hint_start_all: "Start All",
    hint_stop_all: "Stop All",
    hint_refresh: "Refresh",
    hint_confirm: "Confirm",
    hint_quit: "Quit",

    svc_header: "Service Management",
    svc_selected: "Selected",
    svc_none: "None",
    svc_empty: "No services. Press 'a' to add",
    col_name: "Service Name",
    col_display: "Display Name",
    col_status: "Status",
    col_message: "Message",
    status_running: "Running",
    status_stopped: "Stopped",
    status_starting: "Starting...",
    status_stopping: "Stopping...",
    status_refreshing: "Refreshing...",

    tools_header: "System Tools",
    tool_sysinfo: "System Info",
    tool_sysinfo_desc: "View OS version, CPU, memory info",
    tool_diskclean: "Disk Cleanup",
    tool_diskclean_desc: "Clean temp files and system junk",
    tool_network: "Network Diagnostics",
    tool_network_desc: "Check network and DNS config",
    tool_process: "Process Manager",
    tool_process_desc: "View and manage system processes",
    tool_registry: "Registry Editor",
    tool_registry_desc: "Edit Windows registry",
    tool_eventlog: "Event Viewer",
    tool_eventlog_desc: "View system event logs",

    scripts_header: "Scripts",
    script_reset_navicat: "Reset Navicat",
    script_reset_navicat_desc: "Clean Navicat registration and CLSID",

    settings_header: "Settings",
    setting_language: "Language",
    setting_theme: "Theme",
    theme_dark: "Dark",
    theme_light: "Light",

    dialog_add_title: "Add Service",
    dialog_search: "Search",
    dialog_empty: "No matching services",
    dialog_hint_add: "Add",
    dialog_hint_cancel: "Cancel",

    err_permission: "Access denied",
    err_running: "Already running",
    err_not_started: "Not started",
    err_cannot_stop: "Cannot stop",
    err_timeout: "Timeout",
    err_not_found: "Service not found",
    err_failed: "Operation failed",
};
