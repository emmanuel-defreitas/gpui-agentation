//! Position inspection marks from the consumer's current layout, never viewport ratios.
use super::*;
use gpui::{
    AnyElement, AvailableSpace, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Style, point, px, size,
};

pub(super) struct AnchorMark {
    pub key: String,
    pub marker: AnyElement,
    pub color: gpui::Hsla,
}

impl IntoElement for AnchorMark {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for AnchorMark {
    type RequestLayoutState = ();
    type PrepaintState = Option<Bounds<Pixels>>;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    position: gpui::Position::Absolute,
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let (_, bounds) = window.inspection_anchor(&self.key)?;
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let at = point(
            (bounds.center().x - side / 2.).max(px(0.)),
            (bounds.center().y - side / 2.).max(px(0.)),
        );
        self.marker.prepaint_as_root(
            at,
            size(
                AvailableSpace::Definite(window.viewport_size().width),
                AvailableSpace::Definite(window.viewport_size().height),
            ),
            window,
            cx,
        );
        Some(bounds)
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        bounds: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(bounds) = bounds {
            window.paint_quad(
                gpui::outline(*bounds, self.color, gpui::BorderStyle::Solid).border_widths(px(2.)),
            );
            self.marker.paint(window, cx);
        }
    }
}
