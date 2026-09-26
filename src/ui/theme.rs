//! 与界面框架无关的设计令牌，以 24 位 RGB 值保存。GPUI 层会将其转换为
//! `Rgba`，从而使状态层不依赖具体的前端框架。
//!
//! 令牌按用途分组，而不是平铺排列，以便界面扩展时仍能保持色彩体系一致：
//!
//! - `bg`：背景层级，从窗口底色到弹出层。
//! - `fg`：文字层级，从主要内容到占位文本。
//! - `border`：细分隔线和边框。
//! - `brand`：统一的品牌强调色及其纯色填充状态。
//! - `status`：`success`、`warning`、`danger`、`info` 状态色，各自包含实心标记色、
//!   易读的文字色和柔和的底色。

/// 分层背景表面。相邻层级应有清晰但柔和的区别，让界面层次无需依赖粗重边框也能辨认。
pub struct BgColors {
    /// 主窗口背景及主要内容画布。
    pub canvas: u32,
    /// 左侧导航栏。
    pub sidebar: u32,
    /// 卡片和表格行。
    pub surface: u32,
    /// 表格行和中性控件的悬停状态。
    pub surface_hover: u32,
    /// 对话框及高于 `surface` 的浮动层。
    pub elevated: u32,
    /// 凹入区域，例如表格标题行。
    pub muted: u32,
}

/// 文字颜色，按强调程度从强到弱排列。
pub struct FgColors {
    pub default: u32,
    pub muted: u32,
    pub subtle: u32,
    /// 绘制在品牌色或状态色纯色背景上的文字。
    pub on_accent: u32,
}

/// 边框和分隔线颜色。
pub struct BorderColors {
    pub default: u32,
    pub subtle: u32,
    pub strong: u32,
}

/// 统一的强调色及按钮纯色填充状态。
pub struct BrandColors {
    /// 用于深色或浅色表面上的文字、图标和焦点环的强调色。
    pub primary: u32,
    /// 主要按钮的纯色填充，确保白色文字保持清晰易读。
    pub fill: u32,
    pub fill_hover: u32,
    /// 用于选中导航项和状态徽标的柔和底色。
    pub soft: u32,
}

/// 一个语义状态对应的标记点颜色、文字颜色和柔和底色。
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
