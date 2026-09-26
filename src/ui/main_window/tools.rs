//! Tool catalog and views for monitoring, network, startup, processes, and system info.
//!
//! Blocking refresh and diagnostic work stays in GPUI background tasks.

use super::shared::*;
use super::*;
use crate::cleanup as cleanup_backend;

impl MainWindow {
    pub(super) fn render_tools_page(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
        if state.tool_detail_active() {
            return match state.tools_selected() {
                1 => self.render_process_list(state, cx),
                2 => self.render_monitor(state, cx),
                3 => self.render_network(window, state, cx),
                4 => self.render_startup(state, cx),
                _ => self.render_system_info(state, cx),
            };
        }
        let translations = state.t();
        let colors = state.theme_colors();
        let rows = TOOLS
            .iter()
            .enumerate()
            .map(|(index, (name, description))| {
                let label = tool_text(translations, name);
                let description = tool_text(translations, description);
                let selected = state.tools_selected() == index;
                components::card(colors)
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .cursor_pointer()
                    .border_color(components::color(if selected {
                        colors.brand.primary
                    } else {
                        colors.border.subtle
                    }))
                    .bg(components::color(if selected {
                        colors.brand.soft
                    } else {
                        colors.bg.surface
                    }))
                    .id(("tool-row", index))
                    .on_click(cx.listener(move |this, event, window, cx| {
                        this.select_tool(index, event, window, cx);
                        this.open_tool(cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(40.))
                            .h(px(40.))
                            .rounded_lg()
                            .flex_shrink_0()
                            .bg(components::color(if selected {
                                colors.bg.surface
                            } else {
                                colors.bg.muted
                            }))
                            .text_lg()
                            .text_color(components::color(colors.brand.primary))
                            .child(ICON_TOOLS),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(components::color(colors.fg.default))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(components::color(colors.fg.muted))
                                    .child(description),
                            ),
                    )
                    .child(
                        div()
                            .text_lg()
                            .text_color(components::color(colors.fg.subtle))
                            .child("›"),
                    )
            });
        div().flex().flex_col().size_full().p_6().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .children(rows)
                .id("tool-list")
                .overflow_y_scroll(),
        )
    }

