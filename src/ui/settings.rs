use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct SettingsView;

impl SettingsView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

        // 顶部信息栏
        let header = Paragraph::new("设置")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, chunks[0]);

        // 设置列表
        let items = [
            ("主题切换", "深色模式"),
            ("语言选择", "简体中文"),
            ("通知设置", "启用"),
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
