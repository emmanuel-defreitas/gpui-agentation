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

struct Geometry {
    origin: (f32, f32),
    clip: Rect,
    screen: bool,
}

/// Unclipped top-left and scroll viewport per target.
#[derive(Default)]
struct Origins(HashMap<(WindowId, String), Geometry>);
impl Global for Origins {}

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
    registered(id, label, child, false)
}

#[track_caller]
pub(crate) fn screen_target(
    id: impl Into<String>,
    label: impl Into<String>,
    child: impl IntoElement,
) -> impl IntoElement {
    registered(id, label, child, true)
}

#[track_caller]
fn registered(
    id: impl Into<String>,
    label: impl Into<String>,
    child: impl IntoElement,
    screen: bool,
) -> Registered {
    let source = Location::caller();
    Registered {
        screen,
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

/// The registered target nearest to a point: the one under it when several
/// overlap the smallest, otherwise the closest by edge distance. Content in one
/// scroll container moves together, so a gap between cards is anchored to a
/// neighbouring card.
pub(crate) fn nearest(window: &Window, cx: &App, x: f32, y: f32) -> Option<Target> {
    let distance = |t: &Target| {
        let dx = (t.bounds.x - x)
            .max(x - (t.bounds.x + t.bounds.width))
            .max(0.0);
        let dy = (t.bounds.y - y)
            .max(y - (t.bounds.y + t.bounds.height))
            .max(0.0);
        dx.hypot(dy)
    };
    let mut candidates = targets(window, cx);
    let is_screen = |target: &Target| {
        cx.try_global::<Origins>().is_some_and(|origins| {
            origins
                .0
                .get(&(window.window_handle().window_id(), target.id.clone()))
                .is_some_and(|geometry| geometry.screen)
        })
    };
    // The full-screen registration is a fallback for picking, not the content
    // beneath a drawing in the gap between scrolling cards.
    candidates.retain(|target| !is_screen(target));
    candidates.into_iter().min_by(|a, b| {
        distance(a).total_cmp(&distance(b)).then_with(|| {
            (a.bounds.width * a.bounds.height).total_cmp(&(b.bounds.width * b.bounds.height))
        })
    })
}

/// Current unclipped origin of a live, visible target.
pub(crate) fn origin(window: &Window, cx: &App, id: &str) -> Option<(f32, f32)> {
    let window_id = window.window_handle().window_id();
    targets(window, cx).iter().find(|t| t.id == id)?;
    Some(
        cx.try_global::<Origins>()?
            .0
            .get(&(window_id, id.to_owned()))?
            .origin,
    )
}

pub(crate) fn clip(window: &Window, cx: &App, id: &str) -> Option<Rect> {
    Some(
        cx.try_global::<Origins>()?
            .0
            .get(&(window.window_handle().window_id(), id.to_owned()))?
            .clip,
    )
}

pub(crate) fn targets(window: &Window, cx: &App) -> Vec<Target> {
    registered_targets(window, cx)
        .into_iter()
        .filter(|t| t.bounds.width > 0.0 && t.bounds.height > 0.0)
        .collect()
}

/// Includes clipped targets so saved pins retain their ancestor while offscreen.
pub(crate) fn registered_targets(window: &Window, cx: &App) -> Vec<Target> {
    cx.try_global::<Registry>()
        .and_then(|r| r.0.get(&window.window_handle().window_id()))
        .map(|targets| {
            targets
                .iter()
                .filter_map(Weak::upgrade)
                .map(|t| t.borrow().clone())
                .collect()
        })
        .unwrap_or_default()
}

struct Registered {
    child: AnyElement,
    target: Target,
    screen: bool,
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
        if !cx.has_global::<Origins>() {
            cx.set_global(Origins::default());
        }
        let origin = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        let previous = cx.global_mut::<Origins>().0.insert(
            (window.window_handle().window_id(), self.target.id.clone()),
            Geometry {
                origin,
                screen: self.screen,
                clip: Rect {
                    x: clip.origin.x.into(),
                    y: clip.origin.y.into(),
                    width: clip.size.width.into(),
                    height: clip.size.height.into(),
                },
            },
        );
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
            if moved || previous.is_none_or(|previous| previous.origin != origin) {
                // Drawing and editor positions are computed during render,
                // before this frame's target geometry is measured. Refresh all
                // views next frame, including when a large target's clip stays
                // unchanged while its unclipped origin moves.
                window.on_next_frame(|window, _| window.refresh());
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
