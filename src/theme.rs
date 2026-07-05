use ratatui::style::Color;

pub struct ThemeColors {
    pub primary: Color,
    pub border: Color,
    pub inactive: Color,
    pub accent: Color,
    pub bg_select: Color,
    pub running: Color,
    pub stopped: Color,
    pub error: Color,
}

pub const DARK: ThemeColors = ThemeColors {
    primary: Color::Cyan,
    border: Color::DarkGray,
    inactive: Color::DarkGray,
    accent: Color::Yellow,
    bg_select: Color::DarkGray,
    running: Color::Green,
    stopped: Color::Red,
    error: Color::Red,
};

pub const LIGHT: ThemeColors = ThemeColors {
    primary: Color::Blue,
    border: Color::Gray,
    inactive: Color::Gray,
    accent: Color::Blue,
    bg_select: Color::Rgb(230, 230, 230),
    running: Color::Green,
    stopped: Color::Red,
    error: Color::Red,
};
