//! Figma Examples/Menubar (49:1621), implemented with Ely controls.
use super::*;
use crate::ely::{
    forms::{JsonInput, Switch},
    layout::Collapsible,
};
use gpui::{AnyElement, FontWeight, IntoElement, RenderOnce, SharedString};

fn card(height: f32, cx: &App) -> gpui::Div {
    div()
        .w_full()
        .min_h(px(height))
        .flex_none()
        .flex()
        .flex_col()
        .rounded(px(12.))
        .bg(cx.theme().colors.fg.opacity(0.1))
}

fn detail(text: impl Into<SharedString>, cx: &App) -> gpui::Div {
    div()
        .text_size(px(9.5))
        .line_height(px(13.))
        .font_weight(FontWeight::NORMAL)
        .line_clamp(2)
        .text_color(cx.theme().colors.fg.opacity(0.55))
        .child(text.into())
}

fn title(text: &'static str) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .text_size(px(13.))
        .line_height(px(16.))
        .font_weight(FontWeight::MEDIUM)
        .child(text)
}

fn symbol(path: &'static str, width: f32, height: f32, cx: &App) -> AnyElement {
    if cx.theme().is_dark() {
        gpui::img(path)
            .w(px(width))
            .h(px(height))
            .flex_none()
            .into_any_element()
    } else {
        // Preserve Figma's original shape/alpha while adapting its white ink
        // to the system's light appearance.
        gpui::svg()
            .path(path)
            .w(px(width))
            .h(px(height))
            .flex_none()
            .text_color(cx.theme().colors.fg.opacity(0.85))
            .into_any_element()
    }
}

fn menu_switch(id: &'static str, on: bool) -> Switch {
    Switch::new(id, on)
        .geometry(size(px(36.), px(24.)), size(px(21.), px(21.)))
        .tint(gpui::rgb(0x0a38ff).into())
        .thumb_color(gpui::rgb(0xffffff).into())
}

/// Ely owns focus, disabled state, and click handling; only presentation changes.
#[derive(IntoElement)]
struct MenuButton {
    button: Button,
    label: SharedString,
    visual: AnyElement,
    width: Option<f32>,
    radius: f32,
    height: f32,
    padding: f32,
}

impl RenderOnce for MenuButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fg = cx.theme().colors.fg;
        let mut element = self
            .button
            .hover_color(fg.opacity(0.10))
            .render(window, cx)
            .into_any_element();
        let root = element
            .downcast_mut::<gpui::Stateful<gpui::Div>>()
            .expect("pinned Ely Button root");
        let root = std::mem::replace(root, div().id("unused-menu-button"));
        root.aria_label(self.label)
            .h(px(self.height))
            .flex_none()
            .gap_0()
            .px(px(self.padding))
            .border_0()
            .rounded(px(self.radius))
            .justify_start()
            .font_weight(FontWeight::MEDIUM)
            .text_size(px(13.))
            .line_height(px(16.))
            .bg(fg.opacity(0.08))
            .when_some(self.width, |root, width| root.w(px(width)))
            .child(self.visual)
    }
}

