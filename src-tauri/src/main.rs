use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::io;

mod app;
mod ui;

use app::App;
use ui::service::ServiceView;
use ui::settings::SettingsView;
use ui::tools::ToolsView;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
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
                    KeyCode::Char('1') => app.set_view(app::View::Service),
                    KeyCode::Char('2') => app.set_view(app::View::Settings),
                    KeyCode::Char('3') => app.set_view(app::View::Tools),
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
        Constraint::Length(1),
    ])
    .split(frame.area());
    
    // Title bar
    let title = Paragraph::new("Rust Win Tool - TUI Mode")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);
    
    // Main content
    match app.current_view() {
        app::View::Service => ServiceView::render(frame, chunks[1], app),
        app::View::Settings => SettingsView::render(frame, chunks[1], app),
        app::View::Tools => ToolsView::render(frame, chunks[1], app),
    }
    
    // Status bar
    let status = Line::from(vec![
        Span::raw("Press 1: Services | 2: Settings | 3: Tools | q: Quit"),
    ]);
    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::White));
    frame.render_widget(status_bar, chunks[2]);
}