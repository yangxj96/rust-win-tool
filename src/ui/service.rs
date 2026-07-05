use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table};
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(header, chunks[0]);

        // 服务表格
        if app.managed_services().is_empty() {
            let empty_msg = Paragraph::new("  暂无管理的服务，按 a 添加服务")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
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
                    let msg = app.service_message(&svc.name);
                    let msg_style = if msg.is_empty() {
                        Style::default()
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    let row_style = if Some(i) == app.selected_service() {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(Span::raw(&svc.name)),
                        Cell::from(Span::raw(&svc.display_name)),
                        Cell::from(Span::styled(status_text, status_style)),
                        Cell::from(Span::styled(msg, msg_style)),
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
                    Constraint::Min(0),
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
                    "消息",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );

            frame.render_widget(table, chunks[1]);
        }

        // 渲染添加服务对话框
        if app.show_add_dialog() {
            render_add_dialog(frame, area, app);
        }
    }
}

fn render_add_dialog(frame: &mut Frame, area: Rect, app: &App) {
    // 居中弹出框
    let popup_area = centered_rect(70, 70, area);

    // 清除背景
    frame.render_widget(Clear, popup_area);

    let inner_chunks = Layout::vertical([
        Constraint::Length(3),  // 搜索框
        Constraint::Min(0),    // 服务列表
        Constraint::Length(1), // 提示
    ])
    .margin(1)
    .split(popup_area);

    // 对话框边框
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 添加服务 ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block, popup_area);

    // 搜索框
    let search_text = format!(" 搜索: {}", app.add_dialog_search());
    let search_box = Paragraph::new(search_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(search_box, inner_chunks[0]);

    // 服务列表
    let filtered = app.add_dialog_filtered();
    let all_services = app.add_dialog_services();

    if filtered.is_empty() {
        let empty_msg = Paragraph::new("  没有匹配的服务")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty_msg, inner_chunks[1]);
    } else {
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(i, &real_idx)| {
                let svc = &all_services[real_idx];
                let status_style = match svc.status.as_str() {
                    "Running" => Style::default().fg(Color::Green),
                    "Stopped" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Yellow),
                };
                let status_text = match svc.status.as_str() {
                    "Running" => "运行中",
                    "Stopped" => "已停止",
                    other => other,
                };

                let row_style = if i == app.add_dialog_selected() {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(Span::raw(&svc.name)),
                    Cell::from(Span::raw(&svc.display_name)),
                    Cell::from(Span::styled(status_text, status_style)),
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
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(table, inner_chunks[1]);
    }

    // 底部提示
    let hint = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" 添加 "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" 取消 "),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" 选择 "),
        Span::styled(
            format!(" {}/{}", app.add_dialog_selected() + 1, filtered.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let hint_bar = Paragraph::new(hint)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
    frame.render_widget(hint_bar, Rect {
        x: inner_chunks[2].x,
        y: inner_chunks[2].y,
        width: inner_chunks[2].width,
        height: inner_chunks[2].height + 2,
    });
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
