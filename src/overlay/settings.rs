use super::*;
use crate::ely::{
    agent::{McpServer, McpServerList, ServerState},
    buttons::{IconButton, SegmentedControl},
    forms::{JsonInput, Switch},
    layout::Collapsible,
    primitives::{FocusScope, IconName, Place},
    theme::ActiveTheme,
    typography::{Caption, Overline},
};
use gpui::{ClipboardItem, RenderOnce};

impl AnnotationOverlay {
    fn set_mcp_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.mcp_requested = enabled;
        if self.mcp_starting {
            cx.notify();
            return;
        }
        if enabled && self._server.is_none() {
            self.mcp_starting = true;
            self.settings_status.clear();
            let store = self.store.clone();
            let port = self.api_port;
            cx.spawn(async move |owner, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move { Server::start(store, port) })
                    .await;
                let _ = owner.update(cx, |this, cx| {
                    this.mcp_starting = false;
                    match result {
                        Ok(server) => {
                            let endpoint = format!("http://{}/mcp", server.address());
                            server.set_mcp_enabled(this.mcp_requested);
                            this.mcp_config_input
                                .update(cx, |input, cx| input.set_text(mcp_config(&endpoint), cx));
                            this._server = Some(server);
                            if this.notice.starts_with("API offline:") {
                                this.notice.clear();
                            }
                        }
                        Err(error) => {
                            this.settings_status = format!("Could not start MCP: {error}")
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
            cx.notify();
            return;
        }
        if let Some(server) = &self._server {
            server.set_mcp_enabled(enabled);
        }
        self.settings_status.clear();
        cx.notify();
    }

    fn set_hot_reload_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.hot_reload_pending || !self.hot_reload_status.available {
            return;
        }
        self.hot_reload_pending = true;
        let control = self.hot_reload.clone();
        self._hot_reload_write = Some(cx.spawn(async move |owner, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { control.set_enabled(enabled) })
                .await;
            let _ = owner.update(cx, |this, cx| {
                this.hot_reload_pending = false;
                match result {
                    Ok(status) => this.hot_reload_status = status,
                    Err(error) => {
                        this.hot_reload_status.detail =
                            format!("Could not update hot reload: {error}")
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn settings(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .debug_selector(|| "feedback-settings-trigger-target".into())
            .child(
                IconButton::new("feedback-settings-trigger", IconName::Settings)
                    .tooltip("Settings")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, window, cx| this.open_sheet(true, window, cx))),
            )
    }

    pub(super) fn open_sheet(
        &mut self,
        settings: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        if !self.settings_open && !self.review_open {
            self.sheet_previous_focus = window.focused(cx);
        }
        self.settings_open = settings;
        self.review_open = !settings;
        window.focus(&self.sheet_focus, cx);
        cx.notify();
    }

    pub(super) fn dismiss_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        self.review_open = false;
        if let Some(focus) = self.sheet_previous_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    pub(super) fn sheet(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let owner = cx.entity().downgrade();
        let close_owner = owner.clone();
        let close: std::rc::Rc<dyn Fn(&mut Window, &mut App)> =
            std::rc::Rc::new(move |window, cx| {
                let _ = close_owner.update(cx, |this, cx| this.dismiss_sheet(window, cx));
            });
        let content = if self.settings_open {
            settings_content(self, cx.entity(), close.clone())
        } else {
            surface::notes_body(self, cx.entity(), close.clone(), true, window, cx)
        };
        let sheet = annotation_sheet(content, close, self.settings_open, window, cx);
        FocusScope::new(&self.sheet_focus).trap().child(sheet)
    }
}

fn settings_content(
    this: &AnnotationOverlay,
    owner: Entity<AnnotationOverlay>,
    close: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
) -> gpui::Div {
    let enabled = this._server.as_ref().is_some_and(Server::is_mcp_enabled);
    let endpoint = this
        ._server
        .as_ref()
        .map(|server| format!("http://{}/mcp", server.address()));
    let position_owner = owner.clone();
    let position = SegmentedControl::new(
        "feedback-position",
        if this.banner { "banner" } else { "floating" },
    )
    .segment("floating", "Floating", None)
    .segment("banner", "Banner", None)
    .on_change(move |value, window, cx| {
        close(window, cx);
        position_owner.update(cx, |this, cx| {
            this.set_toolbar_banner(value == "banner", window, cx)
        });
    });
    let copy_owner = owner.clone();
    let config = div()
        .w_full()
        .debug_selector(|| "feedback-mcp-config-code".into())
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .child(JsonInput::new(&this.mcp_config_input));
    let copy = CopyConfigButton {
        button: Button::new(
            "feedback-mcp-copy-config",
            if this.mcp_config_copied {
                "copied!"
            } else {
                "Copy"
            },
        )
        .disabled(endpoint.is_none())
        .on_click(move |_, _, cx| copy_owner.update(cx, |this, cx| this.copy_mcp_config(cx))),
        copied: this.mcp_config_copied,
    };
    let config_owner = owner.clone();
    let config_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .debug_selector(|| "feedback-mcp-config-toggle".into())
                        .child(
                            Button::new("feedback-mcp-config-toggle", "MCP config")
                                .variant(ButtonVariant::Ghost)
                                .icon(if this.mcp_config_expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .on_click(move |_, _, cx| {
                                    config_owner.update(cx, |this, cx| {
                                        this.mcp_config_expanded = !this.mcp_config_expanded;
                                        cx.notify();
                                    });
                                }),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .debug_selector(|| "feedback-mcp-copy-config-target".into())
                        .child(copy),
                ),
        )
        .child(
            Collapsible::new("feedback-mcp-config-details", this.mcp_config_expanded).child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(config)
                    .child(Caption::new(
                        "Turn on to connect an MCP client. Annotations remain available.",
                    )),
            ),
        );
    let toggle_owner = owner.clone();
    let server_list = McpServerList::new(
        "feedback-mcp-server",
        [McpServer {
            key: "gpui-agentation".into(),
            name: "GPUI Agentation".into(),
            detail: if this.mcp_starting {
                "Starting"
            } else if enabled {
                "Streamable HTTP"
            } else {
                "Server is off"
            }
            .into(),
            state: if this.mcp_starting {
                ServerState::Connecting
            } else if enabled {
                ServerState::Connected
            } else {
                ServerState::Off
            },
            tools: if enabled { 6 } else { 0 },
        }],
    )
    .on_toggle(move |_, enabled, _, cx| {
        toggle_owner.update(cx, |this, cx| this.set_mcp_enabled(enabled, cx));
    });
    let position_section = div()
        .debug_selector(|| "feedback-settings-position".into())
        .flex()
        .flex_col()
        .gap_2()
        .child(Overline::new("Position"))
        .child(position);
    let mcp_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(Overline::new("MCP"))
        .child(server_list)
        .child(config_section)
        .when(!this.settings_status.is_empty(), |section| {
            section.child(Caption::new(this.settings_status.clone()))
        });
    let hot_reload = hot_reload_section(this, owner.clone());
    let about = div()
        .debug_selector(|| "feedback-settings-about".into())
        .flex()
        .flex_col()
        .gap_2()
        .child(Overline::new("About"))
        .child(Caption::new(format!(
            "GPUI Agentation {}",
            env!("CARGO_PKG_VERSION")
        )))
        .child(Caption::new(
            "Native annotations for GPUI development builds.",
        ))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("feedback-crates", "crates.io")
                        .size(ControlSize::Sm)
                        .icon(IconName::Package)
                        .on_click(|_, _, cx| {
                            cx.open_url("https://crates.io/crates/gpui-agentation")
                        }),
                )
                .child(StarButton(
                    Button::new("feedback-github", "Star on GitHub")
                        .size(ControlSize::Sm)
                        .variant(ButtonVariant::Secondary)
                        .on_click(|_, _, cx| {
                            cx.open_url("https://github.com/emmanuel-defreitas/gpui-agentation")
                        }),
                )),
        );
    let body = div()
        .id("feedback-settings-scroll")
        .debug_selector(|| "feedback-settings-scroll".into())
        .size_full()
        .min_h_0()
        .overflow_y_scroll()
        .child(
            div()
                .w_full()
                .p_6()
                .flex()
                .flex_col()
                .gap_5()
                .child(position_section)
                .child(hot_reload)
                .child(mcp_section)
                .child(about),
        );
    div().size_full().child(body)
}

fn hot_reload_section(this: &AnnotationOverlay, owner: Entity<AnnotationOverlay>) -> gpui::Div {
    let reload_owner = owner.clone();
    let reload_key_owner = owner.clone();
    div()
        .debug_selector(|| "feedback-settings-hot-reload".into())
        .flex()
        .flex_col()
        .gap_2()
        .child(Overline::new("Hot reload"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child("Server status")
                .child(
                    div()
                        .debug_selector(|| "feedback-hot-reload-toggle".into())
                        // The pinned Ely Switch owns focus and pointer behavior;
                        // supply keyboard activation on its focused event path.
                        .on_key_down(move |event, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "space" | "enter")
                                && !event.keystroke.modifiers.modified()
                                && !event.is_held
                            {
                                reload_key_owner.update(cx, |this, cx| {
                                    this.set_hot_reload_enabled(
                                        !this.hot_reload_status.enabled,
                                        cx,
                                    );
                                });
                                cx.stop_propagation();
                            }
                        })
                        .child(
                            Switch::new(
                                "feedback-hot-reload-toggle",
                                this.hot_reload_status.enabled,
                            )
                            .name("Hot reload server status")
                            .disabled(!this.hot_reload_status.available || this.hot_reload_pending)
                            .on_change(move |enabled, _, cx| {
                                reload_owner.update(cx, |this, cx| {
                                    this.set_hot_reload_enabled(enabled, cx)
                                });
                            }),
                        ),
                ),
        )
        .child(Caption::new(this.hot_reload_status.detail.clone()))
        .child(Caption::new(
            "Rebuilds and restarts this app after a source save. Save app work before enabling.",
        ))
}

pub(super) fn mcp_config(url: &str) -> String {
    serde_json::to_string_pretty(
        &serde_json::json!({ "mcpServers": { "gpui_agentation": { "type": "http", "url": url } } }),
    )
    .expect("static MCP configuration serializes to JSON")
}

impl AnnotationOverlay {
    fn copy_mcp_config(&mut self, cx: &mut Context<Self>) {
        if self._server.is_none() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.mcp_config_input.read(cx).text().to_string(),
        ));
        self.mcp_config_copied = true;
        self._copy_feedback = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = owner.update(cx, |this, cx| {
                this.mcp_config_copied = false;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

#[derive(IntoElement)]
struct CopyConfigButton {
    button: Button,
    copied: bool,
}
impl RenderOnce for CopyConfigButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = if self.copied {
            cx.theme().colors.success
        } else {
            cx.theme().colors.fg
        };
        let mut button = self.button.render(window, cx).into_any_element();
        if let Some(root) = button.downcast_mut::<gpui::Stateful<gpui::Div>>() {
            root.style()
                .refine(&gpui::StyleRefinement::default().text_color(color));
            root.style().flex_direction = Some(gpui::FlexDirection::RowReverse);
            root.extend([Icon::new(if self.copied {
                IconName::Check
            } else {
                IconName::Copy
            })
            .size(IconSize::Sm)
            .color(color)
            .into_any_element()]);
        }
        button
    }
}

// Ely's pinned Sheet has a fixed text-only title row. Compose its Backdrop,
// sizing and transition with a richer header without adding a second title row.
fn annotation_sheet(
    content: gpui::Div,
    close: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    settings: bool,
    window: &mut Window,
    cx: &App,
) -> gpui::AnyElement {
    use crate::ely::{motion, primitives::Backdrop, theme::Elevation, typography::Heading};
    use gpui::{Animation, AnimationExt};
    let dismiss = close.clone();
    let title = if settings { "Settings" } else { "Comments" };
    let size = cx.theme().sheet_size();
    let travel = size.to_pixels(window.rem_size());
    let header = div()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_3()
        .debug_selector(|| "feedback-sheet-header".into())
        .bg(cx.theme().colors.sunken)
        .border_b_1()
        .border_color(cx.theme().colors.border)
        .child(
            Icon::new(if settings {
                IconName::Settings
            } else {
                IconName::MessageSquare
            })
            .size(if settings { IconSize::Lg } else { IconSize::Md }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .when(settings, |title| title.gap_0())
                .flex_1()
                .child(Heading::new(5, title).line_height(gpui::relative(1.15)))
                .child(Caption::new("gpui-dev-mcp")),
        )
        .child(
            IconButton::new("feedback-settings-close", IconName::X)
                .tooltip(format!("Close {title}"))
                .on_click(move |_, window, cx| dismiss(window, cx)),
        );
    let panel = div()
        .flex()
        .flex_col()
        .h_full()
        .w(size)
        .bg(cx.theme().colors.overlay)
        .shadow(cx.theme().elevation(Elevation::Modal))
        .child(header)
        .child(
            div()
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .debug_selector(|| "feedback-sheet-body".into())
                .child(content),
        )
        .with_animation(
            "annotation-sheet-in",
            Animation::new(motion::duration(motion::BASE, cx)).with_easing(motion::ease_out_cubic),
            move |panel, t| panel.mr(-(travel * (1.0 - t))),
        );
    Backdrop::new("feedback-settings-sheet")
        .place(Place::Right)
        .on_dismiss(move |window, cx| close(window, cx))
        .child(panel)
        .into_any_element()
}

#[derive(IntoElement)]
struct StarButton(Button);
impl RenderOnce for StarButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut button = self.0.render(window, cx).into_any_element();
        if let Some(root) = button.downcast_mut::<gpui::Stateful<gpui::Div>>() {
            root.style().flex_direction = Some(gpui::FlexDirection::RowReverse);
            root.extend([Icon::new(IconName::Star)
                .size(IconSize::Sm)
                .color(cx.theme().colors.warning)
                .into_any_element()]);
        }
        button
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn settings_scroll_should_reach_sheet_edges_with_padding_inside_content(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            crate::ely::init_for_tests(cx);
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        });
        let store = Store::memory();
        let session = store
            .ensure_session("Scroll test", "gpui://scroll")
            .unwrap();
        let (overlay, cx) =
            cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
        cx.update(|window, cx| {
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
            overlay.update(cx, |this, cx| this.open_sheet(true, window, cx));
        });
        for height in [560., 900.] {
            cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(height)));
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.run_until_parked();
            let header = cx.debug_bounds("feedback-sheet-header").unwrap();
            let body = cx.debug_bounds("feedback-sheet-body").unwrap();
            let scroll = cx.debug_bounds("feedback-settings-scroll").unwrap();
            assert_eq!(body.top(), header.bottom());
            assert_eq!(body.bottom(), gpui::px(height));
            assert_eq!(scroll, body, "scroll viewport must reach all body edges");
            // Return to the start after resizing; spacing belongs to the
            // content, not a stationary mask around the scroll viewport.
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: scroll.origin + gpui::point(gpui::px(2.), gpui::px(2.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(2000.))),
                modifiers: gpui::Modifiers::none(),
                touch_phase: gpui::TouchPhase::Moved,
            });
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.run_until_parked();
            let position = cx.debug_bounds("feedback-settings-position").unwrap();
            let margin = position.left() - scroll.left();
            assert!(margin > gpui::px(0.));
            assert_eq!(scroll.right() - position.right(), margin);
            assert_eq!(position.top() - scroll.top(), margin);
            // Scroll in the gutter, outside the intentionally locked JSON input.
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: scroll.origin + gpui::point(gpui::px(2.), gpui::px(2.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-2000.))),
                modifiers: gpui::Modifiers::none(),
                touch_phase: gpui::TouchPhase::Moved,
            });
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.run_until_parked();
            let about = cx.debug_bounds("feedback-settings-about").unwrap();
            assert!(about.top() >= scroll.top());
            assert!(about.bottom() <= scroll.bottom() - margin + gpui::px(1.));
        }
    }

    #[gpui::test]
    fn copying_config_should_use_live_endpoint_and_restart_two_second_feedback(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            crate::ely::init_for_tests(cx);
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        });
        let store = Store::memory();
        let session = store
            .ensure_session("Config test", "gpui://config")
            .unwrap();
        let (overlay, cx) =
            cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
        cx.update(|window, cx| {
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
            overlay.update(cx, |this, cx| this.open_sheet(true, window, cx));
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        assert!(cx.debug_bounds("feedback-mcp-config-code").is_none());
        let toggle = cx.debug_bounds("feedback-mcp-config-toggle").unwrap();
        let copy = cx.debug_bounds("feedback-mcp-copy-config-target").unwrap();
        assert_eq!(toggle.center().y, copy.center().y);
        assert!(toggle.right() < copy.left());
        cx.simulate_click(copy.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(overlay.read(cx).mcp_config_copied);
            assert!(!overlay.read(cx).mcp_config_expanded);
        });
        cx.simulate_click(toggle.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        assert!(cx.update(|_, cx| overlay.read(cx).mcp_config_expanded));
        let field = cx.debug_bounds("feedback-mcp-config-code").unwrap();
        assert!(
            field.size.height > gpui::px(100.),
            "formatted JSON grows to multiple lines"
        );
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: field.center(),
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(-80.), gpui::px(-160.))),
            modifiers: gpui::Modifiers::none(),
            touch_phase: gpui::TouchPhase::Moved,
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        assert_eq!(
            cx.debug_bounds("feedback-mcp-config-code").unwrap(),
            field,
            "scrolling over JSON must not move the field"
        );
        let target = cx.debug_bounds("feedback-mcp-copy-config-target").unwrap();
        cx.simulate_click(target.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        cx.update(|_, cx| {
            let this = overlay.read(cx);
            assert!(this.mcp_config_input.read(cx).is_disabled());
            assert!(this.mcp_config_copied);
            let copied = cx.read_from_clipboard().unwrap().text().unwrap();
            let config: serde_json::Value = serde_json::from_str(&copied).unwrap();
            assert_eq!(copied, serde_json::to_string_pretty(&config).unwrap());
            assert_eq!(copied, this.mcp_config_input.read(cx).text());
            assert_eq!(
                config["mcpServers"]["gpui_agentation"]["url"],
                format!("http://{}/mcp", this._server.as_ref().unwrap().address())
            );
            assert_eq!(config["mcpServers"]["gpui_agentation"]["type"], "http");
        });
        cx.executor().advance_clock(Duration::from_millis(1500));
        cx.run_until_parked();
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.copy_mcp_config(cx)));
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(1000));
        cx.run_until_parked();
        cx.update(|_, cx| assert!(overlay.read(cx).mcp_config_copied));
        cx.executor().advance_clock(Duration::from_millis(1001));
        cx.run_until_parked();
        cx.update(|_, cx| assert!(!overlay.read(cx).mcp_config_copied));
    }

    #[gpui::test]
    fn starting_mcp_should_retain_an_off_request_until_start_finishes(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            crate::ely::init_for_tests(cx);
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        });
        let store = Store::memory();
        let session = store
            .ensure_session("Startup test", "gpui://startup")
            .unwrap();
        let (overlay, cx) =
            cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
        cx.update(|_, cx| {
            overlay.update(cx, |this, cx| {
                this._server.take();
                this.set_mcp_enabled(true, cx);
                assert!(this.mcp_starting);
                this.set_mcp_enabled(false, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|_, cx| {
            let this = overlay.read(cx);
            assert!(!this.mcp_starting);
            assert!(
                !this
                    ._server
                    .as_ref()
                    .expect("server started")
                    .is_mcp_enabled()
            );
        });
    }
}

#[cfg(test)]
mod hot_reload_tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext};

    #[gpui::test]
    fn hot_reload_switch_updates_the_runner_without_closing_settings(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("enabled"), "0").unwrap();
        std::fs::write(directory.path().join("status"), "Off").unwrap();
        cx.update(|cx| {
            crate::ely::init_for_tests(cx);
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        });
        let store = Store::memory();
        let session = store.ensure_session("Reload test", "gpui://test").unwrap();
        let control = crate::hot_reload::HotReload::for_directory(directory.path().into());
        let (overlay, cx) = cx.add_window_view(|window, cx| {
            AnnotationOverlay::new_with_hot_reload(store, session, 0, window, cx, control)
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.open_sheet(true, window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.executor().advance_clock(Duration::from_millis(400));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = cx.debug_bounds("feedback-hot-reload-toggle").unwrap();
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(overlay.read(cx).settings_open);
            assert!(
                overlay.read(cx).hot_reload_status.enabled,
                "bounds={bounds:?}, status={:?}, pending={}",
                overlay.read(cx).hot_reload_status,
                overlay.read(cx).hot_reload_pending
            );
        });
        assert_eq!(
            std::fs::read_to_string(directory.path().join("enabled")).unwrap(),
            "1"
        );
    }
    struct ReloadSection {
        overlay: Entity<AnnotationOverlay>,
        focus: FocusHandle,
        _observe: Subscription,
    }
    impl Render for ReloadSection {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            FocusScope::new(&self.focus).child(hot_reload_section(
                self.overlay.read(cx),
                self.overlay.clone(),
            ))
        }
    }
    #[gpui::test]
    fn hot_reload_switch_is_reachable_by_tab_and_space(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("enabled"), "0").unwrap();
        std::fs::write(directory.path().join("status"), "Off").unwrap();
        cx.update(|cx| {
            crate::ely::init_for_tests(cx);
            crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        });
        let control = crate::hot_reload::HotReload::for_directory(directory.path().into());
        let (_, cx) = cx.add_window_view(|window, cx| {
            let store = Store::memory();
            let session = store
                .ensure_session("Reload keyboard", "gpui://test")
                .unwrap();
            let overlay = cx.new(|cx| {
                AnnotationOverlay::new_with_hot_reload(store, session, 0, window, cx, control)
            });
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            let observe = cx.observe(&overlay, |_, _, cx| cx.notify());
            ReloadSection {
                overlay,
                focus,
                _observe: observe,
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("tab space");
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("enabled")).unwrap(),
            "1"
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("space");
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("enabled")).unwrap(),
            "0"
        );
    }
}
