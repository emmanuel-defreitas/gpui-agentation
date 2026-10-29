use std::{
    cell::RefCell,
    collections::HashMap,
    panic::Location,
    rc::{Rc, Weak},
};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, WindowId,
};

use crate::Rect;

#[derive(Clone, Debug)]
pub(crate) struct Target {
    pub id: String,
    pub label: String,
    pub source: String,
    pub bounds: Rect,
}

#[derive(Default)]
struct Registry(HashMap<WindowId, Vec<Weak<RefCell<Target>>>>);
impl Global for Registry {}

/// Prune removed targets before rendering. Cached element registrations remain alive.
pub fn begin_frame(window: &Window, cx: &mut App) {
    if !cx.has_global::<Registry>() {
        cx.set_global(Registry::default());
    }
    cx.global_mut::<Registry>()
        .0
        .entry(window.window_handle().window_id())
        .or_default()
        .retain(|target| target.strong_count() > 0);
}

/// Register a stable ID, label, and Rust call site without adding a layout box.
#[track_caller]
pub fn target(
    id: impl Into<String>,
    label: impl Into<String>,
    child: impl IntoElement,
) -> impl IntoElement {
    let source = Location::caller();
    Registered {
        child: child.into_any_element(),
        target: Target {
            id: id.into(),
            label: label.into(),
            source: format!("{}:{}:{}", source.file(), source.line(), source.column()),
            bounds: Rect::default(),
        },
    }
}

pub(crate) fn pick(window: &Window, cx: &App, x: f32, y: f32) -> Option<Target> {
    targets(window, cx)
        .into_iter()
        .rev()
        .filter(|t| t.bounds.contains(x, y))
        .min_by(|a, b| {
            (a.bounds.width * a.bounds.height).total_cmp(&(b.bounds.width * b.bounds.height))
        })
}

pub(crate) fn targets(window: &Window, cx: &App) -> Vec<Target> {
    cx.try_global::<Registry>()
        .and_then(|r| r.0.get(&window.window_handle().window_id()))
        .map(|targets| {
            targets
                .iter()
                .filter_map(Weak::upgrade)
                .map(|t| t.borrow().clone())
                .filter(|t| t.bounds.width > 0.0 && t.bounds.height > 0.0)
                .collect()
        })
        .unwrap_or_default()
}

struct Registered {
    child: AnyElement,
    target: Target,
}
impl IntoElement for Registered {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Registered {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(format!("agentation-target-{}", self.target.id).into())
    }
    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let clip = window.content_mask().bounds;
        let x = f32::from(bounds.origin.x.max(clip.origin.x));
        let y = f32::from(bounds.origin.y.max(clip.origin.y));
        let right =
            f32::from((bounds.origin.x + bounds.size.width).min(clip.origin.x + clip.size.width));
        let bottom =
            f32::from((bounds.origin.y + bounds.size.height).min(clip.origin.y + clip.size.height));
        self.target.bounds = Rect {
            x,
            y,
            width: (right - x).max(0.0),
            height: (bottom - y).max(0.0),
        };
        if let Some(global_id) = global_id {
            if !cx.has_global::<Registry>() {
                cx.set_global(Registry::default());
            }
            let measured = self.target.clone();
            let (state, new, moved) =
                window.with_element_state(global_id, |state: Option<Rc<RefCell<Target>>>, _| {
                    let new = state.is_none();
                    let state = state.unwrap_or_else(|| Rc::new(RefCell::new(measured.clone())));
                    let moved = state.borrow().bounds != measured.bounds;
                    *state.borrow_mut() = measured;
                    ((state.clone(), new, moved), state)
                });
            if new {
                cx.global_mut::<Registry>()
                    .0
                    .entry(window.window_handle().window_id())
                    .or_default()
                    .push(Rc::downgrade(&state));
            }
            if moved {
                window.request_animation_frame();
            }
        }
        self.child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[cfg(all(test, feature = "test-support"))]
#[path = "tests/target.rs"]
mod tests;
