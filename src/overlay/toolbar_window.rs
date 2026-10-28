//! One vertical companion window per consumer, outside its annotation/picker tree.
use super::*;
use crate::ely::{
    forms::{ColorSwatch, Switch},
    media::Tool,
    primitives::{Divider, FocusScope, IconName},
    theme::Elevation,
};
use gpui::{
    AnyWindowHandle, Bounds, Pixels, WeakEntity, WindowBounds, WindowOptions, point, px, size,
};

#[cfg(target_os = "macos")]
#[path = "toolbar_window/mac.rs"]
#[allow(deprecated, unexpected_cfgs)]
mod native;

#[cfg(agentation_external_inspector)]
mod width;

const GAP: f32 = 14.0;
pub(super) const TOOLS: [(Option<Tool>, IconName, &str, &str); 6] = [
    (None, IconName::Crosshair, "Inspect", "inspect"),
    (Some(Tool::Box), IconName::Square, "Box", "box"),
    (Some(Tool::Arrow), IconName::ArrowUpRight, "Arrow", "arrow"),
    (Some(Tool::Pen), IconName::PenLine, "Pen", "pen"),
    (
        Some(Tool::Pin),
        IconName::MessageSquare,
        "Comment",
        "comment",
    ),
    (Some(Tool::Select), IconName::Type, "Textbox", "text"),
];

#[derive(Clone, PartialEq)]
struct ToolbarState {
    active: bool,
    tool: Option<Tool>,
    color: usize,
    count: usize,
    mcp: bool,
    #[cfg(agentation_external_inspector)]
    inspector: Option<Entity<gpui::Inspector>>,
    #[cfg(agentation_external_inspector)]
    rail_width: Pixels,
}

#[derive(Clone, PartialEq)]
pub(super) struct ToolbarSpec {
    bounds: Bounds<Pixels>,
    visible: bool,
    state: ToolbarState,
}

pub(super) struct ToolbarWindow {
    owner: WeakEntity<AnnotationOverlay>,
    host: AnyWindowHandle,
    focus: FocusHandle,
    state: ToolbarState,
    _activation: Subscription,
    #[cfg(agentation_external_inspector)]
    _feedback: Entity<inspector::external::ExternalFeedback>,
    #[cfg(agentation_external_inspector)]
    _external_release: Subscription,
    #[cfg(agentation_external_inspector)]
    width: width::WidthAnimation,
    #[cfg(agentation_external_inspector)]
    _content_resize: Subscription,
}

fn dock_bounds(host: Bounds<Pixels>, width: Pixels) -> Bounds<Pixels> {
    Bounds::new(
        point(host.right() + px(GAP), host.top()),
        size(width, host.size.height),
    )
}

impl AnnotationOverlay {
    fn toolbar_spec(&self, window: &Window, cx: &App) -> ToolbarSpec {
        let width = cx
            .theme()
            .switch_track()
            .width
            .to_pixels(window.rem_size())
            .max(
                cx.theme()
                    .control_height(ControlSize::Sm)
                    .to_pixels(window.rem_size()),
            )
            + gpui::rems(1.0).to_pixels(window.rem_size())
            + px(2.0);
        let fullscreen =
            window.is_fullscreen() || window.is_simple_fullscreen() || window.is_maximized();
        #[cfg(target_os = "macos")]
        let fullscreen = fullscreen || native::fills_screen(window);
        #[cfg(agentation_external_inspector)]
        let inspector = window.inspector_entity();
        #[cfg(agentation_external_inspector)]
        let total_width = width
            + if inspector.is_some() {
                px(420.)
            } else {
                px(0.)
            };
        #[cfg(not(agentation_external_inspector))]
        let total_width = width;
        ToolbarSpec {
            bounds: dock_bounds(window.bounds(), total_width),
            visible: !self.toolbar_hidden
                && !fullscreen
                && window.is_visible()
                && (window.is_window_active()
                    || self.companion_toolbar_active
                    || super::settings_popup::is_open_for(window.window_handle().window_id(), cx)),
            state: ToolbarState {
                active: self.active,
                tool: self.tool,
                color: self.color,
                count: self
                    .store
                    .app_annotations(&self.session.app_name)
                    .map_or(0, |notes| notes.len()),
                mcp: self._server.as_ref().is_some_and(Server::is_mcp_enabled),
                #[cfg(agentation_external_inspector)]
                inspector,
                #[cfg(agentation_external_inspector)]
                rail_width: width,
            },
        }
    }

