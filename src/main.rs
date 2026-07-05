use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs, Paragraph};
use ratatui::Frame;
use std::io;

mod app;
mod i18n;
mod theme;
mod ui;

use app::{App, View};
use ui::service::ServiceView;
use ui::scripts::ScriptsView;
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

        if let Some(action) = app.take_pending_action() {
            app.execute_pending(action);
            terminal.draw(|frame| render(frame, app))?;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                let should_quit = if app.show_add_dialog() {
                    handle_add_dialog_key(app, key.code)
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

fn handle_add_dialog_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.close_add_dialog(),
        KeyCode::Up | KeyCode::Char('k') => app.add_dialog_select_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.add_dialog_select_next(),
        KeyCode::Enter => app.confirm_add_service(),
        KeyCode::Backspace => app.add_dialog_backspace(),
        KeyCode::Char(c) => app.add_dialog_input(c),
        _ => {}
    }
    false
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
        KeyCode::Char('S') => {
            if *app.current_view() == View::Service {
                app.start_all_services();
            }
        }
        KeyCode::Char('P') => {
            if *app.current_view() == View::Service {
                app.stop_all_services();
            }
        }
        KeyCode::Enter if *app.current_view() == View::Settings && app.settings_selected() == 0 => {
            app.cycle_language();
        }
        KeyCode::Enter if *app.current_view() == View::Settings && app.settings_selected() == 1 => {
            app.cycle_theme();
        }
        KeyCode::Enter if *app.current_view() == View::Scripts => {
            app.execute_script_for_selected();
        }
        _ => {}
    }
    false
}

fn render(frame: &mut Frame, app: &App) {
    let c = app.theme_colors();

    // 用主题背景色填满整个终端区域
    frame.render_widget(
        Block::default().style(Style::default().fg(c.fg_default).bg(c.bg_terminal)),
        frame.area(),
    );

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
        View::Scripts => ScriptsView::render(frame, chunks[1], app),
    }

    // 底部状态栏
    render_status_bar(frame, chunks[2], app);
}

fn render_title_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let t = app.t();
    let c = app.theme_colors();
    let titles = vec![
        t.tab_service,
        t.tab_tools,
        t.tab_scripts,
        t.tab_settings,
    ];

    let selected = app.current_view().index();
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(c.inactive))
        .highlight_style(
            Style::default()
                .fg(c.primary)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t.app_title)
                .title_style(
                    Style::default()
                        .fg(c.primary)
                        .add_modifier(Modifier::BOLD),
                )
                .border_style(Style::default().fg(c.border)),
        );
    frame.render_widget(tabs, area);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let t = app.t();
    let c = app.theme_colors();
    let y = Style::default().fg(c.accent);
    let mut s: Vec<Span> = vec![
        Span::styled(format!(" ←→ {} ", t.hint_switch), y),
        Span::styled(format!("↑↓ {} ", t.hint_select), y),
    ];

    match app.current_view() {
        View::Service => {
            s.push(Span::styled(format!("a {} ", t.hint_add), y));
            s.push(Span::styled(format!("d {} ", t.hint_delete), y));
            s.push(Span::styled(format!("s {} ", t.hint_start), y));
            s.push(Span::styled(format!("S {} ", t.hint_start_all), y));
            s.push(Span::styled(format!("p {} ", t.hint_stop), y));
            s.push(Span::styled(format!("P {} ", t.hint_stop_all), y));
            s.push(Span::styled(format!("r {} ", t.hint_refresh), y));
        }
        View::Settings | View::Tools | View::Scripts => {
            s.push(Span::styled(format!("Enter {} ", t.hint_confirm), y));
        }
    }
    s.push(Span::styled(format!("q {}", t.hint_quit), y));

    frame.render_widget(Paragraph::new(Line::from(s)), area);
}
