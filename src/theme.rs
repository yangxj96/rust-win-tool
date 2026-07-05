use ratatui::style::Color;

pub struct ThemeColors {
    pub primary: Color,
    pub border: Color,
    pub inactive: Color,
    pub accent: Color,
    pub bg_terminal: Color,
    pub fg_default: Color,
    pub bg_select: Color,
    pub fg_select: Color,
    pub running: Color,
    pub stopped: Color,
    pub error: Color,
}

pub const DARK: ThemeColors = ThemeColors {
    primary: Color::Cyan,
    border: Color::DarkGray,
    inactive: Color::DarkGray,
    accent: Color::Yellow,
    bg_terminal: Color::Black,
    fg_default: Color::White,
    bg_select: Color::DarkGray,
    fg_select: Color::White,
    running: Color::Green,
    stopped: Color::Red,
    error: Color::Red,
};

pub const LIGHT: ThemeColors = ThemeColors {
    primary: Color::Rgb(0, 100, 180),
    border: Color::Rgb(180, 180, 180),
    inactive: Color::Rgb(120, 120, 120),
    accent: Color::Rgb(0, 130, 150),
    bg_terminal: Color::White,
    fg_default: Color::Black,
    bg_select: Color::Rgb(200, 220, 240),
    fg_select: Color::Black,
    running: Color::Rgb(0, 150, 0),
    stopped: Color::Rgb(200, 50, 50),
    error: Color::Rgb(200, 50, 50),
};
