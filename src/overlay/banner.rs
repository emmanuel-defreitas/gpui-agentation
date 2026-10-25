use super::surface_button::SurfaceButton;
use super::*;
use crate::ely::{
    data_display::Presence, forms::ColorSwatch, media::Tool, messaging::OnlineStatus,
    overlays::Popover, primitives::IconName, theme::ActiveTheme,
};

impl AnnotationOverlay {
    pub(super) fn set_toolbar_banner(
        &mut self,
        banner: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_toolbar_position(
            if banner {
                crate::ToolbarPosition::Banner
            } else {
                crate::ToolbarPosition::Floating
            },
            cx,
        );
    }

    pub(super) fn set_toolbar_position(
        &mut self,
        position: crate::ToolbarPosition,
        cx: &mut Context<Self>,
    ) {
        let preference = crate::store::ToolbarPreference {
            position,
            at: self.toolbar_at,
        };
        if let Err(error) = self
            .store
            .set_toolbar_preference(&self.session.app_name, preference)
        {
            self.notice = error.to_string();
            cx.notify();
            return;
        }
        self.banner = position == crate::ToolbarPosition::Banner;
        self.toolbar_hidden = position == crate::ToolbarPosition::None;
        self.toolbar_drag = None;
        self.toolbar_width = None;
        cx.notify();
    }

    pub(super) fn persist_toolbar(&mut self, cx: &mut Context<Self>) {
        let position = if self.toolbar_hidden {
            crate::ToolbarPosition::None
        } else if self.banner {
            crate::ToolbarPosition::Banner
        } else {
            crate::ToolbarPosition::Floating
        };
        if let Err(error) = self.store.set_toolbar_preference(
            &self.session.app_name,
            crate::store::ToolbarPreference {
                position,
                at: self.toolbar_at,
            },
        ) {
            self.notice = error.to_string();
            cx.notify();
        }
    }