    pub(super) fn render_monitor(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let monitor = state.monitor();
        let latest = monitor.latest.as_ref();

        let cpu_percent = latest.map(|metrics| metrics.cpu_percent).unwrap_or(0.0);
        let memory_percent = latest.map(|metrics| metrics.memory_percent).unwrap_or(0.0);
        let (memory_used, memory_total) = latest
            .map(|metrics| (metrics.memory_used, metrics.memory_total))
            .unwrap_or((0, 0));
        let (net_rx, net_tx) = latest
            .map(|metrics| (metrics.net_rx_bps, metrics.net_tx_bps))
            .unwrap_or((0.0, 0.0));
        let disks = latest
            .map(|metrics| metrics.disks.clone())
            .unwrap_or_default();

        let toolbar = div().flex().items_center().w_full().child(
            components::button(
                translations.sysinfo_back,
                colors,
                components::ButtonVariant::Secondary,
            )
            .id("monitor-back")
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.update_state(cx, |state| state.close_tool_detail());
            })),
        );

        let cpu_card = components::card(colors)
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.mon_cpu),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(components::color(colors.brand.primary))
                            .child(format!("{cpu_percent:.0}%")),
                    ),
            )
            .child(components::sparkline(
                &monitor.cpu_history,
                colors.brand.primary,
            ));

        let memory_card = components::card(colors)
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.mon_memory),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(components::color(colors.fg.subtle))
                                    .child(format!(
                                        "{} {} / {}",
                                        translations.mon_used,
                                        cleanup_backend::format_bytes(memory_used),
                                        cleanup_backend::format_bytes(memory_total)
                                    )),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(components::color(colors.success.text))
                                    .child(format!("{memory_percent:.0}%")),
                            ),
                    ),
            )
            .child(components::sparkline(
                &monitor.memory_history,
                colors.success.text,
            ));

        let network_card = components::card(colors)
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .p_4()
            .child(components::card_title(translations.mon_network, colors))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.mon_rx)
                            .child(
                                div()
                                    .text_color(components::color(colors.fg.default))
                                    .child(crate::monitor::format_rate(net_rx)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.mon_tx)
                            .child(
                                div()
                                    .text_color(components::color(colors.fg.default))
                                    .child(crate::monitor::format_rate(net_tx)),
                            ),
                    ),
            );

        let disk_card = components::card(colors)
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_4()
            .child(components::card_title(translations.mon_disk, colors))
            .children(disks.into_iter().map(|disk| {
                let used = disk.total.saturating_sub(disk.free);
                let fraction = if disk.total > 0 {
                    used as f32 / disk.total as f32
                } else {
                    0.0
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .text_sm()
                            .text_color(components::color(colors.fg.default))
                            .child(disk.name.clone())
                            .child(div().text_color(components::color(colors.fg.muted)).child(
                                format!(
                                    "{} {} / {}",
                                    translations.mon_free,
                                    cleanup_backend::format_bytes(disk.free),
                                    cleanup_backend::format_bytes(disk.total)
                                ),
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(6.))
                            .rounded_full()
                            .overflow_hidden()
                            .bg(components::color(colors.bg.muted))
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(components::color(colors.brand.primary))
                                    .w(relative(fraction.clamp(0.0, 1.0))),
                            ),
                    )
            }));

        let body: AnyElement = if latest.is_none() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .child(translations.mon_waiting)
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .id("monitor-body")
                .overflow_y_scroll()
                .child(cpu_card)
                .child(memory_card)
                .child(network_card)
                .child(disk_card)
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(body)
    }

    pub(super) fn render_network(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();

        let host_focused = self.net_host_focus.is_focused(window);
        let host_input = {
            let input_entity = cx.entity();
            let input_focus = self.net_host_focus.clone();
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    window.handle_input(
                        &input_focus,
                        ElementInputHandler::new(bounds, input_entity.clone()),
                        cx,
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let host_field = components::search_field(
            state.net_host(),
            state.net_host_marked(),
            translations.net_placeholder,
            colors,
            host_focused,
        )
        .relative()
        .track_focus(&self.net_host_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("net-host")
        .on_click(cx.listener(Self::focus_net_host))
        .child(host_input);

        let ports = [80u16, 443, 3306, 5432, 6379];
        let port_row = div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div()
                    .w(px(72.))
                    .flex_shrink_0()
                    .text_sm()
                    .text_color(components::color(colors.fg.muted))
                    .child(translations.net_port_label),
            )
            .children(ports.into_iter().map(|port| {
                let selected = state.net_port() == port;
                segment(&port.to_string(), selected, colors, net_port_id(port)).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.update_state(cx, move |state| state.set_net_port(port))
                    }),
                )
            }));

        let toolbar = div().flex().items_center().w_full().child(
            components::button(
                translations.sysinfo_back,
                colors,
                components::ButtonVariant::Secondary,
            )
            .id("net-back")
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.update_state(cx, |state| state.close_tool_detail());
            })),
        );

        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                components::button(
                    translations.net_ping,
                    colors,
                    components::ButtonVariant::Primary,
                )
                .id("net-ping")
                .on_click(cx.listener(Self::net_ping_clicked)),
            )
            .child(
                components::button(
                    translations.net_dns,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("net-dns")
                .on_click(cx.listener(Self::net_dns_clicked)),
            )
            .child(
                components::button(
                    translations.net_check_port,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("net-port")
                .on_click(cx.listener(Self::net_port_clicked)),
            );

        let result_card = components::card(colors)
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .flex_1()
            .p_4()
            .child(components::card_title(translations.tool_network, colors))
            .child(
                div()
                    .text_sm()
                    .text_color(components::color(if state.net_loading() {
                        colors.fg.muted
                    } else {
                        colors.fg.default
                    }))
                    .child(if state.net_loading() {
                        translations.net_running.to_string()
                    } else {
                        state.net_result().unwrap_or_default().to_string()
                    }),
            );

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .child(
                        div()
                            .w(px(72.))
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(components::color(colors.fg.muted))
                            .child(translations.net_host_label),
                    )
                    .child(div().flex_1().min_w(px(0.)).child(host_field)),
            )
            .child(port_row)
            .child(actions)
            .child(result_card)
    }

    pub(super) fn render_startup(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let startup = state.startup();

        let toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                components::button(
                    translations.sysinfo_back,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("startup-back")
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.update_state(cx, |state| state.close_tool_detail());
                })),
            )
            .child(
                components::button(
                    translations.hint_refresh,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("startup-refresh")
                .on_click(cx.listener(Self::refresh_startup_clicked)),
            );

        let body: Div = if startup.loading {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .child(translations.status_refreshing)
        } else if let Some(error) = startup.error.as_deref() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.danger.text))
                .child(error.to_string())
        } else if startup.items.is_empty() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .child(translations.startup_empty)
        } else {
            let header = div()
                .flex()
                .items_center()
                .w_full()
                .h(px(32.))
                .px_3()
                .bg(components::color(colors.bg.muted))
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(components::color(colors.fg.muted))
                .child(div().flex_1().child(translations.startup_name))
                .child(div().w(px(170.)).child(translations.startup_source))
                .child(div().w(px(80.)).child(translations.col_status))
                .child(div().w(px(84.)));

            let rows = startup.items.iter().enumerate().map(|(index, item)| {
                let location = item.location;
                let value_name = item.value_name.clone();
                let enabled = item.enabled;
                let state_color = if enabled {
                    colors.success.text
                } else {
                    colors.fg.muted
                };
                let toggle = if enabled {
                    components::button(
                        translations.startup_disable,
                        colors,
                        components::ButtonVariant::Danger,
                    )
                } else {
                    components::button(
                        translations.startup_enable,
                        colors,
                        components::ButtonVariant::Secondary,
                    )
                };
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(44.))
                    .px_3()
                    .border_b_1()
                    .border_color(components::color(colors.border.subtle))
                    .text_sm()
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(item.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(components::color(colors.fg.subtle))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(item.command.clone()),
                            ),
                    )
                    .child(
                        div()
                            .w(px(170.))
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(components::color(colors.fg.muted))
                            .child(startup_source_label(location, translations)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .flex_shrink_0()
                            .text_color(components::color(state_color))
                            .child(if enabled {
                                translations.startup_enabled
                            } else {
                                translations.startup_disabled
                            }),
                    )
                    .child(
                        div().w(px(84.)).flex_shrink_0().child(
                            toggle
                                .h(px(26.))
                                .px_2()
                                .text_xs()
                                .id(("startup-toggle", index))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_startup(location, value_name.clone(), !enabled, cx)
                                })),
                        ),
                    )
            });

            components::card(colors)
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .overflow_hidden()
                .child(header)
                .child(
                    div()
                        .flex()
                        .w_full()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w_full()
                                .flex_1()
                                .min_w(px(0.))
                                .min_h(px(0.))
                                .id("startup-list")
                                .overflow_y_scroll()
                                .track_scroll(&self.startup_scroll)
                                .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                .children(rows),
                        )
                        .child(self.render_scrollbar(
                            &self.startup_scroll,
                            colors,
                            cx,
                            "startup-scrollbar",
                            "startup-thumb",
                        )),
                )
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(body)
    }

    pub(super) fn render_process_list(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let sort = state.process_sort();

        let sort_control = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                segment(
                    translations.proc_sort_memory,
                    sort == ProcessSort::Memory,
                    colors,
                    "proc-sort-memory",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_state(cx, |state| state.set_process_sort(ProcessSort::Memory))
                })),
            )
            .child(
                segment(
                    translations.proc_sort_name,
                    sort == ProcessSort::Name,
                    colors,
                    "proc-sort-name",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_state(cx, |state| state.set_process_sort(ProcessSort::Name))
                })),
            )
            .child(
                segment(
                    translations.proc_sort_pid,
                    sort == ProcessSort::Pid,
                    colors,
                    "proc-sort-pid",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_state(cx, |state| state.set_process_sort(ProcessSort::Pid))
                })),
            );

        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.sysinfo_back,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("process-back")
                        .on_click(cx.listener(
                            |this, _event, _window, cx| {
                                this.update_state(cx, |state| state.close_tool_detail());
                            },
                        )),
                    )
                    .child(
                        components::button(
                            translations.hint_refresh,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("process-refresh")
                        .on_click(cx.listener(Self::refresh_processes_clicked)),
                    )
                    .child(
                        components::button(
                            translations.proc_port_lookup,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("process-port-lookup")
                        .on_click(cx.listener(Self::open_port_lookup_clicked)),
                    ),
            )
            .child(sort_control);

        let body: Div = if state.processes_loading() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .child(translations.status_refreshing)
        } else if let Some(error) = state.processes_error() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.danger.text))
                .child(error.to_string())
        } else if state.processes().is_empty() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .child(translations.proc_empty)
        } else {
            let header = div()
                .flex()
                .items_center()
                .w_full()
                .h(px(32.))
                .px_3()
                .bg(components::color(colors.bg.muted))
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(components::color(colors.fg.muted))
                .child(div().w(px(90.)).child(translations.proc_pid))
                .child(div().flex_1().child(translations.proc_name))
                .child(div().w(px(110.)).child(translations.proc_memory))
                .child(div().w(px(80.)).child(translations.proc_cpu))
                .child(div().w(px(72.)));

            let rows = state.sorted_processes().into_iter().map(|process| {
                let pid = process.pid;
                let name = process.name.clone();
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(40.))
                    .px_3()
                    .border_b_1()
                    .border_color(components::color(colors.border.subtle))
                    .text_sm()
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .w(px(90.))
                            .text_color(components::color(colors.fg.muted))
                            .child(pid.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .w(px(110.))
                            .child(crate::process::format_memory(process.memory_bytes)),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .child(format!("{:.1}%", process.cpu_percent)),
                    )
                    .child(
                        div().w(px(72.)).child(
                            components::button(
                                translations.proc_end,
                                colors,
                                components::ButtonVariant::Danger,
                            )
                            .h(px(26.))
                            .px_2()
                            .text_xs()
                            .id(("proc-end", pid))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.pending_kill = Some((pid, name.clone()));
                                    cx.notify();
                                },
                            )),
                        ),
                    )
            });

            components::card(colors)
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .overflow_hidden()
                .child(header)
                .child(
                    div()
                        .flex()
                        .w_full()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w_full()
                                .flex_1()
                                .min_w(px(0.))
                                .min_h(px(0.))
                                .id("process-list")
                                .overflow_y_scroll()
                                .track_scroll(&self.process_scroll)
                                .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                .children(rows),
                        )
                        .child(self.render_scrollbar(
                            &self.process_scroll,
                            colors,
                            cx,
                            "process-scrollbar",
                            "process-thumb",
                        )),
                )
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(body)
    }

    pub(super) fn render_system_info(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let content = if state.system_info_loading() {
            div()
                .flex_1()
                .min_h(px(0.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(components::color(colors.fg.muted))
                .id("system-info-loading")
                .child(translations.sysinfo_fetching)
        } else if let Some(info) = state.system_info() {
            let system_rows = [
                (translations.sysinfo_os, info.os.as_str()),
                (translations.sysinfo_version, info.version.as_str()),
                (translations.sysinfo_build, info.build.as_str()),
                (translations.sysinfo_computer, info.computer.as_str()),
                (translations.sysinfo_user, info.user.as_str()),
            ];
            let hardware_rows = [
                (translations.sysinfo_cpu, info.cpu.as_str()),
                (translations.sysinfo_cores, info.cores.as_str()),
                (translations.sysinfo_ram, info.ram.as_str()),
            ];
            div()
                .flex()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .overflow_hidden()
                .id("system-info-scroll")
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_4()
                        .flex_1()
                        .min_w(px(0.))
                        .min_h(px(0.))
                        .id("system-info-groups")
                        .overflow_y_scroll()
                        .track_scroll(&self.sysinfo_scroll)
                        .on_scroll_wheel(cx.listener(Self::on_scrolled))
                        .child(info_group(
                            translations.sysinfo_group_system,
                            &system_rows,
                            colors,
                        ))
                        .child(info_group(
                            translations.sysinfo_group_hardware,
                            &hardware_rows,
                            colors,
                        )),
                )
                .child(self.render_scrollbar(
                    &self.sysinfo_scroll,
                    colors,
                    cx,
                    "sysinfo-scrollbar",
                    "sysinfo-thumb",
                ))
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(components::color(colors.danger.text))
                .id("system-info-error")
                .child(translations.err_failed)
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        components::button(
                            translations.hint_refresh,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("sysinfo-refresh")
                        .on_click(cx.listener(Self::refresh_system_info_clicked)),
                    )
                    .child(
                        components::button(
                            translations.sysinfo_back,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("sysinfo-back")
                        .on_click(cx.listener(
                            |this, _event, _window, cx| {
                                this.update_state(cx, |state| state.close_tool_detail());
                            },
                        )),
                    ),
            )
            .child(content)
    }

    pub(super) fn refresh_system_info_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.request_system_info_refresh());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async { backend::fetch_system_info() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_system_info(result);
                    cx.notify();
                })
                .ok();
            Timer::after(Duration::from_millis(100)).await;
            state.update(cx, |_state, cx| cx.notify()).ok();
        })
        .detach();
    }

    pub(super) fn refresh_processes_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_processes(cx);
    }

    pub(super) fn open_port_lookup_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.open_port_lookup());
        window.focus(&self.port_lookup_focus);
    }

    pub(super) fn focus_port_lookup(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.port_lookup_focus);
        cx.notify();
    }

    pub(super) fn close_port_lookup_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_port_lookup());
    }

    pub(super) fn search_port_lookup_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_port_lookup(cx);
    }

    pub(super) fn run_port_lookup(&mut self, cx: &mut Context<Self>) {
        let Some(port) = self
            .state
            .update(cx, |state, _cx| state.begin_port_lookup())
        else {
            return;
        };
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::lookup_port(port) })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_port_lookup(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn refresh_startup_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.begin_startup_refresh());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx.background_spawn(async { backend::list_startup() }).await;
            state
                .update(cx, |state, cx| {
                    state.set_startup(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn toggle_startup(
        &mut self,
        location: StartupLocation,
        value_name: String,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    backend::set_startup_enabled(location, &value_name, enabled)
                })
                .await;
            let list = cx.background_spawn(async { backend::list_startup() }).await;
            state
                .update(cx, |state, cx| {
                    state.set_startup(list);
                    if let Err(error) = result {
                        state.set_refresh_notice(error.to_string());
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn confirm_kill_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((pid, _name)) = self.pending_kill.take() else {
            return;
        };
        // If the port lookup dialog is showing results, refresh them once the
        // process is gone so the freed port disappears from the list.
        let port = self.state.read(cx).port_lookup_port();
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::terminate_process(pid) })
                .await;
            let list = cx
                .background_spawn(async { backend::list_processes() })
                .await;
            let port_result = match port {
                Some(port) => Some(
                    cx.background_spawn(async move { backend::lookup_port(port) })
                        .await,
                ),
                None => None,
            };
            state
                .update(cx, |state, cx| {
                    state.set_processes(list);
                    if let Some(result) = port_result {
                        state.set_port_lookup(result);
                    }
                    if let Err(error) = result {
                        state.set_refresh_notice(error.to_string());
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn cancel_kill_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_kill = None;
        cx.notify();
    }

    pub(super) fn focus_net_host(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.net_host_focus);
        cx.notify();
    }

    pub(super) fn net_ping_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_net_check(NetCheck::Ping, cx);
    }

    pub(super) fn net_dns_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_net_check(NetCheck::Dns, cx);
    }

    pub(super) fn net_port_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_net_check(NetCheck::Port, cx);
    }

    pub(super) fn start_net_check(&mut self, kind: NetCheck, cx: &mut Context<Self>) {
        let (host, port, translations) = {
            let state = self.state.read(cx);
            (
                state.net_host().trim().to_string(),
                state.net_port(),
                state.t(),
            )
        };
        if host.is_empty() {
            let message = translations.net_host_required.to_string();
            self.update_state(cx, move |state| state.set_net_result(message));
            return;
        }
        self.update_state(cx, |state| state.begin_net_check());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move {
                    match kind {
                        NetCheck::Ping => crate::network::ping(&host, 4, 1000).map(|summary| {
                            format!(
                                "{}/{}  {} ms",
                                summary.received, summary.sent, summary.avg_ms
                            )
                        }),
                        NetCheck::Dns => crate::network::resolve(&host)
                            .map(|ips| crate::network::format_addresses(&ips)),
                        NetCheck::Port => {
                            crate::network::check_port(&host, port, Duration::from_secs(3)).map(
                                |open| {
                                    if open {
                                        translations.net_open.to_string()
                                    } else {
                                        translations.net_closed.to_string()
                                    }
                                },
                            )
                        }
                    }
                })
                .await;
            let message = match result {
                Ok(message) => message,
                Err(error) => error,
            };
            state
                .update(cx, |state, cx| {
                    state.set_net_result(message);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// Modal that looks up which process is using a given local port and lets
    /// the user terminate it.
    pub(super) fn render_port_lookup_dialog(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let focused = self.port_lookup_focus.is_focused(window);

        let input = {
            let input_entity = cx.entity();
            let input_focus = self.port_lookup_focus.clone();
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    window.handle_input(
                        &input_focus,
                        ElementInputHandler::new(bounds, input_entity.clone()),
                        cx,
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let field = components::search_field(
            state.port_lookup_query(),
            state.port_lookup_query_marked(),
            translations.proc_port_placeholder,
            colors,
            focused,
        )
        .relative()
        .track_focus(&self.port_lookup_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("port-lookup-input")
        .on_click(cx.listener(Self::focus_port_lookup))
        .child(input);

        let header = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(30.))
            .px_3()
            .bg(components::color(colors.bg.muted))
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(components::color(colors.fg.muted))
            .child(div().w(px(56.)).child(translations.proc_col_protocol))
            .child(div().w(px(160.)).child(translations.proc_col_local))
            .child(div().w(px(160.)).child(translations.proc_col_remote))
            .child(div().w(px(92.)).child(translations.proc_col_state))
            .child(div().w(px(72.)).child(translations.proc_pid))
            .child(div().flex_1().child(translations.proc_name))
            .child(div().w(px(64.)));

        let rows = state
            .port_lookup_results()
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let pid = entry.pid;
                let name = entry.process_name.clone();
                let confirm_name = if name.is_empty() {
                    format!("PID {pid}")
                } else {
                    name.clone()
                };
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .min_h(px(38.))
                    .px_3()
                    .border_b_1()
                    .border_color(components::color(colors.border.subtle))
                    .text_sm()
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .w(px(56.))
                            .text_color(components::color(colors.fg.muted))
                            .child(entry.protocol.label()),
                    )
                    .child(
                        div()
                            .w(px(160.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(format!("{}:{}", entry.local_addr, entry.local_port)),
                    )
                    .child(
                        div()
                            .w(px(160.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(components::color(colors.fg.muted))
                            .child(if entry.remote_addr.is_empty() {
                                "—".to_string()
                            } else {
                                entry.remote_addr.clone()
                            }),
                    )
                    .child(
                        div()
                            .w(px(92.))
                            .text_color(components::color(colors.fg.muted))
                            .child(if entry.state.is_empty() {
                                "—".to_string()
                            } else {
                                entry.state.clone()
                            }),
                    )
                    .child(
                        div()
                            .w(px(72.))
                            .text_color(components::color(colors.fg.muted))
                            .child(pid.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(if name.is_empty() {
                                "—".to_string()
                            } else {
                                name.clone()
                            }),
                    )
                    .child(
                        div().w(px(64.)).child(
                            components::button(
                                translations.proc_end,
                                colors,
                                components::ButtonVariant::Danger,
                            )
                            .h(px(24.))
                            .px_2()
                            .text_xs()
                            .id(("port-end", index))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.pending_kill = Some((pid, confirm_name.clone()));
                                    cx.notify();
                                },
                            )),
                        ),
                    )
            });

        let results: Div = if let Some(error) = state.port_lookup_error() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(components::color(colors.danger.text))
                .child(error.to_string())
        } else if state.port_lookup_loading() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(translations.status_refreshing)
        } else if !state.port_lookup_searched() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(components::color(colors.fg.subtle))
                .child(translations.proc_port_placeholder)
        } else if state.port_lookup_results().is_empty() {
            div()
                .flex()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(components::color(colors.fg.muted))
                .child(translations.proc_port_none)
        } else {
            div()
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .child(header)
                .child(
                    div()
                        .flex()
                        .w_full()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w_full()
                                .flex_1()
                                .min_w(px(0.))
                                .min_h(px(0.))
                                .id("port-lookup-list")
                                .overflow_y_scroll()
                                .track_scroll(&self.port_scroll)
                                .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                .children(rows),
                        )
                        .child(self.render_scrollbar(
                            &self.port_scroll,
                            colors,
                            cx,
                            "port-scrollbar",
                            "port-thumb",
                        )),
                )
        };

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .occlude()
            .bg(rgba(0x00000099))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(820.))
                    .h(px(520.))
                    .p_5()
                    .rounded_xl()
                    .shadow_2xl()
                    .bg(components::color(colors.bg.elevated))
                    .border_1()
                    .border_color(components::color(colors.border.default))
                    .text_color(components::color(colors.fg.default))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(translations.proc_port_title),
                            )
                            .child(
                                components::button("✕", colors, components::ButtonVariant::Ghost)
                                    .w(px(28.))
                                    .h(px(28.))
                                    .px_0()
                                    .id("port-lookup-close-icon")
                                    .on_click(cx.listener(Self::close_port_lookup_clicked)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w_full()
                            .child(div().flex_1().min_w(px(0.)).child(field))
                            .child(
                                components::button(
                                    translations.proc_port_search,
                                    colors,
                                    components::ButtonVariant::Primary,
                                )
                                .id("port-lookup-search")
                                .on_click(cx.listener(Self::search_port_lookup_clicked)),
                            ),
                    )
                    .child(results)
                    .child(
                        div().flex().justify_end().child(
                            components::button(
                                translations.dialog_close,
                                colors,
                                components::ButtonVariant::Secondary,
                            )
                            .id("port-lookup-close")
                            .on_click(cx.listener(Self::close_port_lookup_clicked)),
                        ),
                    ),
            )
    }
}