    pub(super) fn sync_toolbar_window(&mut self, window: &Window, cx: &mut Context<Self>) {
        let desired = self.toolbar_spec(window, cx);
        if self.toolbar_sync_pending
            || self.companion_toolbar_spec.as_ref() == Some(&desired)
            || self.companion_toolbar_failure.as_ref() == Some(&desired)
        {
            return;
        }
        let settle_activation = !desired.visible
            && !self.toolbar_hidden
            && window.is_visible()
            && self.companion_toolbar.is_some();
        self.toolbar_sync_pending = true;
        let owner = cx.entity().downgrade();
        let host = window.window_handle();
        let reconcile = move |cx: &mut App| {
            let _ = host.update(cx, |_, window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    this.toolbar_sync_pending = false;
                    this.reconcile_toolbar(window, cx);
                });
            });
        };
        if settle_activation {
            // AppKit sends resign-key before become-key during a panel handoff.
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(30))
                    .await;
                cx.update(reconcile);
            })
            .detach();
        } else {
            cx.defer(reconcile);
        }
    }

    fn reconcile_toolbar(&mut self, host: &mut Window, cx: &mut Context<Self>) {
        let spec = self.toolbar_spec(host, cx);
        if self.companion_toolbar_spec.as_ref() == Some(&spec) {
            return;
        }
        if let Some(handle) = self.companion_toolbar {
            let applied = handle
                .update(cx, |toolbar, window, cx| {
                    if toolbar.state != spec.state {
                        toolbar.state = spec.state.clone();
                        cx.notify();
                    }
                    #[cfg(agentation_external_inspector)]
                    toolbar.retarget_width(spec.bounds.size.width, spec.visible, window, cx);
                    #[cfg(agentation_external_inspector)]
                    let width = toolbar.width.current;
                    #[cfg(not(agentation_external_inspector))]
                    let width = spec.bounds.size.width;
                    #[cfg(target_os = "macos")]
                    if native::place(host, window, width, spec.visible, cx) {
                        return true;
                    }
                    // Backends without a native parent/position API retain one window
                    // when only its content/height changes, and reopen it on a move.
                    let same_origin = self
                        .companion_toolbar_spec
                        .as_ref()
                        .is_some_and(|previous| previous.bounds.origin == spec.bounds.origin);
                    if spec.visible && same_origin {
                        window.resize(size(width, spec.bounds.size.height));
                        return true;
                    }
                    false
                })
                .unwrap_or(false);
            if applied {
                self.companion_toolbar_spec = Some(spec);
                return;
            }
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            self.companion_toolbar = None;
            self.companion_toolbar_active = false;
        }
        self.companion_toolbar_spec = Some(spec.clone());
        if !spec.visible {
            return;
        }
        let owner = cx.entity().downgrade();
        let host_handle = host.window_handle();
        let rem_size = host.rem_size();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(spec.bounds)),
            titlebar: None,
            focus: false,
            window_decorations: None,
            kind: gpui::WindowKind::PopUp,
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: host.display(cx).map(|display| display.id()),
            ..Default::default()
        };
        match cx.open_window(options, |window, cx| {
            window.set_rem_size(rem_size);
            window.set_window_title("Agentation tools");
            #[cfg(target_os = "macos")]
            native::place(host, window, spec.bounds.size.width, true, cx);
            cx.new(|cx| {
                let activation =
                    cx.observe_window_activation(window, |this: &mut ToolbarWindow, window, cx| {
                        this.notify_host(window, cx);
                    });
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                #[cfg(agentation_external_inspector)]
                let feedback = inspector::external::mount(owner.clone(), host_handle, window, cx);
                #[cfg(agentation_external_inspector)]
                let external_release = {
                    let id = window.window_handle().window_id();
                    cx.on_release(move |_, cx| {
                        crate::ely::tooling::set_inspector_target(id, None, cx)
                    })
                };
                #[cfg(agentation_external_inspector)]
                let content_resize =
                    cx.observe_window_bounds(window, |this: &mut ToolbarWindow, window, cx| {
                        this.confirm_content_reveal(window, cx);
                    });
                ToolbarWindow {
                    owner,
                    host: host_handle,
                    focus,
                    state: spec.state.clone(),
                    _activation: activation,
                    #[cfg(agentation_external_inspector)]
                    _feedback: feedback,
                    #[cfg(agentation_external_inspector)]
                    _external_release: external_release,
                    #[cfg(agentation_external_inspector)]
                    width: width::WidthAnimation::new(
                        spec.bounds.size.width,
                        spec.visible,
                        spec.state.inspector.clone(),
                    ),
                    #[cfg(agentation_external_inspector)]
                    _content_resize: content_resize,
                }
            })
        }) {
            Ok(handle) => {
                self.companion_toolbar_failure = None;
                self.companion_toolbar = Some(handle);
            }
            Err(error) => {
                self.companion_toolbar_failure = Some(spec);
                self.companion_toolbar_spec = None;
                self.notice = format!("Could not open annotation toolbar: {error}");
                cx.notify();
            }
        }
    }
}

