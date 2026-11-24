//! Agentation-owned inspection state, routing and lifecycle.
//! Presentation adapters consume GPUI primitives and owned data only.
pub(crate) mod kit;
pub(crate) mod source;
pub(crate) mod style;

use gpui::{
    AnyWindowHandle, App, Bounds, DivInspectorState, Entity, EntityId, Global, InspectorElementId,
    Pixels, Subscription, WeakEntity, Window, WindowId,
};
use std::collections::{BTreeMap, HashMap};
use style::{ColorTarget, Side, StyleEditState, describe_color, layers, length_pixels, number};

// Compatibility exports: the public previous presentation tooling names remain available until #120.
pub use crate::{
    GpuiConfiguration as InspectorStyleConfiguration, GpuiStyleChange as InspectorStyleChange,
};

/// Snapshot of the element displayed in a host window's inspector.
#[derive(Clone)]
pub struct InspectorSelection {
    /// GPUI's identity for the selected element.
    pub id: InspectorElementId,
    /// Logical bounds in the host window.
    pub bounds: Bounds<Pixels>,
    /// Stable property names and formatted values exported in annotations.
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RendererOwner {
    Default,
    Agentation,
}

struct Route {
    target: AnyWindowHandle,
    owner: Option<EntityId>,
}

#[derive(Default)]
struct State {
    selections: HashMap<WindowId, InspectorSelection>,
    configurations: HashMap<WindowId, InspectorStyleConfiguration>,
    targets: HashMap<WindowId, Route>,
    sessions: HashMap<WindowId, WeakEntity<StyleEditState>>,
    renderer: Option<RendererOwner>,
    element_registered: bool,
    _closed: Option<Subscription>,
}
impl Global for State {}

pub(crate) fn initialize(cx: &mut App) {
    if cx.has_global::<State>() {
        return;
    }
    let closed = cx.on_window_closed(|cx, id| forget_window(id, cx));
    cx.set_global(State {
        _closed: Some(closed),
        ..State::default()
    });
}

/// Default presentation may initialize once; Agentation explicitly takes over
/// once during native init. Subsequent default installs cannot replace it.
pub(crate) fn claim_renderer(owner: RendererOwner, cx: &mut App) -> bool {
    initialize(cx);
    let current = &mut cx.global_mut::<State>().renderer;
    if current.is_none()
        || (*current == Some(RendererOwner::Default) && owner == RendererOwner::Agentation)
    {
        *current = Some(owner);
        true
    } else {
        false
    }
}

pub(crate) fn owns_renderer(cx: &App) -> bool {
    cx.try_global::<State>()
        .is_some_and(|state| state.renderer == Some(RendererOwner::Agentation))
}

pub(crate) fn register_element_renderer(cx: &mut App) -> bool {
    initialize(cx);
    !std::mem::replace(&mut cx.global_mut::<State>().element_registered, true)
}

pub(crate) fn register_target(
    view: WindowId,
    target: AnyWindowHandle,
    owner: EntityId,
    cx: &mut App,
) {
    initialize(cx);
    cx.global_mut::<State>().targets.insert(
        view,
        Route {
            target,
            owner: Some(owner),
        },
    );
}

pub(crate) fn unregister_target(view: WindowId, owner: EntityId, cx: &mut App) {
    if cx.has_global::<State>() {
        let state = cx.global_mut::<State>();
        if state
            .targets
            .get(&view)
            .is_some_and(|route| route.owner == Some(owner))
        {
            state.targets.remove(&view);
        }
    }
}

/// Return the host for a detached presentation, or the current window.
pub fn inspector_target(window: &Window, cx: &App) -> AnyWindowHandle {
    cx.try_global::<State>()
        .and_then(|state| state.targets.get(&window.window_handle().window_id()))
        .map_or(window.window_handle(), |route| route.target)
}

/// Read the selected host's current style configuration.
pub fn active_inspector_configuration(
    window: &Window,
    cx: &App,
) -> Option<InspectorStyleConfiguration> {
    cx.try_global::<State>()?
        .configurations
        .get(&window.window_handle().window_id())
        .cloned()
}

/// Read the current host selection and its property snapshot.
pub fn inspector_selection(window: &Window, cx: &App) -> Option<InspectorSelection> {
    cx.try_global::<State>()?
        .selections
        .get(&window.window_handle().window_id())
        .cloned()
}

/// Clear a host selection and exported configuration without changing live styles.
pub fn clear_inspector_selection(window: &Window, cx: &mut App) {
    if cx.has_global::<State>() {
        let state = cx.global_mut::<State>();
        let id = window.window_handle().window_id();
        state.selections.remove(&id);
        state.configurations.remove(&id);
    }
}

/// Idempotent cleanup, also valid after the native window has disappeared.
pub(crate) fn forget_window(id: WindowId, cx: &mut App) {
    let session = if cx.has_global::<State>() {
        let state = cx.global_mut::<State>();
        state.selections.remove(&id);
        state.configurations.remove(&id);
        state
            .targets
            .retain(|view, route| *view != id && route.target.window_id() != id);
        state.sessions.remove(&id)
    } else {
        None
    };
    if let Some(session) = session {
        let _ = session.update(cx, |state, _| *state = StyleEditState::default());
    }
}

pub(crate) fn capture(
    id: InspectorElementId,
    state: &DivInspectorState,
    edits: &Entity<StyleEditState>,
    window: &Window,
    cx: &mut App,
) -> bool {
    initialize(cx);
    let target_id = inspector_target(window, cx).window_id();
    let changed = edits.update(cx, |edits, _| {
        edits.select(id.clone(), &state.base_style, state.bounds.size)
    });
    if changed {
        let configuration = cx.global_mut::<State>();
        configuration.selections.remove(&target_id);
        configuration.configurations.remove(&target_id);
    }
    cx.global_mut::<State>().selections.insert(
        target_id,
        InspectorSelection {
            id: id.clone(),
            bounds: state.bounds,
            properties: {
                let rem = window.rem_size();
                let written = layers(&state.base_style, rem);
                let sides = |values: [Side; 4]| values.map(|value| value.to_string()).join(" / ");
                std::collections::BTreeMap::from([
                    (
                        "Width".into(),
                        format!(
                            "{} px",
                            number(length_pixels(
                                state.base_style.size.width,
                                state.bounds.size.width,
                                rem
                            ))
                        ),
                    ),
                    (
                        "Height".into(),
                        format!(
                            "{} px",
                            number(length_pixels(
                                state.base_style.size.height,
                                state.bounds.size.height,
                                rem
                            ))
                        ),
                    ),
                    ("Padding".into(), sides(written.padding)),
                    ("Margin".into(), sides(written.margin)),
                    ("Border".into(), sides(written.border)),
                    (
                        "Opacity".into(),
                        number(state.base_style.opacity.unwrap_or(1.)),
                    ),
                    (
                        "Background".into(),
                        describe_color(ColorTarget::Background.read(&state.base_style)),
                    ),
                    (
                        "Text color".into(),
                        describe_color(ColorTarget::Text.read(&state.base_style)),
                    ),
                ])
            },
        },
    );
    cx.global_mut::<State>()
        .sessions
        .insert(target_id, edits.downgrade());
    changed
}

pub(crate) mod picking;
#[cfg(test)]
mod tests;
