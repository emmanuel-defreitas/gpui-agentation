use super::*;
use gpui::RenderOnce;

// Kit button with colors supplied by its owning toolbar or panel.
#[derive(IntoElement)]
pub(super) struct SurfaceButton {
    id: gpui::ElementId,
    label: gpui::SharedString,
    icon: IconName,
    selected: bool,
    colors: Option<(gpui::Hsla, gpui::Hsla)>,
    disabled: bool,
    icon_only: bool,
    shortcut: Option<gpui::SharedString>,
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
            shortcut: None,
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
    pub(super) fn shortcut(mut self, shortcut: impl Into<gpui::SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = cx.theme();
        let (bg, fg) = self.colors.unwrap_or(if self.selected {
            (t.primary, t.primary_foreground)
        } else {
            (t.secondary, t.foreground)
        });
        let mut button = Button::new(self.id)
            .icon(self.icon)
            .small()
            .accessibility_label(self.label.clone())
            .toggled(self.selected)
            .disabled(self.disabled)
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(bg)
                    .foreground(fg)
                    .hover(bg.mix(fg, 0.14))
                    .active(bg.mix(fg, 0.22)),
            );
        if self.icon_only {
            let tooltip = self
                .shortcut
                .map(|key| format!("{} ({key})", self.label))
                .unwrap_or_else(|| self.label.to_string());
            button = button.tooltip(tooltip);
        } else {
            button = button.label(self.label);
        }
        if let Some(handler) = self.handler {
            button = button.on_click(handler);
        }
        button
    }
}