impl ToolbarWindow {
    fn notify_host(&self, window: &Window, cx: &mut App) {
        let active = window.is_window_active();
        let palette_id = window.window_handle().window_id();
        let owner = self.owner.clone();
        let host = self.host;
        cx.defer(move |cx| {
            let _ = host.update(cx, |_, window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if this
                        .companion_toolbar
                        .is_none_or(|toolbar| toolbar.window_id() != palette_id)
                    {
                        return;
                    }
                    this.companion_toolbar_active = active;
                    this.sync_debug_menu(host.window_id(), cx);
                    this.sync_toolbar_window(window, cx);
                    cx.notify();
                });
            });
        });
    }

    fn action(&self, name: impl Into<String>, cx: &mut App) {
        let host = self.host;
        let owner = self.owner.clone();
        let name = name.into();
        let activate_host = name != "settings" || !super::settings_popup::available(cx);
        cx.defer(move |cx| {
            let _ = host.update(cx, |_, window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if let Some((tool, _, _, _)) =
                        TOOLS.iter().find(|(_, _, _, command)| *command == name)
                    {
                        if this.settings_open || this.review_open {
                            this.dismiss_sheet(window, cx);
                        }
                        this.choose_tool(*tool, window, cx);
                    } else if let Some(color) = name
                        .strip_prefix("color-")
                        .and_then(|color| color.parse::<usize>().ok())
                        .filter(|color| *color < 4)
                    {
                        this.color = color;
                        cx.notify();
                    } else {
                        this.dispatch_command(&commands::AnnotationCommand { name }, window, cx);
                    }
                });
                if activate_host {
                    window.activate_window();
                }
            });
        });
    }

    fn button(
        &self,
        id: impl Into<gpui::ElementId>,
        label: &'static str,
        icon: IconName,
        command: &'static str,
        selected: bool,
        cx: &Context<Self>,
    ) -> super::surface_button::SurfaceButton {
        super::surface_button::SurfaceButton::new(id, label, icon, selected)
            .when(command != "stop", |button| button.icon_only())
            .when(command == "stop", |button| {
                button.stacked().shortcut("escape")
            })
            .when_some(commands::shortcut(command), |button, key| {
                button.shortcut(key)
            })
            .on_click(cx.listener(move |this, _, _, cx| this.action(command, cx)))
    }
}

