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

        // 顶部信息栏
        let count = app.managed_services().len();
        let selected_info = match app.selected_service() {
            Some(idx) => format!("  选中: {}/{}", idx + 1, count),
            None => "  未选择".to_string(),
        };
        let header = Paragraph::new(format!("服务管理 ({}){}", count, selected_info))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, chunks[0]);

        // 服务表格
        if app.managed_services().is_empty() {
            let empty_msg = Paragraph::new("  暂无管理的服务，按 a 添加服务")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            frame.render_widget(empty_msg, chunks[1]);
        } else {
            let rows: Vec<Row> = app
                .managed_services()
                .iter()
                .enumerate()
                .map(|(i, svc)| {
                    let status = app.service_status(&svc.name);
                    let status_style = match status {
                        "Running" | "运行中" => Style::default().fg(Color::Green),
                        "Stopped" | "已停止" => Style::default().fg(Color::Red),
                        _ => Style::default().fg(Color::Yellow),
                    };
                    let status_text = match status {
                        "Running" => "运行中",
                        "Stopped" => "已停止",
                        other => other,
                    };
                    let enabled_text = if svc.enabled { "是" } else { "否" };

                    let row_style = if Some(i) == app.selected_service() {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(Span::raw(&svc.name)),
                        Cell::from(Span::raw(&svc.display_name)),
                        Cell::from(Span::styled(status_text, status_style)),
                        Cell::from(Span::raw(enabled_text)),
                    ])
                    .style(row_style)
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(30),
                    Constraint::Min(20),
                    Constraint::Length(10),
                    Constraint::Length(8),
                ],
            )
            .header(Row::new(vec![
                Cell::from(Span::styled(
                    "服务名称",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "显示名称",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "状态",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "启用",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

            frame.render_widget(table, chunks[1]);
        }
    }
}
