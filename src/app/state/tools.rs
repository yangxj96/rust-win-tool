use super::*;

impl AppState {
    pub fn processes(&self) -> &[ProcessInfo] {
        &self.processes
    }

    pub fn processes_loading(&self) -> bool {
        self.processes_loading
    }

    pub fn processes_error(&self) -> Option<&str> {
        self.processes_error.as_deref()
    }

    pub fn process_sort(&self) -> ProcessSort {
        self.process_sort
    }

    pub fn set_process_sort(&mut self, sort: ProcessSort) {
        self.process_sort = sort;
    }

    pub fn begin_process_refresh(&mut self) {
        self.processes_loading = true;
        self.processes_error = None;
        self.operation_state = OperationState::LoadingSystemInfo;
    }

    pub fn set_processes(&mut self, result: Result<Vec<ProcessInfo>, BackendError>) {
        self.processes_loading = false;
        match result {
            Ok(processes) => {
                self.processes = processes;
                self.processes_error = None;
                self.operation_state = OperationState::Idle;
            }
            Err(error) => {
                self.processes_error = Some(map_error(&error, self.language));
                self.operation_state = OperationState::Error;
            }
        }
    }

    pub fn show_port_lookup(&self) -> bool {
        self.port_lookup.open
    }

    pub fn open_port_lookup(&mut self) {
        self.port_lookup.open = true;
        self.port_lookup.query = SearchText::default();
        self.port_lookup.port = None;
        self.port_lookup.loading = false;
        self.port_lookup.error = None;
        self.port_lookup.results.clear();
        self.port_lookup.searched = false;
    }

    pub fn close_port_lookup(&mut self) {
        self.port_lookup = PortLookupState::default();
    }

    pub fn port_lookup_query(&self) -> &str {
        self.port_lookup.query.text.as_str()
    }

    pub fn port_lookup_query_marked(&self) -> &str {
        self.port_lookup.query.marked.as_str()
    }

    pub fn port_lookup_query_utf16_len(&self) -> usize {
        self.port_lookup.query.utf16_len()
    }

    pub fn port_lookup_query_marked_range(&self) -> Option<Range<usize>> {
        self.port_lookup.query.marked_range()
    }

    pub fn port_lookup_query_text_range(&self, range: Range<usize>) -> String {
        self.port_lookup.query.text_range(range)
    }