impl Render for ToolbarWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(agentation_external_inspector)]
        self.confirm_content_reveal(_window, cx);
        let inspecting = self.state.active && self.state.tool.is_none();
        let mode_owner = cx.entity().downgrade();
        let mut tools = div()
            .id("feedback-companion-tools-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        Switch::new("feedback-mode-switch", self.state.active)
                            .name("Annotation mode")
                            .on_change(move |_, _, cx| {
                                let _ = mode_owner.update(cx, |this, cx| this.action("toggle", cx));
                            }),
                    )
                    .children(TOOLS.into_iter().enumerate().filter_map(
                        |(index, (tool, icon, label, command))| {
                            if inspecting && tool.is_some() {
                                return None;
                            }
                            let stop = inspecting && tool.is_none();
                            let selector = format!(
                                "companion-tool-{}",
                                surface::tool_shortcut(tool).to_ascii_lowercase()
                            );
                            Some(
                                div()
                                    .debug_selector(move || selector.clone())
                                    .child(self.button(
                                        ("feedback-tool", index),
                                        if stop { "Stop" } else { label },
                                        icon,
                                        if stop { "stop" } else { command },
                                        self.state.active && self.state.tool == tool,
                                        cx,
                                    )),
                            )
                        },
                    )),
            );
        if !inspecting {
            tools = tools.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .pt_1()
                    .children((0..4).map(|color| {
                        let owner = cx.entity().downgrade();
                        ColorSwatch::new(("feedback-color", color), cx.theme().colors.chart[color])
                            .selected(color == self.state.color)
                            .on_click(move |_, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.action(format!("color-{color}"), cx)
                                });
                            })
                    })),
            );
        }
        let comments = surface::annotation_count(
            self.state.count,
            self.button(
                "feedback-review",
                "Comments",
                IconName::MessageSquare,
                "comments",
                false,
                cx,
            ),
            cx,
        );
        let theme = cx.theme();
        let rail =
            div()
                .id("feedback-native-toolbar")
                .debug_selector(|| "feedback-native-toolbar".into())
                .role(gpui::Role::Toolbar)
                .aria_label("Annotation tools")
                .size_full()
                .when(cfg!(agentation_external_inspector), |rail| {
                    #[cfg(agentation_external_inspector)]
                    // The shared window surface supplies the two outer border pixels.
                    let rail = rail.w(self.state.rail_width - px(2.)).flex_none();
                    rail
                })
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .py_2()
                .px_1()
                .text_color(theme.colors.fg)
                .when(!cfg!(agentation_external_inspector), |rail| {
                    rail.rounded_xl()
                        .border_1()
                        .border_color(theme.colors.border)
                        .bg(theme.colors.surface)
                        .shadow(theme.elevation(Elevation::Raised))
                })
                .child(tools)
                .child(
                    div()
                        .debug_selector(|| "feedback-toolbar-mcp".into())
                        .flex_none()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_1()
                        .py_1()
                        .cursor_pointer()
                        .id("feedback-toolbar-mcp")
                        .role(gpui::Role::Button)
                        .aria_label(if self.state.mcp {
                            "MCP Running"
                        } else {
                            "MCP Off"
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.action("mcp", cx)))
                        .child(div().size(theme.status_dot()).rounded_full().bg(
                            if self.state.mcp {
                                theme.colors.success
                            } else {
                                theme.colors.fg_muted
                            },
                        ))
                        .child(
                            gpui::svg()
                                .path("agentation/mcp.svg")
                                .size(gpui::rems(1.0))
                                .text_color(theme.colors.fg),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .px_1()
                        .py_1()
                        .debug_selector(|| "feedback-toolbar-settings-divider".into())
                        .child(Divider::horizontal()),
                )
                .child(
                    div()
                        .debug_selector(|| "feedback-floating-comments".into())
                        .child(comments),
                )
                .child(
                    div()
                        .debug_selector(|| "feedback-settings-trigger-target".into())
                        .child(self.button(
                            "feedback-settings-trigger",
                            "Settings",
                            IconName::Settings,
                            "settings",
                            false,
                            cx,
                        )),
                )
                .child(
                    div()
                        .debug_selector(|| "feedback-toolbar-close".into())
                        .child(self.button(
                            "feedback-toolbar-close",
                            "Hide toolbar",
                            IconName::X,
                            "close-toolbar",
                            false,
                            cx,
                        )),
                );
        let body = div().flex().size_full().child(rail);
        #[cfg(agentation_external_inspector)]
        let contents = body
            .debug_selector(|| "feedback-companion-contents".into())
            .opacity(self.width.opacity)
            .children(self.width.inspector.clone().map(|inspector| {
                div()
                    .debug_selector(|| "external-inspector-panel".into())
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .relative()
                    .child(div().w(px(420.)).min_w(px(420.)).h_full().child(inspector))
                    .when(self.state.inspector.is_none(), |panel| {
                        // Retained exit content is visual only; the native picker has stopped.
                        panel.child(
                            div()
                                .absolute()
                                .inset_0()
                                .occlude()
                                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .on_scroll_wheel(|_, _, cx| cx.stop_propagation()),
                        )
                    })
            }));
        #[cfg(agentation_external_inspector)]
        let body = div()
            .debug_selector(|| "feedback-companion-window-surface".into())
            .size_full()
            .relative()
            .overflow_hidden()
            .rounded_xl()
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .shadow(theme.elevation(Elevation::Raised))
            .child(contents)
            .when(self.width.opacity < 1., |surface| {
                surface.child(
                    div()
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation()),
                )
            });
        div()
            .size_full()
            .key_context(if cfg!(agentation_external_inspector) && inspecting {
                "AgentationMode"
            } else {
                "AgentationMode AgentationTools"
            })
            .capture_action(
                cx.listener(|this, action: &commands::AnnotationCommand, _, cx| {
                    this.action(action.name.clone(), cx);
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.action("stop", cx);
                }
            }))
            .child(FocusScope::new(&self.focus).root().size_full().child(body))
    }
}

#[cfg(test)]
#[path = "toolbar_window.test.rs"]
mod tests;
