//! Kit Root companion, using the existing AppKit geometry bridge.
use super::*;
use gpui::{AnyWindowHandle, Global, Render, WeakEntity, WindowBounds, WindowId, WindowOptions};
use std::collections::HashMap;

const GAP: f32 = 8.;
// One implementation of native docking/margin ownership during the cutover.
#[path = "../../overlay/toolbar_window/mac.rs"]
#[allow(deprecated, unexpected_cfgs)]
mod native;

#[derive(Default)]
struct Sessions {
    hosts: HashMap<WindowId, Session>,
    inspectors: HashMap<WindowId, WeakEntity<Inspector>>,
    _closed: Option<Subscription>,
}
impl Global for Sessions {}
struct Session {
    panel: AnyWindowHandle,
    content: WeakEntity<Companion>,
    margin: Option<native::HiddenMargin>,
    _resize: Subscription,
}
struct Companion {
    inspector: Option<Entity<Inspector>>,
    selection: Option<(Option<InspectorElementId>, bool)>,
    rem: Pixels,
}
impl Render for Companion {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(self.rem);
        div().size_full().children(self.inspector.clone())
    }
}

/// Open an inspector companion when AppKit can preserve host layout.
/// Returns false for full-screen/fill or unsupported windows; the caller can
/// use GPUI's ordinary in-host inspector in that case.
pub fn open(host: &mut Window, cx: &mut App) -> bool {
    if native::fills_screen(host) || host.is_fullscreen() {
        return false;
    }
    initialize(cx);
    let host_id = host.window_handle().window_id();
    if cx.global::<Sessions>().hosts.contains_key(&host_id) {
        return true;
    }
    // Verified against Window::draw_roots in the selected GPUI 0.3.14 source.
    let width = gpui::rems(30.).to_pixels(host.rem_size());
    let Some(margin) = native::HiddenMargin::new(host, width, cx) else {
        return false;
    };
    let host_handle = host.window_handle();
    let rem = host.rem_size();
    let result = gpui::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.), px(0.)),
                size(width, host.viewport_size().height),
            ))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Agentation · Inspector".into()),
                ..Default::default()
            }),
            focus: false,
            ..Default::default()
        },
        cx,
        |panel, cx| {
            cx.new(|cx| {
                service::register_target(
                    panel.window_handle().window_id(),
                    host_handle,
                    cx.entity_id(),
                    cx,
                );
                Companion {
                    inspector: None,
                    selection: None,
                    rem,
                }
            })
        },
    );
    let Ok((panel, content)) = result else {
        margin.remove(cx);
        return false;
    };
    let resize = content.update(cx, |_, cx| {
        cx.observe_window_bounds(host, move |_, host, cx| {
            let _ = panel.update(cx, |_, panel, cx| {
                native::place(host, panel, width, true, cx);
            });
        })
    });
    let _ = panel.update(cx, |_, panel, cx| {
        native::place(host, panel, width, true, cx);
    });
    cx.global_mut::<Sessions>().hosts.insert(
        host_id,
        Session {
            panel,
            content: content.downgrade(),
            margin: Some(margin),
            _resize: resize,
        },
    );
    if !is_open(host, cx) {
        host.toggle_inspector(cx);
    }
    host.refresh();
    true
}

fn close_for_window(id: WindowId, cx: &mut App) {
    cx.global_mut::<Sessions>().inspectors.remove(&id);
    let key = cx
        .global::<Sessions>()
        .hosts
        .iter()
        .find(|(host, session)| **host == id || session.panel.window_id() == id)
        .map(|(host, _)| *host);
    let Some(key) = key else {
        return;
    };
    let session = cx.global_mut::<Sessions>().hosts.remove(&key).unwrap();
    service::forget_window(session.panel.window_id(), cx);
    service::forget_window(key, cx);
    if let Some(margin) = session.margin {
        margin.remove(cx);
    }
    if id == key {
        let _ = session
            .panel
            .update(cx, |_, panel, _| panel.remove_window());
    } else {
        // A close callback can run during another window update.
        cx.defer(move |cx| {
            if cx.global::<Sessions>().hosts.contains_key(&key) {
                return;
            }
            if let Some(host) = cx.windows().into_iter().find(|w| w.window_id() == key) {
                let _ = host.update(cx, |_, host, cx| {
                    if is_open(host, cx) {
                        host.toggle_inspector(cx);
                    }
                });
            }
        });
    }
}

/// Called by the shared renderer. Hidden host strips update the companion's
/// Inspector entity; the companion renders the same entity through Kit controls.
pub(super) fn render(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
) -> Option<AnyElement> {
    let target = inspector_target(window, cx);
    if target != window.window_handle() {
        return None;
    }
    let entity = cx.entity();
    cx.global_mut::<Sessions>()
        .inspectors
        .insert(target.window_id(), entity.downgrade());
    let content = cx
        .try_global::<Sessions>()?
        .hosts
        .get(&target.window_id())?
        .content
        .clone();
    let rem = window.rem_size();
    let selection = (
        inspector.active_element_id().cloned(),
        inspector.is_picking(),
    );
    let _ = content.update(cx, |content, cx| {
        if content.selection.as_ref() != Some(&selection)
            || content.inspector.as_ref().map(Entity::entity_id) != Some(entity.entity_id())
            || content.rem != rem
        {
            content.inspector = Some(entity);
            content.rem = rem;
            content.selection = Some(selection);
            cx.notify();
        }
    });
    if inspector.is_picking() {
        window.request_animation_frame();
    }
    Some(div().into_any_element())
}

/// Close the companion through its normal window lifecycle callback.
pub(super) fn stop(host: WindowId, cx: &mut App) {
    let panel = cx
        .try_global::<Sessions>()
        .and_then(|s| s.hosts.get(&host))
        .map(|s| s.panel);
    if let Some(panel) = panel {
        cx.defer(move |cx| {
            let _ = panel.update(cx, |_, panel, _| panel.remove_window());
        });
    }
}

pub(super) fn initialize(cx: &mut App) {
    if !cx.has_global::<Sessions>() {
        let closed = cx.on_window_closed(|cx, id| close_for_window(id, cx));
        cx.set_global(Sessions {
            _closed: Some(closed),
            ..Default::default()
        });
    }
}

// GPUI exposes picking and selected-state access, but no public enabled getter.
// A retained old Inspector in a companion is not evidence of host ownership.
pub(super) fn is_open(host: &mut Window, cx: &mut App) -> bool {
    if host.is_inspector_picking(cx) {
        return true;
    }
    let id = cx
        .try_global::<Sessions>()
        .and_then(|s| s.inspectors.get(&host.window_handle().window_id()))
        .and_then(WeakEntity::upgrade)
        .and_then(|i| i.read(cx).active_element_id().cloned());
    host.with_inspector_state::<DivInspectorState, _>(id.as_ref(), cx, |_, _| ())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn companion_restores_host_rem_after_kit_root_prepares(cx: &mut gpui::TestAppContext) {
        cx.update(gpui::init);
        let (_, visual) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Companion {
                inspector: None,
                selection: None,
                rem: px(24.),
            });
            gpui::base::Root::new(content, window, cx)
        });
        visual.run_until_parked();
        visual.update(|window, _| assert_eq!(window.rem_size(), px(24.)));
    }
}
