use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;

pub struct ServiceView;

impl ServiceView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);
        
        // Header
        let header = Paragraph::new(format!(
            "Managed Services ({})",
            app.managed_services().len()
        ))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Services"));
        frame.render_widget(header, chunks[0]);
        
        // Services table
        let rows: Vec<Row> = app
            .managed_services()
            .iter()
            .enumerate()
            .map(|(i, svc)| {
                let status = app.service_status(&svc.name);
                let status_style = match status {
                    "Running" => Style::default().fg(Color::Green),
                    "Stopped" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
                };
                
                Row::new(vec![
                    Cell::from(Span::raw(&svc.name)),
                    Cell::from(Span::raw(&svc.display_name)),
                    Cell::from(Span::styled(status, status_style)),
                    Cell::from(Span::raw(if svc.enabled { "Yes" } else { "No" })),
                ])
                .style(if Some(i) == app.selected_service() {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
            })
            .collect();
        
        let table = Table::new(
            rows,
            [
                Constraint::Length(30),
                Constraint::Min(20),
                Constraint::Length(15),
                Constraint::Length(10),
            ],
        )
        .header(Row::new(vec![
            Cell::from(Span::styled("Name", Style::default().add_modifier(Modifier::BOLD))),
            Cell::from(Span::styled("Display Name", Style::default().add_modifier(Modifier::BOLD))),
            Cell::from(Span::styled("Status", Style::default().add_modifier(Modifier::BOLD))),
            Cell::from(Span::styled("Enabled", Style::default().add_modifier(Modifier::BOLD))),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Service List"));
        
        frame.render_widget(table, chunks[1]);
    }
}