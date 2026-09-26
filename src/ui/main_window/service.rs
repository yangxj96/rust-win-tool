//! Service page rendering and UI actions for service lifecycle and metadata.
//!
//! Backend calls remain asynchronous; confirmation actions preserve the
//! existing managed-service replacement and deletion flows.

use super::shared::*;
use super::*;

impl MainWindow {
    pub(super) fn render_service_page(
        &self,
        window: &Window,
        state: &AppState,
        cx: &Context<Self>,
    ) -> Div {
        let translations = state.t();
        let colors = state.theme_colors();
        let refreshing = state.operation_state() == OperationState::Refreshing;

        let refresh_button: AnyElement = if refreshing {
            components::button(
                translations.status_refreshing,
                colors,
                components::ButtonVariant::Ghost,
            )
            .id("service-refresh")
            .on_click(cx.listener(Self::refresh_clicked))
            .with_animation(
                "service-refresh-pulse",
                Animation::new(Duration::from_millis(760)).repeat(),
                |element, delta| element.opacity(0.45 + 0.55 * delta),
            )
            .into_any_element()
        } else {
            components::button(
                translations.hint_refresh,
                colors,
                components::ButtonVariant::Ghost,
            )
            .id("service-refresh")
            .on_click(cx.listener(Self::refresh_clicked))
            .into_any_element()
        };

        let progress = refreshing.then(|| {
            components::progress_bar(colors)
                .absolute()
                .bottom_0()
                .left_0()
                .w_full()
        });
        let toolbar = div()
            .relative()
            .pb_2()
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
                            translations.hint_add,
                            colors,
                            components::ButtonVariant::Primary,
                        )
                        .id("service-add")
                        .debug_selector(|| "service-add".to_string())
                        .on_click(cx.listener(Self::add_clicked)),
                    )
                    .child(refresh_button)
                    .child(
                        components::button(
                            translations.hint_history,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .id("service-history")
                        .on_click(cx.listener(Self::toggle_history_clicked)),
                    )
                    .child(
                        components::button(
                            translations.hint_export,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .id("service-export")
                        .on_click(cx.listener(Self::export_services_clicked)),
                    )
                    .child(
                        components::button(
                            translations.hint_import,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .id("service-import")
                        .on_click(cx.listener(Self::import_services_clicked)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        components::button(
                            translations.hint_start_all,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("service-start-all")
                        .on_click(cx.listener(Self::start_all_clicked)),
                    )
                    .child(
                        components::button(
                            translations.hint_stop_all,
                            colors,
                            components::ButtonVariant::Secondary,
                        )
                        .id("service-stop-all")
                        .on_click(cx.listener(Self::stop_all_clicked)),
                    ),
            )
            .children(progress);

        let search_focused = self.service_search_focus.is_focused(window);
        let search_input = {
            let input_entity = cx.entity();
            let input_focus = self.service_search_focus.clone();
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
        let search_field = components::search_field(
            state.service_search(),
            state.service_search_marked(),
            translations.dialog_search,
            colors,
            search_focused,
        )
        .relative()
        .track_focus(&self.service_search_focus)
        .cursor_text()
        .focus(|style| style.border_color(components::color(colors.brand.primary)))
        .id("service-search")
        .on_click(cx.listener(Self::focus_service_search))
        .child(search_input);

        let filter_row = div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .child(div().flex_1().min_w(px(0.)).child(search_field))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        segment(
                            translations.filter_all,
                            state.service_filter() == ServiceFilter::All,
                            colors,
                            "filter-all",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::All)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.status_running,
                            state.service_filter() == ServiceFilter::Running,
                            colors,
                            "filter-running",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Running)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.status_stopped,
                            state.service_filter() == ServiceFilter::Stopped,
                            colors,
                            "filter-stopped",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Stopped)
                            })
                        })),
                    )
                    .child(
                        segment(
                            translations.svc_pending,
                            state.service_filter() == ServiceFilter::Pending,
                            colors,
                            "filter-pending",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| {
                                state.set_service_filter(ServiceFilter::Pending)
                            })
                        })),
                    )
                    .child(
                        components::button(
                            service_sort_label(state.service_sort(), translations),
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .id("service-sort")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_state(cx, |state| state.cycle_service_sort())
                        })),
                    ),
            );

        let header = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(36.))
            .px_3()
            .bg(components::color(colors.bg.muted))
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(components::color(colors.fg.muted))
            .child(
                div()
                    .w(px(SERVICE_NAME_COLUMN_WIDTH))
                    .child(translations.col_name),
            )
            .child(
                div()
                    .w(px(SERVICE_DISPLAY_COLUMN_WIDTH))
                    .child(translations.col_display),
            )
            .child(
                div()
                    .w(px(SERVICE_STATUS_COLUMN_WIDTH))
                    .child(translations.col_status),
            )
            .child(div().flex_1().child(translations.col_message))
            .child(
                div()
                    .w(px(SERVICE_ACTIONS_COLUMN_WIDTH))
                    .child(translations.col_actions),
            );

        let visible = state.filtered_service_indices();
        let rows = visible.iter().map(|&index| {
            let service = &state.managed_services()[index];
            let status = state.service_status(&service.name);
            let message = state.service_message(&service.name).unwrap_or("");
            let service_name = service.name.clone();
            let start_name = service_name.clone();
            let stop_name = service_name.clone();
            let detail_name = service_name.clone();
            let delete_name = service_name;
            let actions =
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .w(px(SERVICE_ACTIONS_COLUMN_WIDTH))
                    .child(
                        components::button(
                            translations.hint_start,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-start", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.start_service(start_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_stop,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-stop", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.stop_service(stop_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_detail,
                            colors,
                            components::ButtonVariant::Ghost,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-detail", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_service_detail(detail_name.clone(), cx)
                        })),
                    )
                    .child(
                        components::button(
                            translations.hint_delete,
                            colors,
                            components::ButtonVariant::Danger,
                        )
                        .h(px(28.))
                        .px_2()
                        .text_xs()
                        .id(("service-delete", index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.pending_delete = Some(delete_name.clone());
                            cx.notify();
                        })),
                    );
            div()
                .flex()
                .items_center()
                .w_full()
                .min_h(px(52.))
                .px_3()
                .border_b_1()
                .border_color(components::color(colors.border.subtle))
                .bg(components::color(colors.bg.surface))
                .text_sm()
                .text_color(components::color(colors.fg.default))
                .hover(move |style| style.bg(components::color(colors.bg.surface_hover)))
                .id(("service-row", index))
                .child(
                    div()
                        .w(px(SERVICE_NAME_COLUMN_WIDTH))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(service.name.clone()),
                )
                .child(
                    div()
                        .w(px(SERVICE_DISPLAY_COLUMN_WIDTH))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(components::color(colors.fg.muted))
                        .child(service.display_name.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .w(px(SERVICE_STATUS_COLUMN_WIDTH))
                        .child(components::badge(
                            status_label(status, translations),
                            status_colors(status, colors),
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(components::color(if message.is_empty() {
                            colors.fg.subtle
                        } else {
                            colors.danger.text
                        }))
                        .child(if message.is_empty() {
                            "—".to_string()
                        } else {
                            message.to_string()
                        }),
                )
                .child(actions)
        });

        let body = if state.managed_services().is_empty() {
            components::card(colors)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .id("service-empty")
                .child(components::empty_state(
                    ICON_SERVICE,
                    translations.svc_header,
                    translations.svc_empty,
                    colors,
                ))
        } else if visible.is_empty() {
            components::card(colors)
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .id("service-no-match")
                .child(components::empty_state(
                    ICON_SERVICE,
                    translations.svc_no_match,
                    translations.dialog_search,
                    colors,
                ))
        } else {
            components::card(colors)
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .overflow_hidden()
                .id("service-table")
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
                                .flex_1()
                                .min_w(px(0.))
                                .min_h(px(0.))
                                .id("service-list")
                                .overflow_y_scroll()
                                .track_scroll(&self.service_scroll)
                                .on_scroll_wheel(cx.listener(Self::on_scrolled))
                                .children(rows),
                        )
                        .child(self.render_scrollbar(
                            &self.service_scroll,
                            colors,
                            cx,
                            "service-scrollbar",
                            "service-thumb",
                        )),
                )
        };

        let notice = state.refresh_notice().map(|text| {
            div()
                .absolute()
                .bottom_4()
                .right_6()
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(components::color(colors.bg.elevated))
                .border_1()
                .border_color(components::color(colors.border.default))
                .shadow_lg()
                .text_sm()
                .text_color(components::color(colors.success.text))
                .child(text.to_string())
        });

        div()
            .relative()
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .p_6()
            .child(toolbar)
            .child(filter_row)
            .child(body)
            .children(notice)
    }

    pub(super) fn add_clicked(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_add_dialog(cx);
        window.focus(&self.search_focus);
    }

    pub(super) fn start_all_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_all(cx);
    }

    pub(super) fn stop_all_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.stop_all(cx);
    }

    pub(super) fn refresh_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_services(cx);
    }

    pub(super) fn focus_search(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.search_focus);
        cx.notify();
    }

    pub(super) fn focus_service_search(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.service_search_focus);
        cx.notify();
    }

    pub(super) fn open_service_detail(&mut self, name: String, cx: &mut Context<Self>) {
        let lookup = name.clone();
        self.update_state(cx, move |state| state.begin_service_detail(&name));
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::service_details(&lookup) })
                .await;
            state
                .update(cx, |state, cx| {
                    state.set_service_detail(result);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn close_service_detail_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.close_service_detail());
    }

    pub(super) fn toggle_history_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history_open = !self.history_open;
        cx.notify();
    }

    pub(super) fn close_history_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history_open = false;
        cx.notify();
    }

    pub(super) fn clear_history_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_state(cx, |state| state.clear_history());
    }

    pub(super) fn export_services_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let services = self.state.read(cx).managed_services().to_vec();
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let path = cx
                .background_spawn(async { crate::file_dialog::save_file("managed_services.json") })
                .await;
            let Some(path) = path else {
                return;
            };
            let target = path.clone();
            let result = cx
                .background_spawn(
                    async move { backend::export_managed_services(&target, &services) },
                )
                .await;
            state
                .update(cx, |state, cx| {
                    let message = match result {
                        Ok(()) => state
                            .t()
                            .export_done
                            .replace("{}", &path.display().to_string()),
                        Err(error) => error.to_string(),
                    };
                    state.set_refresh_notice(message);
                    cx.notify();
                })
                .ok();
            Timer::after(Duration::from_millis(2500)).await;
            state
                .update(cx, |state, cx| {
                    state.clear_refresh_notice();
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn import_services_clicked(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.clone();
        cx.spawn(async move |_this, cx| {
            let path = cx
                .background_spawn(async { crate::file_dialog::open_file() })
                .await;
            let Some(path) = path else {
                return;
            };
            let result = cx
                .background_spawn(async move { backend::import_managed_services(&path) })
                .await;
            let services = match result {
                Ok(services) => services,
                Err(error) => {
                    state
                        .update(cx, |state, cx| {
                            state.set_refresh_notice(error.to_string());
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let names: Vec<String> = services
                .iter()
                .map(|service| service.name.clone())
                .collect();
            let count = services.len();
            state
                .update(cx, |state, cx| {
                    state.replace_managed_services(services);
                    cx.notify();
                })
                .ok();
            let results = cx
                .background_spawn(async move {
                    backend::execute_service_operation(ServiceOperation::RefreshAll(names))
                })
                .await;
            state
                .update(cx, |state, cx| {
                    for result in results {
                        match result.status {
                            Ok(status) => state.apply_service_success(&result.name, &status),
                            Err(error) => state.apply_service_error(&result.name, &error),
                        }
                    }
                    let message = state.t().import_done.replace("{}", &count.to_string());
                    state.set_refresh_notice(message);
                    cx.notify();
                })
                .ok();
            Timer::after(Duration::from_millis(2500)).await;
            state
                .update(cx, |state, cx| {
                    state.clear_refresh_notice();
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn set_start_type(
        &mut self,
        name: String,
        start_type: StartType,
        cx: &mut Context<Self>,
    ) {
        let state = self.state.clone();
        let history_name = name.clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { backend::set_service_start_type(&name, start_type) })
                .await;
            state
                .update(cx, |state, cx| {
                    match &result {
                        Ok(()) => state.apply_start_type(start_type),
                        Err(error) => state.set_service_detail_error(error),
                    }
                    let ok = result.is_ok();
                    state.record_history(HistoryEntry {
                        action: HistoryAction::StartType,
                        service: history_name.clone(),
                        ok,
                        message: result
                            .err()
                            .map(|error| error.to_string())
                            .unwrap_or_default(),
                    });
                    crate::logging::log(&format!(
                        "StartType {} ({:?}) -> {}",
                        history_name,
                        start_type,
                        if ok { "ok" } else { "failed" }
                    ));
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub(super) fn confirm_delete_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(name) = self.pending_delete.take() {
            self.update_state(cx, move |state| state.remove_service(&name));
        }
    }

    pub(super) fn cancel_delete_clicked(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_delete = None;
        cx.notify();
    }
}
