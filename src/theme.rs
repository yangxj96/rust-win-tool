/// Theme colors are stored as RGB values so the state layer stays independent
/// of the frontend framework. The GPUI layer converts them to `Rgba` values.
pub struct ThemeColors {
    pub primary: u32,
    pub success: u32,
    pub warning: u32,
    pub danger: u32,
    pub border: u32,
    pub inactive: u32,
    pub accent: u32,
    pub bg_window: u32,
    pub bg_sidebar: u32,
    pub bg_surface: u32,
    pub bg_select: u32,
    pub fg_default: u32,
    pub error: u32,
}

pub const DARK: ThemeColors = ThemeColors {
    primary: 0x66b1ff,
    success: 0x85ce61,
    warning: 0xebb563,
    danger: 0xf78989,
    border: 0x334155,
    inactive: 0x94a3b8,
    accent: 0xebb563,
    bg_window: 0x0f172a,
    bg_sidebar: 0x111827,
    bg_surface: 0x1f2937,
    bg_select: 0x1e3a5f,
    fg_default: 0xe5e7eb,
    error: 0xf78989,
};

pub const LIGHT: ThemeColors = ThemeColors {
    primary: 0x409eff,
    success: 0x67c23a,
    warning: 0xe6a23c,
    danger: 0xf56c6c,
    border: 0xe4e7ed,
    inactive: 0x909399,
    accent: 0xe6a23c,
    bg_window: 0xf5f7fa,
    bg_sidebar: 0xffffff,
    bg_surface: 0xffffff,
    bg_select: 0xecf5ff,
    fg_default: 0x303133,
    error: 0xf56c6c,
};

#[cfg(test)]
mod tests {
    use super::{DARK, LIGHT};

    #[test]
    fn element_plus_tokens_keep_status_roles_distinct() {
        assert_eq!(LIGHT.primary, 0x409eff);
        assert_eq!(LIGHT.success, 0x67c23a);
        assert_eq!(LIGHT.warning, 0xe6a23c);
        assert_eq!(LIGHT.danger, 0xf56c6c);
        assert_ne!(DARK.bg_sidebar, DARK.bg_window);
        assert_ne!(LIGHT.bg_sidebar, LIGHT.bg_window);
    }
}
