//! Pins attached to a registered target move with it, e.g. when its container scrolls.
use std::{cell::Cell, rc::Rc};

use super::*;
use gpui::{
    AnyElement, AvailableSpace, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Style, point, px, size,
};

/// Where a target sat when its pin was first laid out; `None` until then.
pub(super) type Baseline = Rc<Cell<Option<(f32, f32)>>>;

pub(super) struct FollowMark {
    pub target: String,
    pub baseline: Baseline,
    /// Saved pin center in window pixels.
    pub center: (f32, f32),
    pub marker: AnyElement,
}

impl IntoElement for FollowMark {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for FollowMark {
    type RequestLayoutState = ();
    type PrepaintState = bool;
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
    ) -> bool {
        let Some(origin) = target::origin(window, cx, &self.target) else {
            return false;
        };
        let base = self.baseline.get().unwrap_or_else(|| {
            self.baseline.set(Some(origin));
            origin
        });
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let at = point(
            (px(self.center.0 + origin.0 - base.0) - side / 2.).max(px(0.)),
            (px(self.center.1 + origin.1 - base.1) - side / 2.).max(px(0.)),
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
        true
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        visible: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *visible {
            self.marker.paint(window, cx);
        }
    }
}