pub(super) fn render(
    popup: &SettingsPopup,
    owner: &Entity<AnnotationOverlay>,
    cx: &Context<SettingsPopup>,
) -> gpui::Div {
    let overlay = owner.read(cx);
    let reload_owner = owner.downgrade();
    let reload_key_owner = reload_owner.clone();
    let mcp_owner = owner.downgrade();
    let mcp_key_owner = mcp_owner.clone();
    let copy = cx.entity().downgrade();
    let disclosure = copy.clone();
    let enabled = if overlay.mcp_starting {
        overlay.mcp_requested
    } else {
        overlay._server.as_ref().is_some_and(Server::is_mcp_enabled)
    };
    let available = overlay._server.is_some();
    let hot_reload = card(79., cx)
        .pb(px(12.))
        .debug_selector(|| "feedback-settings-hot-reload".into())
        .child(
            div()
                .flex_none()
                .h(px(29.))
                .mx(px(9.))
                .mt(px(7.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(symbol("agentation/menubar/reload.svg", 25., 25., cx))
                .child(title("Hot Reload"))
                .child(
                    div()
                        .debug_selector(|| "feedback-hot-reload-toggle".into())
                        .on_key_down(move |event, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "space" | "enter")
                                && !event.keystroke.modifiers.modified()
                                && !event.is_held
                            {
                                let _ = reload_key_owner.update(cx, |this, cx| {
                                    this.set_hot_reload_enabled(!this.hot_reload_status.enabled, cx)
                                });
                                cx.stop_propagation();
                            }
                        })
                        .child(
                            menu_switch("tray-hot-reload", overlay.hot_reload_status.enabled)
                                .name(if overlay.hot_reload_status.available {
                                    "Hot Reload".to_string()
                                } else {
                                    format!("Hot Reload — {}", overlay.hot_reload_status.detail)
                                })
                                .disabled(
                                    !overlay.hot_reload_status.available
                                        || overlay.hot_reload_pending,
                                )
                                .on_change(move |enabled, _, cx| {
                                    let _ = reload_owner.update(cx, |this, cx| {
                                        this.set_hot_reload_enabled(enabled, cx)
                                    });
                                }),
                        ),
                ),
        )
        .child(
            detail(
                "Rebuilds and restarts this app after a source save. Save app work before enabling",
                cx,
            )
            .mx(px(12.))
            .mt(px(5.)),
        );
    let copy_label = if popup.copied { "Copied!" } else { "Copy" };
    let copy_row = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .w(px(12.))
                .h(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(symbol("agentation/menubar/copy.svg", 11., 13., cx)),
        )
        .child(div().flex_1().child(copy_label))
        .child(detail("⌃⌘C", cx).text_size(px(13.)).line_height(px(16.)));
    let copy_button = MenuButton {
        button: Button::new("feedback-mcp-copy-config", "")
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .full_width()
            .disabled(!available)
            .on_click(move |_, _, cx| {
                let _ = copy.update(cx, |this, cx| this.copy(cx));
            }),
        label: copy_label.into(),
        visual: copy_row.into_any_element(),
        width: None,
        radius: 5.,
        height: 25.,
        padding: 7.,
    };
    let mcp = card(116., cx).pb(px(10.))
        .debug_selector(|| "feedback-settings-mcp".into())
        .child(div().flex_none().h(px(31.)).mx(px(9.)).mt(px(10.))
            .flex().items_center().gap(px(6.))
            .child(div().debug_selector(|| "feedback-mcp-config-toggle".into())
                .child(MenuButton {
                    button: Button::new("tray-config-disclosure", "").variant(ButtonVariant::Ghost).size(ControlSize::Sm)
                        .on_click(move |_, _, cx| { let _ = disclosure.update(cx, |this, cx| { this.expanded = !this.expanded; cx.notify(); }); }),
                    label: if popup.expanded { "Hide MCP configuration" } else { "Show MCP configuration" }.into(),
                    visual: div().w_full().h_full().flex().items_center().justify_center()
                        .child(symbol("agentation/menubar/mcp.svg", 15., 15., cx)).into_any_element(),
                    width: Some(25.), radius: 13.,
                    height: 25., padding: 0.,
                }))
            .child(title("MCP Server"))
            .child(detail(if overlay.mcp_starting { "Starting…" } else if enabled { "6 tools" } else { "Off" }, cx).text_size(px(11.)).line_height(px(16.)))
            .child(div().debug_selector(|| "feedback-mcp-server-toggle".into())
                .on_key_down(move |event, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "space" | "enter")
                        && !event.keystroke.modifiers.modified() && !event.is_held {
                        let _ = mcp_key_owner.update(cx, |this, cx| this.toggle_mcp(cx)); cx.stop_propagation();
                    }
                })
                .child(menu_switch("tray-mcp", enabled).name("MCP Server").disabled(overlay.mcp_starting)
                    .on_change(move |enabled, _, cx| { let _ = mcp_owner.update(cx, |this, cx| this.set_mcp_enabled(enabled, cx)); }))))
        .child(detail("Copy the MCP streamable HTTP configuration below and paste it into your IDE or AI provider.", cx).mx(px(9.)).mt(px(4.)))
        .child(div().debug_selector(|| "feedback-mcp-copy-config-target".into()).mx(px(11.)).mt(px(10.)).child(copy_button))
        .when(popup.expanded, |card| card.child(Collapsible::new("tray-config", true).child(
            div().id("tray-config-scroll").debug_selector(|| "feedback-mcp-config-code".into())
                .mx(px(9.)).mt(px(8.)).max_h(px(160.)).overflow_y_scroll().child(JsonInput::new(&popup.config)))))
        .when(!overlay.settings_status.is_empty(), |card| card.child(detail(overlay.settings_status.clone(), cx).mx(px(9.)).mt(px(5.))));
    let crates = MenuButton {
        button: Button::new("feedback-crates", "")
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(|_, _, cx| cx.open_url("https://crates.io/crates/gpui-agentation")),
        label: "crates.io".into(),
        width: Some(94.),
        radius: 8.,
        height: 24.,
        padding: 7.,
        visual: div()
            .flex()
            .items_center()
            .gap(px(5.))
            .child(gpui::img("agentation/menubar/crates.png").size(px(16.)))
            .child("crates.io")
            .into_any_element(),
    };
    let github = MenuButton {
        button: Button::new("feedback-github", "")
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(|_, _, cx| {
                cx.open_url("https://github.com/emmanuel-defreitas/gpui-agentation")
            }),
        label: "github.com".into(),
        width: Some(107.),
        radius: 8.,
        height: 24.,
        padding: 7.,
        visual: div()
            .flex()
            .items_center()
            .gap(px(5.))
            .child(symbol("agentation/menubar/github.svg", 16., 15., cx))
            .child("github.com")
            .into_any_element(),
    };
    let about = card(98., cx)
        .px(px(14.))
        .pt(px(10.))
        .pb(px(10.))
        .debug_selector(|| "feedback-settings-about".into())
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .child("About"),
        )
        .child(
            detail(
                format!("GPUI Agentation - version {}", env!("CARGO_PKG_VERSION")),
                cx,
            )
            .line_clamp(1)
            .mt(px(2.)),
        )
        .child(
            detail("Native annotation devtools for GPUI projects.", cx)
                .line_clamp(1)
                .mt(px(3.)),
        )
        .child(
            div()
                .flex()
                .gap(px(6.))
                .mt(px(7.))
                .child(crates)
                .child(github),
        );
    div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(11.))
        .gap(px(4.))
        .font_family(".SystemUIFont")
        .text_size(px(13.))
        .line_height(px(16.))
        .text_color(cx.theme().colors.fg.opacity(0.85))
        .child(
            div()
                .flex_none()
                .h(px(22.))
                .flex()
                .items_center()
                .gap(px(4.))
                .debug_selector(|| "feedback-settings-brand".into())
                .child(
                    gpui::img("agentation/menubar/brand.svg")
                        .w(px(19.))
                        .h(px(21.6553)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .child("GPUI Agentation"),
                )
                .child(
                    detail(format!("v{}", env!("CARGO_PKG_VERSION")), cx)
                        .text_color(cx.theme().colors.fg.opacity(0.18)),
                ),
        )
        .child(
            div()
                .id("feedback-settings-scroll")
                .debug_selector(|| "feedback-settings-scroll".into())
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(hot_reload)
                        .child(mcp)
                        .child(about),
                ),
        )
}
