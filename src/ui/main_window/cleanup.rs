//! 清理页面及其扫描、删除、回收站和确认流程。
//!
//! 白名单和删除语义由清理后端负责；本模块只调度操作并展示结果。

use super::shared::*;
use super::*;
use crate::features::cleanup as cleanup_backend;

impl MainWindow {
    pub(super) fn render_cleanup_page(&self, state: &AppState, cx: &Context<Self>) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let cleanup = state.cleanup();
        let busy = cleanup.scanning || cleanup.cleaning;

        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .opacity(if busy { 0.5 } else { 1.0 })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.cleanup_scan,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("cleanup-scan")
                        .debug_selector(|| "cleanup-scan".to_string())
                        .on_click(cx.listener(Self::cleanup_scan_clicked)),
                    )
                    .child(
                        components::button(
                            translations.cleanup_clean,
                            colors,
                            components::ButtonVariant::Primary,
                        )
                        .id("cleanup-clean")
                        .on_click(cx.listener(Self::cleanup_clean_clicked)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            translations.cleanup_mode_recycle,
                            cleanup.delete_mode == DeleteMode::Recycle,
                            colors,
                            "cleanup-mode-recycle",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_cleanup_delete_mode(DeleteMode::Recycle)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.cleanup_mode_permanent,
                            cleanup.delete_mode == DeleteMode::Permanent,
                            colors,
                            "cleanup-mode-permanent",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_cleanup_delete_mode(DeleteMode::Permanent)
                            })
                        })),
                    ),
            );

        let rows = cleanup.rows.iter().enumerate().map(|(index, row)| {
            let selected = row.selected;
            let checkbox = div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(16.))
                .h(px(16.))
                .rounded_sm()
                .border_1()
                .border_color(components::color(if selected {
                    colors.brand.fill
                } else {
                    colors.border.strong
                }))
                .bg(components::color(if selected {
                    colors.brand.fill
                } else {
                    colors.bg.canvas
                }))
                .text_xs()
                .text_color(components::color(colors.fg.on_accent))
                .child(if selected { "✓" } else { "" });
            let size = match row.scan {
                Some(scan) => format!(
                    "{}  ·  {}",
                    cleanup_backend::format_bytes(scan.bytes),
                    translations
                        .dialog_count
                        .replace("{}", &scan.files.to_string())
                ),
                None => translations.cleanup_not_scanned.to_string(),
            };
            let base = components::card(colors).flex().items_center().gap_3().p_3();
            let card = if busy {
                base.opacity(0.6).id(("cleanup-row", index))
            } else {
                base.cursor_pointer()
                    .id(("cleanup-row", index))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_state(cx, move |state| state.toggle_cleanup_row(index))
                    }))
            };
            card.child(checkbox)
                .child(
                    div()
                        .flex_1()
                        .child(cleanup_category_label(row.category, translations)),
                )
                .children(
                    row.category
                        .requires_admin()
                        .then(|| components::badge(translations.cleanup_admin, &colors.info)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(components::color(colors.fg.muted))
                        .child(size),
                )
        });

        let recycle_row = components::card(colors)
            .flex()
            .items_center()
            .gap_3()
            .p_3()
            .child(div().flex_1().child(translations.cleanup_recycle_bin))
            .child(
                div()
                    .text_sm()
                    .text_color(components::color(colors.fg.muted))
                    .child(match cleanup.recycle_bin {
                        Some((bytes, _items)) => cleanup_backend::format_bytes(bytes),
                        None => translations.cleanup_not_scanned.to_string(),
                    }),
            )
            .child(
                components::button(
                    translations.cleanup_empty_bin,
                    colors,
                    components::ButtonVariant::Secondary,
                )
                .id("cleanup-empty-bin")
                .on_click(cx.listener(Self::cleanup_empty_bin_clicked)),
            );

        let message = cleanup
            .message
            .clone()
            .map(|text| components::alert(text, &colors.info));
        let progress = busy.then(|| components::progress_bar(colors));

        div()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .children(progress)
            .children(message)
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
                            .gap_2()
                            .w_full()
                            .flex_1()
                            .min_w(px(0.))
                            .min_h(px(0.))
                            .id("cleanup-list")
                            .overflow_y_scroll()
                            .track_scroll(&self.cleanup_scroll)
                            .on_scroll_wheel(cx.listener(Self::on_scrolled))
                            .children(rows)
                            .child(recycle_row),
                    )
                    .child(self.render_scrollbar(
                        &self.cleanup_scroll,
                        colors,
                        cx,
                        "cleanup-scrollbar",
                        "cleanup-thumb",
                    )),
            )
    }

    pub(super) fn cleanup_scan_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cleanup = self.state.read(cx).cleanup();
        if cleanup.scanning || cleanup.cleaning {
            return;
        }
        self.update_state(cx, |state| state.begin_cleanup_scan());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let scans = cx
                .background_spawn(async {
                    CleanupCategory::all()
                        .iter()
                        .map(|&category| (category, cleanup_backend::scan(category)))
                        .collect::<Vec<_>>()
                })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup_backend::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.apply_cleanup_scan(scans, recycle_bin);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn cleanup_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cleanup = self.state.read(cx).cleanup();
        if cleanup.scanning || cleanup.cleaning {
            return;
        }
        self.pending_clean = true;
        cx.notify();
    }

    pub(super) fn confirm_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pending_clean {
            return;
        }
        self.pending_clean = false;
        let categories = self.state.read(cx).selected_cleanup_categories();
        let mode = self.state.read(cx).cleanup().delete_mode;
        self.update_state(cx, |state| state.begin_cleanup());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let rescan = categories.clone();
            let report = cx
                .background_spawn(async move { cleanup_backend::clean(&categories, mode) })
                .await;
            let scans = cx
                .background_spawn(async move {
                    rescan
                        .iter()
                        .map(|&category| (category, cleanup_backend::scan(category)))
                        .collect::<Vec<_>>()
                })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup_backend::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    let message = state
                        .t()
                        .cleanup_done
                        .replacen("{}", &cleanup_backend::format_bytes(report.freed_bytes), 1)
                        .replacen("{}", &report.skipped_files.to_string(), 1);
                    state.apply_cleanup_result(scans, recycle_bin, message);
                    cx.notify();
                })
                .ok();
            crate::support::logging::log(&format!(
                "cleanup freed {} bytes, removed {}, skipped {}",
                report.freed_bytes, report.removed_files, report.skipped_files
            ));
        })
        .detach();
    }

    pub(super) fn cancel_clean_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_clean = false;
        cx.notify();
    }

    pub(super) fn cleanup_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cleanup = self.state.read(cx).cleanup();
        if cleanup.scanning || cleanup.cleaning {
            return;
        }
        self.pending_empty_bin = true;
        cx.notify();
    }

    pub(super) fn confirm_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pending_empty_bin {
            return;
        }
        self.pending_empty_bin = false;
        self.update_state(cx, |state| state.begin_cleanup());
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let emptied = cx
                .background_spawn(async { cleanup_backend::empty_recycle_bin() })
                .await;
            let recycle_bin = cx
                .background_spawn(async { cleanup_backend::recycle_bin_usage() })
                .await;
            state
                .update(cx, |state, cx| {
                    state.apply_cleanup_scan(Vec::new(), recycle_bin);
                    if emptied {
                        let message = state.t().cleanup_bin_done.to_string();
                        state.set_cleanup_message(message);
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn cancel_empty_bin_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_empty_bin = false;
        cx.notify();
    }
}
