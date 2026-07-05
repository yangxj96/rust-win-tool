use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use winreg::enums::*;
use winreg::RegKey;

use crate::app::App;

pub enum ScriptAction {
    ResetNavicat,
}

pub struct Script {
    pub action: ScriptAction,
}

pub const SCRIPTS: &[Script] = &[
    Script {
        action: ScriptAction::ResetNavicat,
    },
];

pub fn reset_navicat(lang: crate::i18n::Language) -> Result<String, String> {
    use crate::i18n::{Language, EN, ZH};
    let t = match lang {
        Language::Chinese => &ZH,
        Language::English => &EN,
    };
    let hku = RegKey::predef(HKEY_CURRENT_USER);

    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Registration17XCS");
    let _ = hku.delete_subkey_all(r"Software\PremiumSoft\NavicatPremium\Update");

    let clsid_path = r"Software\Classes\CLSID";
    let clsid = hku
        .open_subkey_with_flags(clsid_path, KEY_READ)
        .map_err(|e| t.script_err_open_clsid.replace("{}", &e.to_string()))?;

    let mut deleted = 0u32;
    for key_name in clsid.enum_keys().filter_map(|k| k.ok()) {
        let full_path = format!(r"{}\{}", clsid_path, key_name);
        if should_delete_key(&hku, &full_path) {
            let _ = hku.delete_subkey_all(&full_path);
            deleted += 1;
        }
    }

    Ok(t.script_result_cleanup.replace("{}", &deleted.to_string()))
}

fn should_delete_key(hku: &RegKey, path: &str) -> bool {
    if let Ok(subkey) = hku.open_subkey_with_flags(path, KEY_READ) {
        for value in subkey.enum_values().filter_map(|v| v.ok()) {
            if value.0.contains("Info") || value.0.contains("ShellFolder") {
                return true;
            }
        }
        for subkey_name in subkey.enum_keys().filter_map(|k| k.ok()) {
            if subkey_name.contains("Info") || subkey_name.contains("ShellFolder") {
                return true;
            }
        }
    }
    false
}

pub struct ScriptsView;

impl ScriptsView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let t = app.t();
        let c = app.theme_colors();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

        let header = Paragraph::new(t.scripts_header)
            .style(
                Style::default()
                    .fg(c.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(c.border)),
            );
        frame.render_widget(header, chunks[0]);

        let list_items: Vec<ListItem> = SCRIPTS
            .iter()
            .enumerate()
            .map(|(i, script)| {
                let name = match script.action {
                    ScriptAction::ResetNavicat => t.script_reset_navicat,
                };
                let desc = match script.action {
                    ScriptAction::ResetNavicat => t.script_reset_navicat_desc,
                };

                let style = if i == app.scripts_selected() {
                    Style::default().fg(c.fg_select).bg(c.bg_select)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {}. ", i + 1),
                        Style::default().fg(c.accent),
                    ),
                    Span::styled(
                        name,
                        Style::default()
                            .fg(c.fg_default)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", desc),
                        Style::default().fg(c.inactive),
                    ),
                ]))
                .style(style)
            })
            .collect();

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(c.border)),
        );
        frame.render_widget(list, chunks[1]);

        // 显示执行结果
        if let Some(result) = app.script_result() {
            let result_line = Line::from(Span::styled(
                format!("  {}", result),
                Style::default().fg(c.accent),
            ));
            let result_bar = Paragraph::new(result_line).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", t.hint_confirm))
                    .border_style(Style::default().fg(c.border)),
            );
            // 在列表下方显示结果
            let result_area = Rect {
                x: chunks[1].x,
                y: chunks[1].y + chunks[1].height.saturating_sub(3),
                width: chunks[1].width,
                height: 3,
            };
            frame.render_widget(result_bar, result_area);
        }
    }
}
