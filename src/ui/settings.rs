use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct SettingsView;

impl SettingsView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let t = app.t();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

        let header = Paragraph::new(t.settings_header)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, chunks[0]);

        let lang_value = app.language().name();
        let theme_value = match app.theme() {
            "light" => t.theme_light,
            _ => t.theme_dark,
        };

        let items = [
            (t.setting_language, lang_value),
            (t.setting_theme, theme_value),
        ];

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, (name, value))| {
                let style = if i == app.settings_selected() {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {}: ", name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(*value),
                ]))
                .style(style)
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );

        frame.render_widget(list, chunks[1]);
    }
}