    pub(super) fn banner_toolbar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut tools = div()
            .flex()
            .flex_wrap()
            .max_w(window.viewport_size().width - gpui::px(24.0))
            .items_center()
            .gap_1()
            .child(
                SurfaceButton::new(
                    "feedback-mode",
                    if self.active {
                        "Annotations on"
                    } else {
                        "Annotations off"
                    },
                    IconName::Power,
                    self.active,
                )
                .on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx))),
            );
        for (id, tool, icon, label) in [
            ("inspect", None, IconName::Crosshair, "Inspect"),
            ("box", Some(Tool::Box), IconName::Square, "Box"),
            ("arrow", Some(Tool::Arrow), IconName::ArrowUpRight, "Arrow"),
            ("pen", Some(Tool::Pen), IconName::PenLine, "Pen"),
            (
                "comment",
                Some(Tool::Pin),
                IconName::MessageSquare,
                "Comment",
            ),
            ("text", Some(Tool::Select), IconName::Type, "Textbox"),
        ] {
            tools = tools.child(
                div().debug_selector(move || format!("banner-{id}")).child(
                    SurfaceButton::new(
                        format!("feedback-banner-tool-{id}"),
                        if self.active {
                            format!("{label} · {}", super::surface::tool_shortcut(tool))
                        } else {
                            label.to_string()
                        },
                        icon,
                        self.active && self.tool == tool,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose_tool(tool, window, cx);
                    })),
                ),
            );
            if id == "inspect" {
                tools = tools.child(
                    div()
                        .w_px()
                        .h_5()
                        .mx_1()
                        .bg(cx.theme().colors.bg.opacity(0.3)),
                );
            }
        }
        let palette_owner = cx.entity().downgrade();
        let selected_index = self.color;
        let selected = cx.theme().colors.chart[selected_index];
        let colors = Popover::with_opener(
            "banner-palette",
            move |toggle| {
                div()
                    .debug_selector(|| "banner-palette-trigger".into())
                    .child(
                        ColorSwatch::new("banner-current-color", selected)
                            .selected(true)
                            .on_click(move |window, cx| toggle(window, cx)),
                    )
            },
            move |close, _, cx| {
                let mut colors = div().flex().items_center().gap_2();
                for color in 0..4 {
                    let owner = palette_owner.clone();
                    let close = close.clone();
                    colors = colors.child(
                        div()
                            .debug_selector(move || format!("banner-palette-color-{color}"))
                            .child(
                                ColorSwatch::new(
                                    ("feedback-banner-color", color),
                                    cx.theme().colors.chart[color],
                                )
                                .selected(
                                    palette_owner
                                        .upgrade()
                                        .is_some_and(|owner| owner.read(cx).color == color),
                                )
                                .on_click(move |window, cx| {
                                    close(window, cx);
                                    let _ = owner.update(cx, |this, cx| {
                                        this.color = color;
                                        cx.notify();
                                    });
                                }),
                            ),
                    );
                }
                colors
            },
        );
        if f32::from(window.viewport_size().width) >= 980.0 {
            for color in 0..4 {
                let owner = cx.entity().downgrade();
                tools = tools.child(
                    ColorSwatch::new(
                        ("feedback-banner-color", color),
                        cx.theme().colors.chart[color],
                    )
                    .selected(self.color == color)
                    .on_click(move |_, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.color = color;
                            cx.notify();
                        });
                    }),
                );
            }
        } else {
            tools = tools.child(colors);
        }
        let count = self
            .store
            .app_annotations(&self.session.app_name)
            .map(|notes| notes.len())
            .unwrap_or(0);
        let comments = SurfaceButton::new(
            "feedback-banner-comments",
            "Comments",
            IconName::MessageSquare,
            false,
        )
        .on_click(cx.listener(|this, _, window, cx| {
            this.open_sheet(false, window, cx);
        }));
        let comments = surface::annotation_count(count, comments, cx);
        let (presence, status) = if self.mcp_starting {
            (Presence::Away, "Starting")
        } else if self._server.as_ref().is_some_and(Server::is_mcp_enabled) {
            (Presence::Online, "Running")
        } else {
            (Presence::Offline, "Off")
        };
        let status = div()
            .id("feedback-banner-mcp-status")
            .role(gpui::Role::Button)
            .tab_index(0)
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| this.toggle_mcp(cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                    this.toggle_mcp(cx);
                    cx.stop_propagation();
                }
            }))
            .aria_label(format!("MCP {status}"))
            .debug_selector(|| "banner-mcp-status".into())
            .flex()
            .items_center()
            .gap_1p5()
            .mx_2()
            .text_size(cx.theme().text_size(crate::ely::theme::TextSize::Sm))
            .child(
                div()
                    .w(cx.theme().status_dot())
                    .overflow_hidden()
                    .child(OnlineStatus::new(presence)),
            )
            .child(
                gpui::svg()
                    .path("agentation/mcp.svg")
                    .size(gpui::rems(1.0))
                    .text_color(cx.theme().colors.fg),
            )
            .child(format!("MCP {status}"));
        let trailing = div()
            .flex()
            .items_center()
            .gap_1()
            .ml_auto()
            .flex_none()
            .child(self.history_controls(cx))
            .child(status)
            .child(
                div()
                    .debug_selector(|| "banner-notes".into())
                    .pr_1()
                    .child(comments),
            )
            .child(
                div()
                    .debug_selector(|| "feedback-settings-trigger-target".into())
                    .child(
                        SurfaceButton::new(
                            "feedback-banner-settings",
                            "Settings",
                            IconName::Settings,
                            false,
                        )
                        .icon_only()
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_sheet(true, window, cx)),
                        ),
                    ),
            );
        // Banner's pinned API mandates a status icon/message. This toolbar needs
        // neither, so compose the same inline strip with Ely controls and tokens.
        div()
            .id("feedback-banner")
            .role(gpui::Role::Toolbar)
            .aria_label("Annotation tools")
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .text_color(cx.theme().colors.fg)
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .child(tools)
            .child(trailing)
            .into_any_element()
    }
}

#[cfg(test)]
#[path = "banner.test.rs"]
mod tests;
