use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct SettingsView;

impl SettingsView {
    pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);
        
        // Header
        let header = Paragraph::new("Settings")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Settings"));
        frame.render_widget(header, chunks[0]);
        
        // Settings content
        let settings = vec![
            Line::from(vec![
                Span::styled("Theme: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("Dark"),
            ]),
            Line::from(vec![
                Span::styled("Language: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("English"),
            ]),
            Line::from(vec![
                Span::styled("Notifications: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("Enabled"),
            ]),
        ];
        
        let settings_block = Paragraph::new(settings)
            .block(Block::default().borders(Borders::ALL).title("Configuration"));
        frame.render_widget(settings_block, chunks[1]);
    }
}