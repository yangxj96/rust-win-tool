use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct ToolsView;

impl ToolsView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        if app.tool_detail_active() {
            render_sysinfo(frame, area, app);
        } else {
            render_tools_list(frame, area, app);
        }
    }
}

fn render_tools_list(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.t();
    let c = app.theme_colors();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(area);

    let header = Paragraph::new(t.tools_header)
        .style(
            Style::default()
                .fg(c.primary)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(c.border)));
    frame.render_widget(header, chunks[0]);

    let tools = [
        ("1", t.tool_sysinfo, t.tool_sysinfo_desc),
        ("2", t.tool_diskclean, t.tool_diskclean_desc),
        ("3", t.tool_network, t.tool_network_desc),
        ("4", t.tool_process, t.tool_process_desc),
        ("5", t.tool_registry, t.tool_registry_desc),
        ("6", t.tool_eventlog, t.tool_eventlog_desc),
    ];

    let list_items: Vec<ListItem> = tools
        .iter()
        .enumerate()
        .map(|(i, (num, name, desc))| {
            let style = if i == app.tools_selected() {
                Style::default().fg(c.fg_select).bg(c.bg_select)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {}. ", num),
                    Style::default().fg(c.accent),
                ),
                Span::styled(
                    *name,
                    Style::default().fg(c.fg_default).add_modifier(Modifier::BOLD),
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
}

fn render_sysinfo(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.t();
    let c = app.theme_colors();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // 标题
    let header = Paragraph::new(t.sysinfo_title)
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

    // 系统信息内容
    if let Some(info) = app.system_info() {
        let items = [
            (t.sysinfo_os, info.os.as_str()),
            (t.sysinfo_version, info.version.as_str()),
            (t.sysinfo_build, info.build.as_str()),
            (t.sysinfo_computer, info.computer.as_str()),
            (t.sysinfo_user, info.user.as_str()),
            (t.sysinfo_cpu, info.cpu.as_str()),
            (t.sysinfo_cores, info.cores.as_str()),
            (t.sysinfo_ram, info.ram.as_str()),
        ];

        let lines: Vec<Line> = items
            .iter()
            .map(|(label, value)| {
                Line::from(vec![
                    Span::styled(
                        format!("  {:<12}", label),
                        Style::default()
                            .fg(c.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        value.to_string(),
                        Style::default().fg(c.fg_default),
                    ),
                ])
            })
            .collect();

        let content = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(c.border)),
        );
        frame.render_widget(content, chunks[1]);
    } else {
        let loading = Paragraph::new(t.sysinfo_fetching)
            .style(Style::default().fg(c.inactive))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(c.border)),
            );
        frame.render_widget(loading, chunks[1]);
    }

    // 底部提示
    let hint = Paragraph::new(Line::from(Span::styled(
        format!("  {}", t.sysinfo_back_hint),
        Style::default().fg(c.inactive),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(c.border)),
    );
    frame.render_widget(hint, chunks[2]);
}
