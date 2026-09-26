use super::*;

impl AppState {
    pub fn scripts_selected(&self) -> usize {
        self.scripts_selected
    }

    pub fn select_script(&mut self, index: usize) {
        if index < self.script_count() {
            self.scripts_selected = index;
        }
    }

    /// 脚本列表行数，包含内置脚本和用户脚本。
    pub fn script_count(&self) -> usize {
        SCRIPTS_COUNT + self.custom_scripts.len()
    }

    pub fn custom_scripts(&self) -> &[CustomScript] {
        &self.custom_scripts
    }

    pub fn custom_script(&self, index: usize) -> Option<&CustomScript> {
        self.custom_scripts.get(index)
    }

    pub fn show_script_dialog(&self) -> bool {
        self.script_dialog.is_some()
    }

    pub fn begin_add_script(&mut self) {
        self.script_dialog = Some(ScriptDialogState::default());
    }

    pub fn close_add_script(&mut self) {
        self.script_dialog = None;
    }

    pub fn script_dialog_name(&self) -> &str {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.name.text.as_str())
            .unwrap_or("")
    }

    pub fn script_dialog_name_marked(&self) -> &str {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.name.marked.as_str())
            .unwrap_or("")
    }

    pub fn script_dialog_name_utf16_len(&self) -> usize {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.name.utf16_len())
            .unwrap_or(0)
    }

    pub fn script_dialog_name_marked_range(&self) -> Option<Range<usize>> {
        self.script_dialog
            .as_ref()
            .and_then(|dialog| dialog.name.marked_range())
    }

    pub fn script_dialog_name_text_range(&self, range: Range<usize>) -> String {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.name.text_range(range))
            .unwrap_or_default()
    }

    pub fn script_dialog_name_replace_range(&mut self, range: Range<usize>, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.name.replace_range(range, text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_name_commit_text(&mut self, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.name.commit(text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_name_set_marked(&mut self, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.name.set_marked(text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_name_unmark(&mut self) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.name.unmark();
            dialog.error = None;
        }
    }

    pub fn script_dialog_name_backspace(&mut self) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.name.backspace();
            dialog.error = None;
        }
    }

    pub fn script_dialog_content(&self) -> &str {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.content.text.as_str())
            .unwrap_or("")
    }

    pub fn script_dialog_content_marked(&self) -> &str {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.content.marked.as_str())
            .unwrap_or("")
    }

    pub fn script_dialog_content_utf16_len(&self) -> usize {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.content.utf16_len())
            .unwrap_or(0)
    }

    pub fn script_dialog_content_marked_range(&self) -> Option<Range<usize>> {
        self.script_dialog
            .as_ref()
            .and_then(|dialog| dialog.content.marked_range())
    }

    pub fn script_dialog_content_text_range(&self, range: Range<usize>) -> String {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.content.text_range(range))
            .unwrap_or_default()
    }

    pub fn script_dialog_content_replace_range(&mut self, range: Range<usize>, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.replace_range(range, text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_content_commit_text(&mut self, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.commit(text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_content_set_marked(&mut self, text: &str) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.set_marked(text);
            dialog.error = None;
        }
    }

    pub fn script_dialog_content_unmark(&mut self) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.unmark();
            dialog.error = None;
        }
    }

    pub fn script_dialog_content_backspace(&mut self) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.backspace();
            dialog.error = None;
        }
    }

    /// 在多行脚本正文中插入换行符。
    pub fn script_dialog_content_newline(&mut self) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.content.text.push('\n');
            dialog.content.marked.clear();
            dialog.error = None;
        }
    }

    pub fn script_dialog_kind(&self) -> ScriptKind {
        self.script_dialog
            .as_ref()
            .map(|dialog| dialog.kind)
            .unwrap_or_default()
    }

    pub fn set_script_dialog_kind(&mut self, kind: ScriptKind) {
        if let Some(dialog) = self.script_dialog.as_mut() {
            dialog.kind = kind;
            dialog.error = None;
        }
    }

    pub fn script_dialog_error(&self) -> Option<&str> {
        self.script_dialog
            .as_ref()
            .and_then(|dialog| dialog.error.as_deref())
    }

    /// 验证并保存用户脚本；验证错误会显示在对话框中。
    pub fn confirm_add_script(&mut self) {
        let translations = self.t();
        let name_error = translations.script_err_name;
        let content_error = translations.script_err_content;
        let duplicate_error = translations.script_err_duplicate;
        let syntax_error = translations.script_err_syntax;

        let Some(dialog) = self.script_dialog.as_mut() else {
            return;
        };
        let name = dialog.name.full_text().trim().to_string();
        let content = dialog.content.full_text();
        if name.is_empty() {
            dialog.error = Some(name_error.to_string());
            return;
        }
        if content.trim().is_empty() {
            dialog.error = Some(content_error.to_string());
            return;
        }
        if self
            .custom_scripts
            .iter()
            .any(|script| script.name.eq_ignore_ascii_case(&name))
        {
            dialog.error = Some(duplicate_error.to_string());
            return;
        }
        if dialog.kind == ScriptKind::PowerShell && !balanced(&content) {
            dialog.error = Some(syntax_error.to_string());
            return;
        }

        let kind = dialog.kind;
        self.custom_scripts.push(CustomScript {
            name,
            kind,
            command: content,
        });
        crate::features::scripts::save(&self.scripts_file, &self.custom_scripts);
        self.script_dialog = None;
    }

    pub fn remove_custom_script(&mut self, index: usize) {
        if index >= self.custom_scripts.len() {
            return;
        }
        self.custom_scripts.remove(index);
        if self.scripts_selected >= self.script_count() {
            self.scripts_selected = 0;
        }
        crate::features::scripts::save(&self.scripts_file, &self.custom_scripts);
    }

    pub fn begin_script(&mut self) {
        self.script_output = None;
        self.script_running = true;
        self.operation_state = OperationState::RunningScript;
    }

    pub fn script_running(&self) -> bool {
        self.script_running
    }

    pub fn script_output(&self) -> Option<&str> {
        self.script_output.as_deref()
    }

    pub fn close_script_output(&mut self) {
        self.script_output = None;
    }

    pub fn set_script_output(&mut self, result: Result<String, BackendError>) {
        self.script_running = false;
        self.script_output = Some(match result {
            Ok(text) if text.trim().is_empty() => self.t().script_no_output.to_string(),
            Ok(text) => text,
            Err(error) => map_error(&error, self.language),
        });
        self.operation_state = OperationState::Idle;
    }
}

/// 对 PowerShell 脚本做轻量语法检查：字符串之外的括号、大括号和引号必须配对。
pub(super) fn balanced(text: &str) -> bool {
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    for ch in text.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => paren += 1,
            ')' if !in_single && !in_double => paren -= 1,
            '{' if !in_single && !in_double => brace += 1,
            '}' if !in_single && !in_double => brace -= 1,
            _ => {}
        }
        if paren < 0 || brace < 0 {
            return false;
        }
    }
    paren == 0 && brace == 0 && !in_single && !in_double
}