    pub fn port_lookup_query_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.port_lookup.query.replace_range(range, text);
        self.port_lookup.error = None;
    }

    pub fn port_lookup_query_commit_text(&mut self, text: &str) {
        self.port_lookup.query.commit(text);
        self.port_lookup.error = None;
    }

    pub fn port_lookup_query_set_marked(&mut self, text: &str) {
        self.port_lookup.query.set_marked(text);
        self.port_lookup.error = None;
    }

    pub fn port_lookup_query_unmark(&mut self) {
        self.port_lookup.query.unmark();
        self.port_lookup.error = None;
    }

    pub fn port_lookup_query_backspace(&mut self) {
        self.port_lookup.query.backspace();
        self.port_lookup.error = None;
    }

    pub fn port_lookup_loading(&self) -> bool {
        self.port_lookup.loading
    }

    pub fn port_lookup_error(&self) -> Option<&str> {
        self.port_lookup.error.as_deref()
    }

    pub fn port_lookup_results(&self) -> &[PortEntry] {
        &self.port_lookup.results
    }

    pub fn port_lookup_searched(&self) -> bool {
        self.port_lookup.searched
    }

    /// 已执行查询时，对话框当前展示的端口号。
    pub fn port_lookup_port(&self) -> Option<u16> {
        if self.port_lookup.open && self.port_lookup.searched {
            self.port_lookup.port
        } else {
            None
        }
    }

    /// 验证输入的端口并开始查询。输入有效时返回待查询端口；无效时在对话框中
    /// 显示错误并返回 `None`。
    pub fn begin_port_lookup(&mut self) -> Option<u16> {
        let translations = self.t();
        let raw = self.port_lookup.query.full_text();
        let trimmed = raw.trim();
        let port = match trimmed.parse::<u32>() {
            Ok(value) if (1..=65535).contains(&value) => value as u16,
            _ => {
                self.port_lookup.error = Some(if trimmed.is_empty() {
                    translations.proc_port_required.to_string()
                } else {
                    translations.proc_port_invalid.to_string()
                });
                self.port_lookup.loading = false;
                self.port_lookup.searched = false;
                self.port_lookup.results.clear();
                return None;
            }
        };
        self.port_lookup.error = None;
        self.port_lookup.loading = true;
        self.port_lookup.searched = true;
        self.port_lookup.port = Some(port);
        self.port_lookup.results.clear();
        Some(port)
    }

    pub fn set_port_lookup(&mut self, result: Result<Vec<PortEntry>, BackendError>) {
        self.port_lookup.loading = false;
        match result {
            Ok(entries) => {
                self.port_lookup.results = entries;
                self.port_lookup.error = None;
            }
            Err(error) => {
                self.port_lookup.results.clear();
                self.port_lookup.error = Some(map_error(&error, self.language));
            }
        }
    }

    pub fn monitor(&self) -> &MonitorState {
        &self.monitor
    }

    pub fn monitor_active(&self) -> bool {
        self.monitor.active
    }

    pub fn push_metrics(&mut self, metrics: Metrics) {
        self.monitor.cpu_history.push(metrics.cpu_percent);
        self.monitor.memory_history.push(metrics.memory_percent);
        if self.monitor.cpu_history.len() > MONITOR_HISTORY {
            self.monitor.cpu_history.remove(0);
        }
        if self.monitor.memory_history.len() > MONITOR_HISTORY {
            self.monitor.memory_history.remove(0);
        }
        self.monitor.latest = Some(metrics);
    }

    pub fn net_host(&self) -> &str {
        self.network.host.text.as_str()
    }

    pub fn net_host_marked(&self) -> &str {
        self.network.host.marked.as_str()
    }

    pub fn net_host_text_utf16_len(&self) -> usize {
        self.network.host.utf16_len()
    }

    pub fn net_host_marked_range(&self) -> Option<Range<usize>> {
        self.network.host.marked_range()
    }

    pub fn net_host_text_range(&self, range: Range<usize>) -> String {
        self.network.host.text_range(range)
    }

    pub fn net_host_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.network.host.replace_range(range, text);
    }

    pub fn net_host_commit_text(&mut self, text: &str) {
        self.network.host.commit(text);
    }

    pub fn net_host_set_marked(&mut self, text: &str) {
        self.network.host.set_marked(text);
    }

    pub fn net_host_unmark(&mut self) {
        self.network.host.unmark();
    }

    pub fn net_host_backspace(&mut self) {
        self.network.host.backspace();
    }

    pub fn net_port(&self) -> u16 {
        self.network.port
    }

    pub fn set_net_port(&mut self, port: u16) {
        self.network.port = port;
    }

    pub fn net_result(&self) -> Option<&str> {
        self.network.result.as_deref()
    }

    pub fn net_loading(&self) -> bool {
        self.network.loading
    }

    pub fn begin_net_check(&mut self) {
        self.network.loading = true;
        self.network.result = None;
        self.operation_state = OperationState::LoadingSystemInfo;
    }

    pub fn set_net_result(&mut self, result: String) {
        self.network.loading = false;
        self.network.result = Some(result);
        self.operation_state = OperationState::Idle;
    }

    pub fn startup(&self) -> &StartupState {
        &self.startup
    }

    pub fn begin_startup_refresh(&mut self) {
        self.startup.loading = true;
        self.startup.error = None;
        self.operation_state = OperationState::LoadingSystemInfo;
    }

    pub fn set_startup(&mut self, result: Result<Vec<StartupItem>, BackendError>) {
        self.startup.loading = false;
        match result {
            Ok(items) => {
                self.startup.items = items;
                self.startup.error = None;
                self.operation_state = OperationState::Idle;
            }
            Err(error) => {
                self.startup.error = Some(map_error(&error, self.language));
                self.operation_state = OperationState::Error;
            }
        }
    }

    /// 按当前排序设置排列的进程列表。
    pub fn sorted_processes(&self) -> Vec<&ProcessInfo> {
        let mut list: Vec<&ProcessInfo> = self.processes.iter().collect();
        match self.process_sort {
            ProcessSort::Memory => {
                list.sort_by_key(|process| std::cmp::Reverse(process.memory_bytes))
            }
            ProcessSort::Name => list.sort_by_key(|process| process.name.to_lowercase()),
            ProcessSort::Pid => list.sort_by_key(|process| process.pid),
        }
        list
    }

    pub fn tools_selected(&self) -> usize {
        self.tools_selected
    }

    pub fn select_tool(&mut self, index: usize) {
        if index < TOOLS_COUNT {
            self.tools_selected = index;
        }
    }

    pub fn tool_detail_active(&self) -> bool {
        self.tool_detail_active
    }

    pub fn open_tool_detail(&mut self) -> bool {
        self.tool_detail_active = true;
        match self.tools_selected {
            0 => {
                if self.system_info.is_none() {
                    self.system_info_loading = true;
                    self.operation_state = OperationState::LoadingSystemInfo;
                }
            }
            1 => {
                self.processes_loading = true;
                self.processes_error = None;
                self.operation_state = OperationState::LoadingSystemInfo;
            }
            2 => {
                self.monitor.active = true;
            }
            3 => {}
            4 => {
                self.startup.loading = true;
                self.startup.error = None;
                self.operation_state = OperationState::LoadingSystemInfo;
            }
            _ => {}
        }
        true
    }

    pub fn close_tool_detail(&mut self) {
        self.tool_detail_active = false;
        self.release_tool_data();
    }

    pub fn system_info(&self) -> Option<&SystemInfo> {
        self.system_info.as_ref()
    }

    pub fn system_info_loading(&self) -> bool {
        self.system_info_loading
    }

    /// 重新获取系统信息，并再次显示加载状态。
    pub fn request_system_info_refresh(&mut self) {
        self.system_info = None;
        self.system_info_loading = true;
        self.operation_state = OperationState::LoadingSystemInfo;
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
}
