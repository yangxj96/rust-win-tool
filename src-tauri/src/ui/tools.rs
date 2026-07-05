use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct ToolsView;

impl ToolsView {
    pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);
        
        // Header
        let header = Paragraph::new("System Tools")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Tools"));
        frame.render_widget(header, chunks[0]);
        
        // Tools list
        let tools = vec![
            ListItem::new(Line::from(vec![
                Span::styled("1. ", Style::default().fg(Color::Yellow)),
                Span::raw("System Information"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("2. ", Style::default().fg(Color::Yellow)),
                Span::raw("Disk Cleanup"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("3. ", Style::default().fg(Color::Yellow)),
                Span::raw("Network Diagnostics"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("4. ", Style::default().fg(Color::Yellow)),
                Span::raw("Process Manager"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("5. ", Style::default().fg(Color::Yellow)),
                Span::raw("Registry Editor"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("6. ", Style::default().fg(Color::Yellow)),
                Span::raw("Event Viewer"),
            ])),
        ];
        
        let tools_list = List::new(tools)
            .block(Block::default().borders(Borders::ALL).title("Available Tools"));
        frame.render_widget(tools_list, chunks[1]);
    }
}