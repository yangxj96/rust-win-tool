//! Small render helpers and localized labels shared by window pages and dialogs.

use super::*;

#[cfg(target_os = "windows")]
pub(super) fn toggle_window_zoom(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsZoomed, ShowWindowAsync, SW_MAXIMIZE, SW_RESTORE,
    };

    let Ok(window_handle) = <Window as HasWindowHandle>::window_handle(window) else {
        window.zoom_window();
        return;
    };
    let RawWindowHandle::Win32(handle) = window_handle.as_raw() else {
        window.zoom_window();
        return;
    };
    let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    let command = unsafe {
        if IsZoomed(hwnd) != 0 {
            SW_RESTORE
        } else {
            SW_MAXIMIZE
        }
    };
    unsafe {
        ShowWindowAsync(hwnd, command);
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn toggle_window_zoom(window: &Window) {
    window.zoom_window();
}

/// A titled group of label/value rows for the system information page.
pub(super) fn info_group(title: &'static str, rows: &[(&str, &str)], colors: &ThemeColors) -> Div {
    components::card(colors)
        .flex()
        .flex_col()
        .gap_4()
        .flex_1()
        .p_4()
        .child(components::card_title(title, colors))
        .children(rows.iter().map(|(label, value)| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(components::color(colors.fg.subtle))
                        .child((*label).to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(components::color(colors.fg.default))
                        .child((*value).to_string()),
                )
        }))
}

/// A two-state pill used by the settings rows.
pub(super) fn segment(
    label: &str,
    active: bool,
    colors: &ThemeColors,
    id: &'static str,
) -> gpui::Stateful<Div> {
    let base = div()
        .flex()
        .items_center()
        .justify_center()
        .h(px(28.))
        .px_3()
        .rounded_md()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .id(id);
    let base = if active {
        base.bg(components::color(colors.brand.soft))
            .text_color(components::color(colors.brand.primary))
    } else {
        base.bg(components::color(colors.bg.canvas))
            .text_color(components::color(colors.fg.muted))
            .hover(move |style| style.bg(components::color(colors.bg.surface_hover)))
    };
    base.child(label.to_string())
}

pub(super) fn status_label(
    status: ServiceStatus,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match status {
        ServiceStatus::Running => translations.status_running,
        ServiceStatus::Stopped | ServiceStatus::Unknown => translations.status_stopped,
        ServiceStatus::Starting => translations.status_starting,
        ServiceStatus::Stopping => translations.status_stopping,
        ServiceStatus::Refreshing => translations.status_refreshing,
    }
}

pub(super) fn status_colors(status: ServiceStatus, colors: &ThemeColors) -> &StatusColors {
    match status {
        ServiceStatus::Running => &colors.success,
        ServiceStatus::Stopped => &colors.danger,
        ServiceStatus::Starting | ServiceStatus::Stopping | ServiceStatus::Refreshing => {
            &colors.warning
        }
        ServiceStatus::Unknown => &colors.info,
    }
}

pub(super) fn startup_source_label(
    location: StartupLocation,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match location {
        StartupLocation::RegistryHkcu => translations.startup_src_hkcu,
        StartupLocation::RegistryHklm => translations.startup_src_hklm,
        StartupLocation::FolderUser => translations.startup_src_user,
        StartupLocation::FolderCommon => translations.startup_src_common,
    }
}

pub(super) fn net_port_id(port: u16) -> &'static str {
    match port {
        80 => "net-port-80",
        443 => "net-port-443",
        3306 => "net-port-3306",
        5432 => "net-port-5432",
        6379 => "net-port-6379",
        _ => "net-port-other",
    }
}

pub(super) fn service_sort_label(
    sort: ServiceSort,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match sort {
        ServiceSort::Manual => translations.sort_manual,
        ServiceSort::Name => translations.sort_name,
        ServiceSort::Status => translations.sort_status,
    }
}

pub(super) fn history_action_label(
    action: HistoryAction,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match action {
        HistoryAction::Start => translations.action_start,
        HistoryAction::Stop => translations.action_stop,
        HistoryAction::StartAll => translations.action_start_all,
        HistoryAction::StopAll => translations.action_stop_all,
        HistoryAction::Refresh => translations.action_refresh,
        HistoryAction::StartType => translations.action_start_type,
    }
}

pub(super) fn detail_row(label: &str, value: String, colors: &ThemeColors) -> Div {
    div()
        .flex()
        .items_start()
        .gap_2()
        .w_full()
        .child(
            div()
                .w(px(96.))
                .flex_shrink_0()
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(components::color(colors.fg.default))
                .child(value),
        )
}

pub(super) fn join_list(items: &[String], empty: &str) -> String {
    if items.is_empty() {
        empty.to_string()
    } else {
        items.join(", ")
    }
}

pub(super) fn cleanup_category_label(
    category: CleanupCategory,
    translations: &crate::i18n::Translations,
) -> &'static str {
    match category {
        CleanupCategory::UserTemp => translations.cleanup_cat_user_temp,
        CleanupCategory::WindowsTemp => translations.cleanup_cat_windows_temp,
        CleanupCategory::ThumbnailCache => translations.cleanup_cat_thumbnails,
        CleanupCategory::WindowsUpdate => translations.cleanup_cat_windows_update,
    }
}

pub(super) fn script_text(translations: &crate::i18n::Translations, key: &str) -> &'static str {
    match key {
        "script_reset_navicat" => translations.script_reset_navicat,
        "script_reset_navicat_desc" => translations.script_reset_navicat_desc,
        "script_flush_dns" => translations.script_flush_dns,
        "script_flush_dns_desc" => translations.script_flush_dns_desc,
        _ => "",
    }
}

pub(super) fn tool_text(translations: &crate::i18n::Translations, key: &str) -> &'static str {
    match key {
        "tool_sysinfo" => translations.tool_sysinfo,
        "tool_sysinfo_desc" => translations.tool_sysinfo_desc,
        "tool_processes" => translations.tool_processes,
        "tool_processes_desc" => translations.tool_processes_desc,
        "tool_monitor" => translations.tool_monitor,
        "tool_monitor_desc" => translations.tool_monitor_desc,
        "tool_network" => translations.tool_network,
        "tool_network_desc" => translations.tool_network_desc,
        "tool_startup" => translations.tool_startup,
        "tool_startup_desc" => translations.tool_startup_desc,
        _ => "",
    }
}
