use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;
use std::io;

mod app;
mod ui;

use app::{App, View};
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
                let should_quit = if app.show_add_dialog() {
                    handle_add_dialog_key(app, key.code);
                    false
                } else {
                    handle_main_key(app, key.code)
                };
                if should_quit {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_add_dialog_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.close_add_dialog(),
        KeyCode::Up | KeyCode::Char('k') => app.add_dialog_select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.add_dialog_select_next(),
        KeyCode::Enter => app.confirm_add_service(),
        KeyCode::Char(c) => app.add_dialog_input(c),
        KeyCode::Backspace => app.add_dialog_backspace(),
        _ => {}
    }
}

fn handle_main_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => {
            return true; // 退出
        }
        KeyCode::Left | KeyCode::Char('h') => app.prev_view(),
        KeyCode::Right | KeyCode::Char('l') => app.next_view(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Char('a') => {
            if *app.current_view() == View::Service {
                app.open_add_dialog();
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
        KeyCode::Enter => {}
        _ => {}
    }
    false
}

fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    // 标题栏 - Tab式显示
    render_title_bar(frame, chunks[0], app);

    // 主内容区
    match app.current_view() {
        View::Service => ServiceView::render(frame, chunks[1], app),
        View::Settings => SettingsView::render(frame, chunks[1], app),
        View::Tools => ToolsView::render(frame, chunks[1], app),
    }

    // 底部状态栏
    render_status_bar(frame, chunks[2], app);
}

fn render_title_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let titles = vec![
        " 服务管理 ",
        " 系统工具 ",
        " 设置 ",
    ];

    let selected = app.current_view().index();
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Rust 系统工具 ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(tabs, area);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let help = match app.current_view() {
        View::Service => Line::from(vec![
            Span::styled(" ←→", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
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
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
        View::Settings => Line::from(vec![
            Span::styled(" ←→", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" 选择 "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" 确认 "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
        View::Tools => Line::from(vec![
            Span::styled(" ←→", Style::default().fg(Color::Yellow)),
            Span::raw(" 切换 "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" 选择 "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" 执行 "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" 退出"),
        ]),
    };
    let help_bar = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help_bar, area);
}
