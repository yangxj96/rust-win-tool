use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct ToolsView;

impl ToolsView {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

        // 顶部信息栏
        let header = Paragraph::new("系统工具")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
        frame.render_widget(header, chunks[0]);

        // 工具列表
        let tools = [
            ("1", "系统信息", "查看系统版本、CPU、内存等信息"),
            ("2", "磁盘清理", "清理临时文件和系统垃圾"),
            ("3", "网络诊断", "检测网络连接和DNS配置"),
            ("4", "进程管理", "查看和管理系统进程"),
            ("5", "注册表编辑器", "编辑 Windows 注册表"),
            ("6", "事件查看器", "查看系统事件日志"),
        ];

        let list_items: Vec<ListItem> = tools
            .iter()
            .enumerate()
            .map(|(i, (num, name, desc))| {
                let style = if i == app.tools_selected() {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {}. ", num),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        *name,
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", desc),
                        Style::default().fg(Color::DarkGray),
                    ),
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
