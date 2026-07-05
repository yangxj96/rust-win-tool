use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;

pub struct ServiceView;

impl ServiceView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let t = app.t();
        let c = app.theme_colors();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

        let count = app.managed_services().len();
        let selected_info = match app.selected_service() {
            Some(idx) => format!("  {}: {}/{}", t.svc_selected, idx + 1, count),
            None => format!("  {}: {}", t.svc_selected, t.svc_none),
        };
        let header = Paragraph::new(format!("{} ({}){}", t.svc_header, count, selected_info))
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

        if app.managed_services().is_empty() {
            let empty_msg = Paragraph::new(t.svc_empty)
                .style(Style::default().fg(c.inactive))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(c.border)),
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
                        "Running" => Style::default().fg(c.running),
                        "Stopped" => Style::default().fg(c.stopped),
                        _ => Style::default().fg(c.accent),
                    };
                    let status_text = match status {
                        "Running" => t.status_running,
                        "Stopped" => t.status_stopped,
                        other => other,
                    };
                    let msg = app.service_message(&svc.name);
                    let msg_style = if msg.is_empty() {
                        Style::default()
                    } else {
                        Style::default().fg(c.error)
                    };

                    let row_style = if Some(i) == app.selected_service() {
                        Style::default().fg(c.fg_select).bg(c.bg_select)
                    } else {
                        Style::default()
                    };

                    Row::new(vec![
                        Cell::from(Span::styled(&svc.name, Style::default().fg(c.fg_default))),
                        Cell::from(Span::styled(&svc.display_name, Style::default().fg(c.fg_default))),
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
                    t.col_name,
                    Style::default()
                        .fg(c.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    t.col_display,
                    Style::default()
                        .fg(c.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    t.col_status,
                    Style::default()
                        .fg(c.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    t.col_message,
                    Style::default()
                        .fg(c.accent)
                        .add_modifier(Modifier::BOLD),
                )),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(c.border)),
            );

            frame.render_widget(table, chunks[1]);
        }

        if app.show_add_dialog() {
            render_add_dialog(frame, area, app);
        }
    }
}

fn render_add_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.t();
    let c = app.theme_colors();
    let popup_area = centered_rect(70, 70, area);

    frame.render_widget(
        Block::default().style(Style::default().bg(c.bg_terminal)),
        popup_area,
    );

    let inner_chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .margin(1)
    .split(popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", t.dialog_add_title))
        .title_style(
            Style::default()
                .fg(c.primary)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(c.primary));
    frame.render_widget(block, popup_area);

    let search_text = format!(" {}: {}", t.dialog_search, app.add_dialog_search());
    let search_box = Paragraph::new(search_text)
        .style(Style::default().fg(c.fg_default))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(c.border)),
        );
    frame.render_widget(search_box, inner_chunks[0]);

    let filtered = app.add_dialog_filtered();
    let all_services = app.add_dialog_services();

    if filtered.is_empty() {
        let empty_msg = Paragraph::new(t.dialog_empty)
            .style(Style::default().fg(c.inactive))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(c.border)),
            );
        frame.render_widget(empty_msg, inner_chunks[1]);
    } else {
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(i, &real_idx)| {
                let svc = &all_services[real_idx];
                let status_style = match svc.status.as_str() {
                    "Running" => Style::default().fg(c.running),
                    "Stopped" => Style::default().fg(c.stopped),
                    _ => Style::default().fg(c.accent),
                };
                let status_text = match svc.status.as_str() {
                    "Running" => t.status_running,
                    "Stopped" => t.status_stopped,
                    other => other,
                };

                let row_style = if i == app.add_dialog_selected() {
                    Style::default().fg(c.fg_select).bg(c.bg_select)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(Span::styled(&svc.name, Style::default().fg(c.fg_default))),
                    Cell::from(Span::styled(&svc.display_name, Style::default().fg(c.fg_default))),
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
                t.col_name,
                Style::default()
                    .fg(c.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                t.col_display,
                Style::default()
                    .fg(c.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                t.col_status,
                Style::default()
                    .fg(c.accent)
                    .add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(c.border)),
        );

        frame.render_widget(table, inner_chunks[1]);
    }

    let hint = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(c.accent)),
        Span::raw(format!(" {} ", t.dialog_hint_add)),
        Span::styled("Esc", Style::default().fg(c.accent)),
        Span::raw(format!(" {} ", t.dialog_hint_cancel)),
        Span::styled("↑↓", Style::default().fg(c.accent)),
        Span::raw(format!(" {} ", t.hint_select)),
        Span::styled(
            format!(" {}/{}", app.add_dialog_selected() + 1, filtered.len()),
            Style::default().fg(c.inactive),
        ),
    ]);
    let hint_bar = Paragraph::new(hint)
        .style(Style::default().fg(c.inactive))
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
