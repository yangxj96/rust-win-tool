use super::*;

impl AppState {
    pub fn managed_services(&self) -> &[ManagedService] {
        &self.managed_services
    }

    pub fn service_status(&self, name: &str) -> ServiceStatus {
        self.service_statuses
            .get(name)
            .copied()
            .unwrap_or(ServiceStatus::Unknown)
    }

    pub fn service_message(&self, name: &str) -> Option<&str> {
        self.service_messages.get(name).map(String::as_str)
    }

    pub fn service_search(&self) -> &str {
        self.service_search.text.as_str()
    }

    pub fn service_search_marked(&self) -> &str {
        self.service_search.marked.as_str()
    }

    pub fn service_filter(&self) -> ServiceFilter {
        self.service_filter
    }

    pub fn set_service_filter(&mut self, filter: ServiceFilter) {
        self.service_filter = filter;
    }

    pub fn service_sort(&self) -> ServiceSort {
        self.service_sort
    }

    pub fn cycle_service_sort(&mut self) {
        self.service_sort = self.service_sort.next();
    }

    pub fn service_search_text_utf16_len(&self) -> usize {
        self.service_search.utf16_len()
    }

    pub fn service_search_marked_range(&self) -> Option<Range<usize>> {
        self.service_search.marked_range()
    }

    pub fn service_search_text_range(&self, range: Range<usize>) -> String {
        self.service_search.text_range(range)
    }

