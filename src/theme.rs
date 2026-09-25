//! Framework-neutral design tokens stored as 24-bit RGB values. The GPUI
//! layer converts them to `Rgba` values, so the state layer stays independent
//! of the frontend framework.
//!
//! Tokens are grouped by role instead of being a flat list, which keeps the
//! palette coherent as the UI grows:
//!
//! - `bg`     — layered surfaces, from the window canvas up to popovers
//! - `fg`     — text weights from primary content to placeholders
//! - `border` — hairline separators and outlines
//! - `brand`  — the single accent color plus its solid fill states
//! - `status` — `success` / `warning` / `danger` / `info`, each with a
//!   solid dot color, a readable text color and a soft tint

/// Layered background surfaces. Each step should be perceptibly distinct so
/// elevation reads without relying on heavy borders.
pub struct BgColors {
    /// Root window background and the main content canvas.
    pub canvas: u32,
    /// Left navigation rail.
    pub sidebar: u32,
    /// Cards and table rows.
    pub surface: u32,
    /// Hover state for rows and neutral controls.
    pub surface_hover: u32,
    /// Dialogs and floating layers above `surface`.
    pub elevated: u32,
    /// Recessed areas such as table headers.
    pub muted: u32,
}

/// Text colors, ordered from strongest to weakest emphasis.
pub struct FgColors {
    pub default: u32,
    pub muted: u32,
    pub subtle: u32,
    /// Text drawn on a solid brand or status fill.
    pub on_accent: u32,
}

/// Border and divider colors.
pub struct BorderColors {
    pub default: u32,
    pub subtle: u32,
    pub strong: u32,
}

/// The single accent color and its solid button fill states.
pub struct BrandColors {
    /// Accent used for text, icons and focus rings on a dark/light surface.
    pub primary: u32,
    /// Solid fill for primary buttons; chosen so white text stays readable.
    pub fill: u32,
    pub fill_hover: u32,
    /// Soft tint used for selected navigation and badges.
    pub soft: u32,
}

/// One semantic status expressed as a dot color, a text color and a tint.
pub struct StatusColors {
    pub solid: u32,
    pub text: u32,
    pub soft: u32,
}

pub struct ThemeColors {
    pub bg: BgColors,
    pub fg: FgColors,
    pub border: BorderColors,
    pub brand: BrandColors,
    pub success: StatusColors,
    pub warning: StatusColors,
    pub danger: StatusColors,
    pub info: StatusColors,
}

pub const DARK: ThemeColors = ThemeColors {
    bg: BgColors {
        canvas: 0x0b0f17,
        sidebar: 0x0e1420,
        surface: 0x141b26,
        surface_hover: 0x1a2230,
        elevated: 0x1b2431,
        muted: 0x10161f,
    },
    fg: FgColors {
        default: 0xe6eaf2,
        muted: 0x9ba7b8,
        subtle: 0x69758a,
        on_accent: 0xffffff,
    },
    border: BorderColors {
        default: 0x232c3a,
        subtle: 0x1a222e,
        strong: 0x303a4a,
    },
    brand: BrandColors {
        primary: 0x5aa2ff,
        fill: 0x2f6fed,
        fill_hover: 0x255fd6,
        soft: 0x16294a,
    },
    success: StatusColors {
        solid: 0x3fb950,
        text: 0x56d364,
        soft: 0x12261a,
    },
    warning: StatusColors {
        solid: 0xd29922,
        text: 0xe3b341,
        soft: 0x2b2416,
    },
    danger: StatusColors {
        solid: 0xf85149,
        text: 0xff7b72,
        soft: 0x2c1719,
    },
    info: StatusColors {
        solid: 0x8b98a9,
        text: 0x9ba7b8,
        soft: 0x1a222e,
    },
};

pub const LIGHT: ThemeColors = ThemeColors {
    bg: BgColors {
        canvas: 0xf6f8fb,
        sidebar: 0xffffff,
        surface: 0xffffff,
        surface_hover: 0xf3f6fa,
        elevated: 0xffffff,
        muted: 0xf3f5f9,
    },
    fg: FgColors {
        default: 0x1b2230,
        muted: 0x5a6675,
        subtle: 0x8a95a3,
        on_accent: 0xffffff,
    },
    border: BorderColors {
        default: 0xe2e6ec,
        subtle: 0xedf0f4,
        strong: 0xcdd5df,
    },
    brand: BrandColors {
        primary: 0x1d4ed8,
        fill: 0x2563eb,
        fill_hover: 0x1d4ed8,
        soft: 0xe8f0fe,
    },
    success: StatusColors {
        solid: 0x1a7f37,
        text: 0x1a7f37,
        soft: 0xe7f5ec,
    },
    warning: StatusColors {
        solid: 0xd97706,
        text: 0xb7791f,
        soft: 0xfbf3e3,
    },
    danger: StatusColors {
        solid: 0xdc2626,
        text: 0xcf222e,
        soft: 0xfceaea,
    },
    info: StatusColors {
        solid: 0x8a95a3,
        text: 0x5a6675,
        soft: 0xf3f5f9,
    },
};

#[cfg(test)]
mod tests {
    use super::{DARK, LIGHT};

    #[test]
    fn surfaces_are_layered_and_distinct() {
        assert_ne!(DARK.bg.canvas, DARK.bg.sidebar);
        assert_ne!(DARK.bg.sidebar, DARK.bg.surface);
        assert_ne!(DARK.bg.surface, DARK.bg.elevated);
        assert_ne!(LIGHT.bg.canvas, LIGHT.bg.sidebar);
        assert_ne!(LIGHT.bg.canvas, LIGHT.bg.surface);
        assert_ne!(LIGHT.bg.surface_hover, LIGHT.bg.surface);
    }

    #[test]
    fn status_roles_keep_distinct_colors() {
        assert_ne!(DARK.success.solid, DARK.warning.solid);
        assert_ne!(DARK.warning.solid, DARK.danger.solid);
        assert_ne!(LIGHT.success.solid, LIGHT.danger.solid);
        assert_ne!(DARK.success.soft, DARK.success.solid);
        assert_ne!(LIGHT.danger.soft, LIGHT.danger.solid);
    }

    #[test]
    fn brand_keeps_separate_accent_and_fill() {
        assert_ne!(DARK.brand.primary, DARK.brand.fill);
        assert_ne!(LIGHT.brand.primary, LIGHT.brand.fill);
        assert_ne!(DARK.bg.muted, DARK.bg.surface);
    }
}
