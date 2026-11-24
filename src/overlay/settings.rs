use super::*;
use gpui::{ClipboardItem, RenderOnce};

impl AnnotationOverlay {
    pub(super) fn toggle_mcp(&mut self, cx: &mut Context<Self>) {
        let enabled = if self.mcp_starting {
            self.mcp_requested
        } else {
            self._server.as_ref().is_some_and(Server::is_mcp_enabled)
        };
        self.set_mcp_enabled(!enabled, cx);
    }

    pub(super) fn set_mcp_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
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
            let host = self.host;
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
                            let input = this.mcp_config_input.clone();
                            let config = mcp_config(&endpoint);
                            cx.defer(move |cx| {
                                let _ = host.update(cx, |_, window, cx| {
                                    input
                                        .update(cx, |input, cx| input.set_value(config, window, cx))
                                });
                            });
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

    pub(super) fn open_sheet(
        &mut self,
        settings: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if settings && super::settings_popup::available(cx) {
            super::settings_popup::open_for(window.window_handle().window_id(), cx);
            return;
        }
        if !self.dismiss_note(window, cx) {
            return;
        }
        self.stop_inspection(window, cx);
        self.inspected_id = None;
        self.inspected_configuration = None;
        if !self.settings_open && !self.review_open {
            self.sheet_previous_focus = window.focused(cx);
        }
        self.settings_open = settings;
        self.hovered_note = None;
        self.review_open = !settings;
        window.focus(&self.sheet_focus, cx);
        cx.notify();
    }

    pub(super) fn dismiss_sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hovered_note = None;
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
            settings_content(self, cx.entity(), sheet_config(self, cx.entity()), cx)
        } else {
            surface::notes_body(self, cx.entity(), close.clone(), true, window, cx)
        };
        let sheet = annotation_sheet(
            content,
            close,
            self.settings_open,
            self.sheet_focus.clone(),
            window,
            cx,
        );
        sheet
    }
}

pub(super) struct ConfigControls {
    pub input: Entity<EditorState>,
    pub copied: bool,
    pub expanded: bool,
    pub copy: std::rc::Rc<dyn Fn(&mut App)>,
    pub toggle: std::rc::Rc<dyn Fn(&mut App)>,
}

fn sheet_config(this: &AnnotationOverlay, owner: Entity<AnnotationOverlay>) -> ConfigControls {
    let copy_owner = owner.clone();
    ConfigControls {
        input: this.mcp_config_input.clone(),
        copied: this.mcp_config_copied,
        expanded: this.mcp_config_expanded,
        copy: std::rc::Rc::new(move |cx| {
            copy_owner.update(cx, |this, cx| this.copy_mcp_config(cx))
        }),
        toggle: std::rc::Rc::new(move |cx| {
            owner.update(cx, |this, cx| {
                this.mcp_config_expanded = !this.mcp_config_expanded;
                cx.notify();
            })
        }),
    }
}

pub(super) fn settings_content(
    this: &AnnotationOverlay,
    owner: Entity<AnnotationOverlay>,
    controls: ConfigControls,
    cx: &App,
) -> gpui::Div {
    let owner = owner.downgrade();
    let enabled = this._server.as_ref().is_some_and(Server::is_mcp_enabled);
    let endpoint = this
        ._server
        .as_ref()
        .map(|server| format!("http://{}/mcp", server.address()));
    let copy_action = controls.copy;
    let config = div()
        .w_full()
        .debug_selector(|| "feedback-mcp-config-code".into())
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .child(
            Editor::new(&controls.input)
                .h(gpui::px(200.))
                .readonly(true)
                .aria_label("MCP config"),
        );
    let copy = CopyConfigButton {
        button: Button::new("feedback-mcp-copy-config")
            .label(if controls.copied { "copied!" } else { "Copy" })
            .ghost()
            .small()
            .disabled(endpoint.is_none())
            .on_click(move |_, _, cx| copy_action(cx)),
        copied: controls.copied,
    };
    let toggle_config = controls.toggle.clone();
    let toggle_owner = owner.clone();
    let server_list = div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div().flex_1().child("GPUI Agentation").child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if this.mcp_starting {
                        "Starting"
                    } else if enabled {
                        "Streamable HTTP · 6 tools"
                    } else {
                        "Server is off"
                    }),
            ),
        )
        .child(
            Switch::new("feedback-mcp-server")
                .checked(enabled)
                .disabled(this.mcp_starting)
                .accessibility_label("MCP Server")
                .on_change(move |enabled, _, cx| {
                    let _ = toggle_owner.update(cx, |this, cx| this.set_mcp_enabled(*enabled, cx));
                }),
        );
    let toggle_chevron = controls.toggle;
    let config_section = div()
        .flex()
        .flex_col()
        .rounded(cx.theme().radius_tokens().md)
        .border_1()
        .border_color(cx.theme().colors.border)
        .child(
            div()
                .debug_selector(|| "feedback-mcp-config-header".into())
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .py_1()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .debug_selector(|| "feedback-mcp-config-toggle".into())
                        .child(DisclosureButton {
                            button: Button::new("feedback-mcp-config-toggle")
                                .accessibility_label("MCP")
                                .ghost()
                                .w_full()
                                .on_click(move |_, _, cx| toggle_config(cx)),
                            expanded: controls.expanded,
                        }),
                )
                .child(
                    div()
                        .flex_none()
                        .debug_selector(|| "feedback-mcp-copy-config-target".into())
                        .child(copy),
                )
                .child(
                    Button::new("feedback-mcp-config-chevron")
                        .icon(if controls.expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .ghost()
                        .small()
                        .tooltip(if controls.expanded {
                            "Collapse MCP configuration"
                        } else {
                            "Expand MCP configuration"
                        })
                        .on_click(move |_, _, cx| toggle_chevron(cx)),
                ),
        )
        .child(
            Collapsible::new().open(controls.expanded).content(
                div()
                    .border_t_1()
                    .border_color(cx.theme().colors.border)
                    .p_2()
                    .child(config),
            ),
        );
    let mcp_section = div()
        .debug_selector(|| "feedback-settings-mcp".into())
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("MCP"),
        )
        .child(server_list)
        .child(config_section)
        .when(!this.settings_status.is_empty(), |section| {
            section.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(this.settings_status.clone()),
            )
        });
    let about = div()
        .debug_selector(|| "feedback-settings-about".into())
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("About"),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("GPUI Agentation {}", env!("CARGO_PKG_VERSION"))),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Native annotations for GPUI development builds."),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("feedback-crates")
                        .label("crates.io")
                        .small()
                        .icon(IconName::Package)
                        .on_click(|_, _, cx| {
                            cx.open_url("https://crates.io/crates/gpui-agentation")
                        }),
                )
                .child(StarButton(
                    Button::new("feedback-github")
                        .label("Star on GitHub")
                        .small()
                        .secondary()
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
                .child(mcp_section)
                .child(about),
        );
    div().size_full().child(body)
}

