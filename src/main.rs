use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::io;

mod app;
mod ui;

use app::{App, MsgType, View};
use ui::service::ServiceView;
use ui::settings::SettingsView;
use ui::tools::ToolsView;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    app.refresh_statuses();
    let result = run_app(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('1') => app.set_view(View::Service),
                    KeyCode::Char('2') => app.set_view(View::Settings),
                    KeyCode::Char('3') => app.set_view(View::Tools),
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Char('a') => {
                        if *app.current_view() == View::Service {
                            app.load_services().ok();
                            app.set_show_add_dialog(true);
                            app.set_status_message(
                                "添加服务功能开发中...".to_string(),
                                MsgType::Info,
                            );
                        }
                    }
                    KeyCode::Char('d') => {
                        if *app.current_view() == View::Service {
                            app.remove_selected_service();
                        }
                    }
                    KeyCode::Char('s') => {
                        if *app.current_view() == View::Service {
                            app.start_selected_service();
                        }
                    }
                    KeyCode::Char('p') => {
                        if *app.current_view() == View::Service {
                            app.stop_selected_service();
                        }
                    }
                    KeyCode::Char('r') => {
                        if *app.current_view() == View::Service {
                            app.refresh_statuses();
                        }
                    }
                    KeyCode::Enter => {
                        match app.current_view() {
                            View::Tools => {
                                let tools = [
                                    "系统信息", "磁盘清理", "网络诊断",
                                    "进程管理", "注册表编辑器", "事件查看器",
                                ];
                                let idx = app.tools_selected();
                                app.set_status_message(
                                    format!("工具 \"{}\" 功能开发中...", tools[idx]),
                                    MsgType::Info,
                                );
                            }
                            View::Settings => {
                                let settings = ["主题切换", "语言选择", "通知设置"];
                                let idx = app.settings_selected();
                                app.set_status_message(
                                    format!("设置 \"{}\" 功能开发中...", settings[idx]),
                                    MsgType::Info,
                                );
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // 标题栏
    let title = Paragraph::new("  Rust 系统工具")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(title, chunks[0]);

    // 主内容区
    match app.current_view() {
        View::Service => ServiceView::render(frame, chunks[1], app),
        View::Settings => SettingsView::render(frame, chunks[1], app),
        View::Tools => ToolsView::render(frame, chunks[1], app),
    }

    // 底部状态栏
    render_status_bar(frame, chunks[2], app);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let inner_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // 操作指引
    let help = match app.current_view() {
        View::Service => Line::from(vec![
            Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" 选择 "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" 添加 "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" 删除 "),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(" 启动 "),
            Span::styled("p", Style::default().fg(Color::Yellow)),
            Span::raw(" 停止 "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" 刷新 "),
            Span::styled("123", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
        View::Settings => Line::from(vec![
            Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" 选择 "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" 确认 "),
            Span::styled("123", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
        View::Tools => Line::from(vec![
            Span::styled(" ↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" 选择 "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" 执行 "),
            Span::styled("123", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
    };
    let help_bar = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
    frame.render_widget(help_bar, inner_chunks[0]);

    // 状态消息
    let msg = app.status_message();
    let (msg_style, prefix) = if msg.is_empty() {
        (Style::default().fg(Color::DarkGray), "")
    } else {
        match app.status_message_type() {
            MsgType::Success => (Style::default().fg(Color::Green), "✓ "),
            MsgType::Error => (Style::default().fg(Color::Red), "✗ "),
            MsgType::Info => (Style::default().fg(Color::Yellow), "ℹ "),
        }
    };
    let msg_line = Line::from(vec![
        Span::styled(format!(" {}{}", prefix, msg), msg_style),
    ]);
    let msg_bar = Paragraph::new(msg_line)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
    frame.render_widget(msg_bar, inner_chunks[1]);
}
