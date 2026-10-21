use super::*;
use crate::ely::primitives::{FocusRing, IconName};
use gpui::RenderOnce;

// Ely's pinned Button has no tone/style slot; keep the same GPUI button
// semantics and Ely geometry while supplying the owning surface's colors.
#[derive(IntoElement)]
pub(super) struct SurfaceButton {
    id: gpui::ElementId,
    label: gpui::SharedString,
    icon: IconName,
    selected: bool,
    colors: Option<(gpui::Hsla, gpui::Hsla)>,
    disabled: bool,
    icon_only: bool,
    size: ControlSize,
    handler: Option<Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>>,
}
impl SurfaceButton {
    pub(super) fn new(
        id: impl Into<gpui::ElementId>,
        label: impl Into<gpui::SharedString>,
        icon: IconName,
        selected: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            selected,
            colors: None,
            disabled: false,
            icon_only: false,
            size: ControlSize::Sm,
            handler: None,
        }
    }
    pub(super) fn colors(mut self, background: gpui::Hsla, foreground: gpui::Hsla) -> Self {
        self.colors = Some((background, foreground));
        self
    }
    pub(super) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub(super) fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }
    pub(super) fn on_click(
        mut self,
        handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Box::new(handler));
        self
    }
}
impl RenderOnce for SurfaceButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use crate::ely::theme::{Mix, Radius, TextSize};
        let focus = window
            .use_keyed_state((self.id.clone(), "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone()
            .tab_stop(!self.disabled);
        let theme = cx.theme();
        let (background, foreground) = self.colors.unwrap_or({
            if self.selected {
                (theme.colors.bg, theme.colors.fg)
            } else {
                (theme.colors.fg, theme.colors.bg)
            }
        });
        let (hover, pressed) = if self.colors.is_some() {
            // On the amber surface, increase contrast instead of washing out
            // its already-contrasting foreground when hovered or pressed.
            (
                background.mix(&theme.colors.fg, 0.08),
                background.mix(&theme.colors.fg, 0.14),
            )
        } else {
            (
                background.mix(&foreground, 0.14),
                background.mix(&foreground, 0.22),
            )
        };
        div()
            .id(self.id)
            .role(gpui::Role::Button)
            .aria_label(self.label.clone())
            .flex()
            .items_center()
            .justify_center()
            .gap_1p5()
            .flex_none()
            .h(theme.control_height(self.size))
            .px_2()
            .when(self.icon_only, |root| {
                root.w(theme.control_height(self.size)).px_0()
            })
            .border_1()
            .border_color(gpui::transparent_black())
            .rounded(theme.radius(Radius::Md))
            .text_size(theme.text_size(TextSize::Sm))
            .font_weight(gpui::FontWeight::MEDIUM)
            .bg(background)
            .text_color(foreground)
            .child(Icon::new(self.icon).size(IconSize::Sm).color(foreground))
            .when(!self.icon_only, |root| root.child(self.label))
            .when(self.disabled, |root| {
                root.opacity(0.45)
                    .cursor_not_allowed()
                    .aria_description("Unavailable")
            })
            .when(!self.disabled, |root| {
                root.track_focus(&focus)
                    .focus_ring(cx)
                    .hover(move |style| style.bg(hover))
                    .active(move |style| style.bg(pressed))
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                        window.prevent_default()
                    })
                    .when_some(self.handler, |root, handler| root.on_click(handler))
            })
    }
}
