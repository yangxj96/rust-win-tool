use super::*;

impl AppState {
    pub fn cleanup(&self) -> &CleanupState {
        &self.cleanup
    }

    pub fn toggle_cleanup_row(&mut self, index: usize) {
        if let Some(row) = self.cleanup.rows.get_mut(index) {
            row.selected = !row.selected;
        }
    }

    pub fn set_cleanup_delete_mode(&mut self, mode: DeleteMode) {
        self.cleanup.delete_mode = mode;
    }

    pub fn begin_cleanup_scan(&mut self) {
        self.cleanup.scanning = true;
        self.cleanup.message = None;
        self.operation_state = OperationState::ScanningCleanup;
    }

    pub fn apply_cleanup_scan(
        &mut self,
        scans: Vec<(CleanupCategory, Scan)>,
        recycle_bin: Option<(u64, u64)>,
    ) {
        for (category, scan) in scans {
            if let Some(row) = self
                .cleanup
                .rows
                .iter_mut()
                .find(|row| row.category == category)
            {
                row.scan = Some(scan);
            }
        }
        self.cleanup.recycle_bin = recycle_bin;
        self.cleanup.scanning = false;
        self.cleanup.cleaning = false;
        self.operation_state = OperationState::Idle;
    }

    pub fn begin_cleanup(&mut self) {
        self.cleanup.cleaning = true;
        self.cleanup.message = None;
        self.operation_state = OperationState::CleaningCleanup;
    }

    pub fn set_cleanup_message(&mut self, message: String) {
        self.cleanup.message = Some(message);
    }

    pub fn apply_cleanup_result(
        &mut self,
        scans: Vec<(CleanupCategory, Scan)>,
        recycle_bin: Option<(u64, u64)>,
        message: String,
    ) {
        for (category, scan) in scans {
            if let Some(row) = self
                .cleanup
                .rows
                .iter_mut()
                .find(|row| row.category == category)
            {
                row.scan = Some(scan);
            }
        }
        self.cleanup.recycle_bin = recycle_bin;
        self.cleanup.message = Some(message);
        self.cleanup.scanning = false;
        self.cleanup.cleaning = false;
        self.operation_state = OperationState::Idle;
    }

    pub fn selected_cleanup_categories(&self) -> Vec<CleanupCategory> {
        self.cleanup
            .rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.category)
            .collect()
    }

    /// 当前选中清理类别的总大小和文件数量。
    pub fn selected_cleanup_totals(&self) -> Scan {
        let mut total = Scan::default();
        for row in &self.cleanup.rows {
            if !row.selected {
                continue;
            }
            if let Some(scan) = row.scan {
                total.bytes += scan.bytes;
                total.files += scan.files;
            }
        }
        total
    }
}