    pub fn service_search_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.service_search.replace_range(range, text);
    }

    pub fn service_search_commit_text(&mut self, text: &str) {
        self.service_search.commit(text);
    }

    pub fn service_search_set_marked(&mut self, text: &str) {
        self.service_search.set_marked(text);
    }

    pub fn service_search_unmark(&mut self) {
        self.service_search.unmark();
    }

    pub fn service_search_backspace(&mut self) {
        self.service_search.backspace();
    }

    /// `managed_services` 中符合当前文本和状态筛选的索引，顺序遵循当前排序设置。
    pub fn filtered_service_indices(&self) -> Vec<usize> {
        let search = self.service_search.text.to_lowercase();
        let mut indices: Vec<usize> = self
            .managed_services
            .iter()
            .enumerate()
            .filter(|(_, service)| {
                let matches_search = search.is_empty()
                    || service.name.to_lowercase().contains(&search)
                    || service.display_name.to_lowercase().contains(&search);
                matches_search
                    && self
                        .service_filter
                        .matches(self.service_status(&service.name))
            })
            .map(|(index, _)| index)
            .collect();

        match self.service_sort {
            ServiceSort::Manual => {}
            ServiceSort::Name => indices
                .sort_by_key(|&index| self.managed_services[index].display_name.to_lowercase()),
            ServiceSort::Status => indices.sort_by_key(|&index| {
                let service = &self.managed_services[index];
                (
                    status_rank(self.service_status(&service.name)),
                    service.display_name.to_lowercase(),
                )
            }),
        }
        indices
    }

    pub fn record_history(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
        if self.history.len() > HISTORY_LIMIT {
            let excess = self.history.len() - HISTORY_LIMIT;
            self.history.drain(0..excess);
        }
    }

    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn service_detail_name(&self) -> Option<&str> {
        self.service_detail_name.as_deref()
    }

    pub fn service_detail(&self) -> Option<&ServiceDetails> {
        self.service_detail.as_ref()
    }

    pub fn service_detail_loading(&self) -> bool {
        self.service_detail_loading
    }

    pub fn service_detail_error(&self) -> Option<&str> {
        self.service_detail_error.as_deref()
    }

    pub fn begin_service_detail(&mut self, name: &str) {
        self.service_detail_name = Some(name.to_string());
        self.service_detail = None;
        self.service_detail_loading = true;
        self.service_detail_error = None;
        self.operation_state = OperationState::LoadingSystemInfo;
    }

    pub fn set_service_detail(&mut self, result: Result<ServiceDetails, BackendError>) {
        self.service_detail_loading = false;
        match result {
            Ok(details) => {
                self.service_detail = Some(details);
                self.service_detail_error = None;
                self.operation_state = OperationState::Idle;
            }
            Err(error) => {
                self.service_detail = None;
                self.service_detail_error = Some(map_error(&error, self.language));
                self.operation_state = OperationState::Error;
            }
        }
    }

    pub fn close_service_detail(&mut self) {
        self.service_detail_name = None;
        self.service_detail = None;
        self.service_detail_loading = false;
        self.service_detail_error = None;
    }

    pub fn apply_start_type(&mut self, start_type: StartType) {
        if let Some(details) = &mut self.service_detail {
            details.start_type = start_type;
        }
        self.service_detail_error = None;
    }

    pub fn set_service_detail_error(&mut self, error: &BackendError) {
        self.service_detail_error = Some(map_error(error, self.language));
    }

    pub fn request_refresh(&mut self) -> PendingAction {
        for service in &self.managed_services {
            self.service_statuses
                .insert(service.name.clone(), ServiceStatus::Refreshing);
        }
        self.operation_state = OperationState::Refreshing;
        PendingAction::RefreshAll(self.service_names())
    }

    pub fn request_start_service(&mut self, name: &str) -> Option<PendingAction> {
        let name = self
            .managed_services
            .iter()
            .find(|service| service.name == name)?
            .name
            .clone();
        self.service_statuses
            .insert(name.clone(), ServiceStatus::Starting);
        self.operation_state = OperationState::Starting;
        Some(PendingAction::StartService(name))
    }

    pub fn request_stop_service(&mut self, name: &str) -> Option<PendingAction> {
        let name = self
            .managed_services
            .iter()
            .find(|service| service.name == name)?
            .name
            .clone();
        self.service_statuses
            .insert(name.clone(), ServiceStatus::Stopping);
        self.operation_state = OperationState::Stopping;
        Some(PendingAction::StopService(name))
    }

    pub fn request_start_all(&mut self) -> PendingAction {
        let names = self.service_names();
        for name in &names {
            self.service_statuses
                .insert(name.clone(), ServiceStatus::Starting);
        }
        self.operation_state = OperationState::Starting;
        PendingAction::StartAll(names)
    }

    pub fn request_stop_all(&mut self) -> PendingAction {
        let names = self.service_names();
        for name in &names {
            self.service_statuses
                .insert(name.clone(), ServiceStatus::Stopping);
        }
        self.operation_state = OperationState::Stopping;
        PendingAction::StopAll(names)
    }

    pub fn apply_service_success(&mut self, name: &str, status: &str) {
        let status = ServiceStatus::from_backend(status);
        self.service_statuses.insert(name.to_string(), status);
        self.service_known_statuses.insert(name.to_string(), status);
        self.service_messages.remove(name);
    }

    pub fn apply_service_error(&mut self, name: &str, error: &BackendError) {
        // 保留后端最近确认的状态，避免启动或停止失败后列表项显示为未知；错误另通过提示
        // 信息呈现。
        let status = self
            .service_known_statuses
            .get(name)
            .copied()
            .unwrap_or(ServiceStatus::Unknown);
        self.service_statuses.insert(name.to_string(), status);
        self.service_messages
            .insert(name.to_string(), map_error(error, self.language));
        self.operation_state = OperationState::Error;
    }

    pub fn remove_service(&mut self, name: &str) {
        let Some(index) = self
            .managed_services
            .iter()
            .position(|service| service.name == name)
        else {
            return;
        };

        let name = self.managed_services.remove(index).name;
        self.service_statuses.remove(&name);
        self.service_known_statuses.remove(&name);
        self.service_messages.remove(&name);
        crate::features::services::save_managed_services(&self.data_file, &self.managed_services);
    }

    /// 用导入结果替换整个托管服务列表并保存到磁盘。
    pub fn replace_managed_services(&mut self, services: Vec<ManagedService>) {
        self.managed_services = services;
        self.service_statuses.clear();
        self.service_known_statuses.clear();
        self.service_messages.clear();
        crate::features::services::save_managed_services(&self.data_file, &self.managed_services);
    }

    pub fn begin_add_dialog(&mut self) {
        self.add_dialog = AddDialogState {
            show: true,
            loading: true,
            ..AddDialogState::default()
        };
        self.operation_state = OperationState::LoadingServices;
    }

    pub fn set_add_dialog_services(&mut self, result: Result<Vec<ServiceInfo>, BackendError>) {
        match result {
            Ok(services) => {
                self.add_dialog.services = services;
                self.add_dialog.loading = false;
                self.add_dialog.error = None;
                self.rebuild_add_dialog_filter();
                self.operation_state = OperationState::Idle;
            }
            Err(error) => {
                self.add_dialog.loading = false;
                self.add_dialog.error = Some(map_error(&error, self.language));
                self.operation_state = OperationState::Error;
            }
        }
    }

    pub fn show_add_dialog(&self) -> bool {
        self.add_dialog.show
    }

    pub fn add_dialog_loading(&self) -> bool {
        self.add_dialog.loading
    }

    pub fn add_dialog_error(&self) -> Option<&str> {
        self.add_dialog.error.as_deref()
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
        self.add_dialog.search.text.as_str()
    }

    /// 输入法正在组合的文本，显示在已提交的搜索文本之后。
    pub fn add_dialog_marked(&self) -> &str {
        self.add_dialog.search.marked.as_str()
    }

    /// 可编辑文本的 UTF-16 总长度，与平台输入协议采用的单位一致。
    pub fn add_dialog_text_utf16_len(&self) -> usize {
        self.add_dialog.search.utf16_len()
    }

    /// 输入法组合文本占用的 UTF-16 范围；没有组合文本时返回空值。
    pub fn add_dialog_marked_range(&self) -> Option<Range<usize>> {
        self.add_dialog.search.marked_range()
    }

    pub fn add_dialog_text_range(&self, range: Range<usize>) -> String {
        self.add_dialog.search.text_range(range)
    }

    /// 替换指定 UTF-16 范围的文本，并清除正在组合的输入法文本。
    pub fn add_dialog_replace_range(&mut self, range: Range<usize>, text: &str) {
        self.add_dialog.search.replace_range(range, text);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    /// 在光标处提交一段文本，例如按键字符或输入法结果。
    pub fn add_dialog_commit_text(&mut self, text: &str) {
        self.add_dialog.search.commit(text);
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    /// 开始或更新输入法组合文本。
    pub fn add_dialog_set_marked(&mut self, text: &str) {
        self.add_dialog.search.set_marked(text);
    }

    /// 提交当前尚未确认的输入法组合文本。
    pub fn add_dialog_unmark(&mut self) {
        self.add_dialog.search.unmark();
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn close_add_dialog(&mut self) {
        self.add_dialog = AddDialogState::default();
        if self.operation_state == OperationState::LoadingServices {
            self.operation_state = OperationState::Idle;
        }
    }

    pub fn add_dialog_backspace(&mut self) {
        self.add_dialog.search.backspace();
        self.add_dialog.selected = 0;
        self.rebuild_add_dialog_filter();
    }

    pub fn select_add_dialog(&mut self, index: usize) {
        if index < self.add_dialog.filtered.len() {
            self.add_dialog.selected = index;
        }
    }

    pub fn confirm_add_service(&mut self) -> bool {
        let Some(&real_index) = self.add_dialog.filtered.get(self.add_dialog.selected) else {
            return false;
        };
        let service = &self.add_dialog.services[real_index];
        if self
            .managed_services
            .iter()
            .any(|item| item.name == service.name)
        {
            self.close_add_dialog();
            return false;
        }

        self.managed_services.push(ManagedService {
            name: service.name.clone(),
            display_name: service.display_name.clone(),
            enabled: true,
        });
        crate::features::services::save_managed_services(&self.data_file, &self.managed_services);
        self.close_add_dialog();
        true
    }

    fn service_names(&self) -> Vec<String> {
        self.managed_services
            .iter()
            .map(|service| service.name.clone())
            .collect()
    }

    fn rebuild_add_dialog_filter(&mut self) {
        let search = self.add_dialog.search.text.to_lowercase();
        self.add_dialog.filtered = self
            .add_dialog
            .services
            .iter()
            .enumerate()
            .filter(|(_, service)| {
                search.is_empty()
                    || service.name.to_lowercase().contains(&search)
                    || service.display_name.to_lowercase().contains(&search)
            })
            .map(|(index, _)| index)
            .collect();
    }
}

fn status_rank(status: ServiceStatus) -> u8 {
    match status {
        ServiceStatus::Running => 0,
        ServiceStatus::Starting | ServiceStatus::Stopping | ServiceStatus::Refreshing => 1,
        ServiceStatus::Stopped => 2,
        ServiceStatus::Unknown => 3,
    }
}
