//! Skip Kit's invisible overlay scaffolding using GPUI's public pick-depth controls.
//!
//! These call sites are pinned to registry Kit 0.11.0. Only the containers are
//! skipped: notification content, dialogs, sheets and their controls still pick
//! normally. No host events are dispatched while the inspector is inactive.
use gpui::{
    App, Context, EntityId, Global, Inspector, InspectorElementId, MouseDownEvent, Pixels,
    PlatformInput, Point, ScrollDelta, ScrollEvent, Window, point, px,
};
use std::collections::HashMap;

#[derive(Default)]
struct PickState {
    position: Option<Point<Pixels>>,
    picking: bool,
    previous: Option<InspectorElementId>,
    pending: bool,
    exhausted: Option<InspectorElementId>,
    finish_selection: bool,
}

#[derive(Default)]
struct Picks {
    states: HashMap<EntityId, PickState>,
    windows: HashMap<gpui::WindowId, EntityId>,
}
impl Global for Picks {}

fn scaffold(id: &InspectorElementId) -> bool {
    let source = id.path.source_location;
    let file = source.file().replace('\\', "/");
    (file.ends_with("gpui-base-0.11.0/src/root.rs") && source.line() == 294)
        || (file.ends_with("gpui-component-0.11.0/src/root.rs")
            && matches!(source.line(), 196 | 203 | 769 | 793))
        || (file.ends_with("gpui-component-0.11.0/src/notification.rs") && source.line() == 1085)
}

/// Return true while a transient scaffold selection must not reach annotations.
pub(crate) fn correct(inspector: &Inspector, window: &Window, cx: &mut Context<Inspector>) -> bool {
    if !cx.has_global::<Picks>() {
        cx.set_global(Picks::default());
    }
    let entity_id = cx.entity_id();
    let window_id = window.window_handle().window_id();
    cx.global_mut::<Picks>()
        .windows
        .insert(window_id, entity_id);
    if !cx.global::<Picks>().states.contains_key(&entity_id) {
        cx.global_mut::<Picks>()
            .states
            .insert(entity_id, PickState::default());
        cx.on_release(move |_, cx| {
            let picks = cx.global_mut::<Picks>();
            picks.states.remove(&entity_id);
            if picks.windows.get(&window_id) == Some(&entity_id) {
                picks.windows.remove(&window_id);
            }
        })
        .detach();
    }
    let position = window.mouse_position();
    let active = inspector.active_element_id().cloned();
    let hidden = active.as_ref().is_some_and(scaffold);
    let state = cx.global_mut::<Picks>().states.get_mut(&entity_id).unwrap();
    let moved = state.position.is_some_and(|p| p != position);
    let reset = moved && (inspector.is_picking() || state.picking);
    state.position = Some(position);
    state.picking = inspector.is_picking();
    if moved {
        state.exhausted = None;
        state.finish_selection = false;
    }
    if state.pending {
        return true;
    }
    if !hidden && !reset && !state.finish_selection {
        state.previous = active;
        state.exhausted = None;
        return false;
    }
    let Some(expected) = active else { return false };
    if state.exhausted.as_ref() == Some(&expected) {
        return hidden;
    }
    let previous = (!reset).then(|| state.previous.clone()).flatten();
    state.pending = true;
    let owner = cx.entity().downgrade();
    let target = window.window_handle();
    cx.defer(move |cx| {
        if let Some(state) = cx.global_mut::<Picks>().states.get_mut(&entity_id) {
            state.pending = false;
        }
        let Some(owner) = owner.upgrade() else { return };
        let _ = target.update(cx, |_, window, cx| {
            if cx.global::<Picks>().windows.get(&window_id) != Some(&entity_id)
                || window.mouse_position() != position
                || owner.read(cx).active_element_id() != Some(&expected)
                || window
                    .with_inspector_state::<(), _>(Some(&expected), cx, |_, _| ())
                    .is_none()
            {
                window.refresh();
                return;
            }
            // A click can arrive between rendering and this callback. Preserve
            // that selected state instead of accidentally restarting hover.
            if !owner.read(cx).is_picking() {
                owner.update(cx, |inspector, _| inspector.start_picking());
                cx.global_mut::<Picks>()
                    .states
                    .get_mut(&entity_id)
                    .unwrap()
                    .finish_selection = true;
                // A selected frame has no inspector hitboxes. First draw a
                // picking frame, then traverse and restore the completed pick.
                window.refresh();
                return;
            }
            let selected = cx
                .global::<Picks>()
                .states
                .get(&entity_id)
                .unwrap()
                .finish_selection;
            // A closed/replaced inspector must never send events to the host.
            if !window.is_inspector_picking(cx) {
                return;
            }
            if reset {
                depth(-1_000_000., window, cx);
            }
            for _ in 0..128 {
                if !owner.read(cx).active_element_id().is_some_and(scaffold) {
                    break;
                }
                if !depth(36., window, cx) {
                    return;
                }
            }
            // If advancing returns to the prior visible element, the user's
            // wheel was traversing backwards. Cross the scaffold in that
            // direction instead of trapping the wheel at the same element.
            if !selected
                && previous
                    .as_ref()
                    .is_some_and(|id| owner.read(cx).active_element_id() == Some(id))
            {
                let forward = owner.read(cx).active_element_id().cloned();
                let mut found = false;
                for _ in 0..128 {
                    if !depth(-36., window, cx) {
                        return;
                    }
                    let active = owner.read(cx).active_element_id();
                    if active.is_some_and(|id| !scaffold(id) && Some(id) != previous.as_ref()) {
                        found = true;
                        break;
                    }
                }
                // Above the first real element there may only be scaffolding.
                // Restore that first candidate rather than blanking the picker.
                if !found {
                    if !depth(-1_000_000., window, cx) {
                        return;
                    }
                    for _ in 0..128 {
                        if owner.read(cx).active_element_id() == forward.as_ref() {
                            break;
                        }
                        if !depth(36., window, cx) {
                            return;
                        }
                    }
                }
            }
            let active = owner.read(cx).active_element_id().cloned();
            let hidden = active.as_ref().is_some_and(scaffold);
            if let Some(state) = cx.global_mut::<Picks>().states.get_mut(&entity_id) {
                state.exhausted = hidden.then(|| active.clone()).flatten();
                state.finish_selection = false;
                if !hidden {
                    state.previous = active;
                }
            }
            if selected && !hidden && window.is_inspector_picking(cx) {
                window.dispatch_event(
                    PlatformInput::MouseDown(MouseDownEvent {
                        position,
                        modifiers: window.modifiers(),
                        click_count: 1,
                        ..Default::default()
                    }),
                    cx,
                );
            }
            window.refresh();
        });
    });
    true
}

fn depth(delta: f32, window: &mut Window, cx: &mut App) -> bool {
    if !window.is_inspector_picking(cx) {
        return false;
    }
    window.dispatch_event(
        PlatformInput::Scroll(ScrollEvent {
            position: window.mouse_position(),
            delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
            modifiers: window.modifiers(),
            touch_phase: gpui::TouchPhase::Moved,
            source: Default::default(),
        }),
        cx,
    );
    true
}