pub(super) fn mcp_config(url: &str) -> String {
    serde_json::to_string_pretty(
        &serde_json::json!({ "mcpServers": { "gpui_agentation": { "type": "http", "url": url } } }),
    )
    .expect("static MCP configuration serializes to JSON")
}

impl AnnotationOverlay {
    pub(super) fn copy_mcp_config(&mut self, cx: &mut Context<Self>) {
        if self._server.is_none() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.mcp_config_input.read(cx).value().to_string(),
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

// Keep the disclosure state on the Kit button that owns focus and activation.
#[derive(IntoElement)]
pub(super) struct DisclosureButton {
    pub(super) button: Button,
    pub(super) expanded: bool,
}

impl RenderOnce for DisclosureButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.button.aria_expanded(self.expanded).child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    gpui::svg()
                        .path("agentation/mcp.svg")
                        .size(gpui::rems(1.))
                        .debug_selector(|| "feedback-mcp-config-icon".into()),
                )
                .child("MCP"),
        )
    }
}

#[derive(IntoElement)]
struct CopyConfigButton {
    button: Button,
    copied: bool,
}
impl RenderOnce for CopyConfigButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.button
            .icon(if self.copied {
                IconName::Check
            } else {
                IconName::Copy
            })
            .text_color(if self.copied {
                cx.theme().success
            } else {
                cx.theme().foreground
            })
    }
}

fn annotation_sheet(
    content: gpui::Div,
    close: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    settings: bool,
    focus: FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    use gpui::{Animation, AnimationExt};
    let dismiss = close.clone();
    let title = if settings { "Settings" } else { "Comments" };
    let size = cx.theme().control_height(ControlSize::Lg) * 12.0;
    let travel = size.to_pixels(window.rem_size());
    let header = div()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_3()
        .debug_selector(|| "feedback-sheet-header".into())
        .bg(cx.theme().colors.muted)
        .border_b_1()
        .border_color(cx.theme().colors.border)
        .child(
            Icon::new(if settings {
                IconName::Settings
            } else {
                IconName::MessageSquare
            })
            .size(gpui::px(if settings { 20. } else { 16. })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .when(settings, |title| title.gap_0())
                .flex_1()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title)
                        .line_height(gpui::relative(1.15)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("gpui-dev-mcp"),
                ),
        )
        .child(
            Button::new("feedback-settings-close")
                .icon(IconName::X)
                .tooltip(format!("Close {title}"))
                .on_click(move |_, window, cx| dismiss(window, cx)),
        );
    let panel = div()
        .flex()
        .flex_col()
        .h_full()
        .w(size)
        .bg(cx.theme().colors.popover)
        .shadow(cx.theme().shadow_tokens().lg)
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
        .map(|panel| {
            if cx.reduce_motion() || cx.theme().motion.duration_normal.is_zero() {
                panel.into_any_element()
            } else {
                panel
                    .with_animation(
                        "annotation-sheet-in",
                        Animation::new(motion::duration(motion::BASE, cx))
                            .with_easing(motion::ease_out_cubic),
                        move |panel, t| panel.mr(-(travel * (1.0 - t))),
                    )
                    .into_any_element()
            }
        });
    gpui::base::Sheet::new(cx)
        .focus_handle(focus)
        .overlay(div().size_full().bg(gpui::black().opacity(0.35)))
        .surface(
            div()
                .absolute()
                .right_0()
                .top_0()
                .h_full()
                .w(size)
                .occlude()
                .child(panel),
        )
        .on_close(move |_, window, cx| close(window, cx))
        .into_any_element()
}

#[derive(IntoElement)]
struct StarButton(Button);
impl RenderOnce for StarButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.0.icon(IconName::Star)
    }
}
