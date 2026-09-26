use super::*;

impl AppState {
    pub fn t(&self) -> &'static Translations {
        translations_for(self.language)
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// 直接切换语言，供设置页的分段选择控件调用。
    pub fn set_language(&mut self, language: Language) {
        if self.language != language {
            self.language = language;
            save_settings(&self.settings_file, self.language, self.theme);
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn theme_colors(&self) -> &'static ThemeColors {
        match self.theme {
            Theme::Dark => &DARK,
            Theme::Light => &LIGHT,
        }
    }

    /// 直接切换主题，供设置页的分段选择控件调用。
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            save_settings(&self.settings_file, self.language, self.theme);
        }
    }
}

pub(super) fn translations_for(language: Language) -> &'static Translations {
    match language {
        Language::Chinese => &ZH,
        Language::English => &EN,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct AppSettings {
    pub(super) language: Language,
    pub(super) theme: Theme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Chinese,
            theme: Theme::Dark,
        }
    }
}

pub(super) fn load_settings(path: &Path) -> AppSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn save_settings(path: &Path, language: Language, theme: Theme) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&AppSettings { language, theme }) {
        let _ = std::fs::write(path, json);
    }
}
