use super::*;
use gpui::{Modifiers, Render, TestAppContext};
use std::{cell::Cell, rc::Rc};

struct Sample;
impl Render for Sample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(cx.theme().background).child(
            div()
                .id("kit-sample")
                .debug_selector(|| "kit-sample".into())
                .w(px(160.))
                .h(px(60.))
                .p_2()
                .bg(gpui::rgb(0x336699))
                .child("Sample"),
        )
    }
}

#[gpui::test]
fn kit_slider_and_reset_update_live_host_configuration(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui::init(cx);
        install(RendererPolicy::Replace, cx);
    });
    let (_, visual) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Sample);
        gpui::base::Root::new(content, window, cx)
    });
    visual.update(|window, cx| window.toggle_inspector(cx));
    visual.run_until_parked();
    let sample = visual.debug_bounds("kit-sample").unwrap();
    visual.simulate_mouse_move(sample.center(), None, Modifiers::none());
    visual.run_until_parked();
    visual.simulate_click(sample.center(), Modifiers::none());
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    // Exercise Kit's color event subscription against the actual picked host.
    let controls = visual.update(|window, cx| {
        let id = service::inspector_selection(window, cx).unwrap().id;
        let state = window
            .with_inspector_state::<DivInspectorState, _>(Some(&id), cx, |state, _| {
                state.as_ref().unwrap().clone()
            })
            .unwrap();
        let edits = cx.new(|_| StyleEditState::default());
        edits.update(cx, |edits, _| {
            edits.select(id.clone(), &state.base_style, state.bounds.size);
        });
        let controls = cx.new(|cx| Controls::new(edits, window, cx));
        controls.update(cx, |controls, cx| {
            controls.synchronize(id, &state, window, cx)
        });
        controls.read(cx).colors[0].clone().update(cx, |_, cx| {
            cx.emit(ColorPickerEvent::Change(Some(gpui::rgb(0x00cc00).into())));
        });
        controls
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert_eq!(
            service::inspector_selection(window, cx).unwrap().properties["Background"],
            "#00CC00".to_lowercase()
        );
    });
    drop(controls);
    let slider = visual.debug_bounds("inspector-slider-Padding").unwrap();
    visual.simulate_click(slider.center(), Modifiers::none());
    visual.run_until_parked();
    visual.update(|window, cx| {
        let config = service::active_inspector_configuration(window, cx).unwrap();
        assert!(
            config
                .changes
                .iter()
                .any(|change| change.property == "Padding")
        );
    });
    let reset = visual.debug_bounds("inspector-reset").unwrap();
    visual.simulate_click(reset.center(), Modifiers::none());
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert!(service::active_inspector_configuration(window, cx).is_none());
        assert!(
            window
                .with_inspector_state::<DivInspectorState, _>(
                    service::inspector_selection(window, cx)
                        .map(|s| s.id)
                        .as_ref(),
                    cx,
                    |_, _| ()
                )
                .is_some()
        );
        assert_eq!(
            service::inspector_selection(window, cx).unwrap().properties["Width"],
            "160 px"
        );
    });
}

#[gpui::test]
fn preserve_and_repeated_replace_leave_host_renderer_intact(cx: &mut TestAppContext) {
    let rendered = Rc::new(Cell::new(0));
    cx.update(|cx| {
        gpui::init(cx);
        let calls = rendered.clone();
        cx.set_inspector_renderer(Box::new(move |_, _, _| {
            calls.set(calls.get() + 1);
            div().into_any_element()
        }));
        install(RendererPolicy::Preserve, cx);
    });
    let (_, visual) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Sample);
        gpui::base::Root::new(content, window, cx)
    });
    visual.update(|window, cx| window.toggle_inspector(cx));
    visual.run_until_parked();
    assert!(rendered.get() > 0);
    visual.update(|window, cx| {
        install(RendererPolicy::Replace, cx);
        let calls = rendered.clone();
        cx.set_inspector_renderer(Box::new(move |_, _, _| {
            calls.set(calls.get() + 1);
            div().into_any_element()
        }));
        install(RendererPolicy::Replace, cx);
        rendered.set(0);
        window.refresh();
    });
    visual.run_until_parked();
    assert!(
        rendered.get() > 0,
        "repeated installation must not overwrite a later host renderer"
    );
}

#[gpui::test]
fn visible_toast_and_underlying_host_remain_pickable(cx: &mut TestAppContext) {
    use gpui::component::{
        WindowExt,
        notification::{Notification, NotificationDelivery},
    };
    cx.update(|cx| {
        gpui::init(cx);
        install(RendererPolicy::Replace, cx);
    });
    let (_, visual) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Sample);
        gpui::base::Root::new(content, window, cx)
    });
    visual.update(|window, cx| {
        window.push_notification(
            Notification::new()
                .autohide(false)
                .placement(gpui::Anchor::BottomLeft)
                .delivery(NotificationDelivery::InApp)
                .content(|_, _, _| {
                    div()
                        .id("toast-pick-target")
                        .debug_selector(|| "toast-pick-target".into())
                        .w(px(100.))
                        .h(px(32.))
                        .bg(gpui::rgb(0xcc6633))
                        .into_any_element()
                }),
            cx,
        );
        window.toggle_inspector(cx);
    });
    visual.run_until_parked();
    visual
        .background_executor
        .advance_clock(std::time::Duration::from_secs(1));
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    // Move between the host and visible overlay without restarting picking.
    // The native depth must reset at each position, not remain underneath a toast.
    for selector in ["kit-sample", "toast-pick-target", "kit-sample"] {
        let bounds = visual.debug_bounds(selector).unwrap();
        visual.simulate_mouse_move(bounds.center(), None, Modifiers::none());
        visual.run_until_parked();
        visual.update(|window, cx| {
            let picked = service::inspector_selection(window, cx).expect(selector);
            assert!(
                picked
                    .id
                    .path
                    .global_id
                    .iter()
                    .any(|id| id.to_string() == selector),
                "hovering {selector} picked {:?}",
                picked.id
            );
        });
    }
    // Native wheel traversal still works in both directions. Returning past
    // the top must settle on the visible control, never on an empty scaffold.
    for selector in ["toast-pick-target", "kit-sample"] {
        let position = visual.debug_bounds(selector).unwrap().center();
        visual.simulate_mouse_move(position, None, Modifiers::none());
        visual.run_until_parked();
        let first =
            visual.update(|window, cx| service::inspector_selection(window, cx).unwrap().id);
        for delta in [36., 36., -36., -3600.] {
            visual.simulate_event(gpui::ScrollEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta))),
                modifiers: Modifiers::none(),
                touch_phase: gpui::TouchPhase::Moved,
                source: Default::default(),
            });
            visual.run_until_parked();
        }
        visual.update(|window, cx| {
            assert_eq!(service::inspector_selection(window, cx).unwrap().id, first);
        });
    }
    for selector in ["toast-pick-target", "kit-sample"] {
        let bounds = visual.debug_bounds(selector).unwrap();
        visual.simulate_mouse_move(bounds.center(), None, Modifiers::none());
        visual.run_until_parked();
        visual.simulate_click(bounds.center(), Modifiers::none());
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
        visual.update(|window, cx| {
            let picked = service::inspector_selection(window, cx).expect(selector);
            assert!(
                picked
                    .id
                    .path
                    .global_id
                    .iter()
                    .any(|id| id.to_string() == selector),
                "picked {:?}",
                picked.id
            );
            assert!(
                window
                    .with_inspector_state::<DivInspectorState, _>(
                        service::inspector_selection(window, cx)
                            .map(|s| s.id)
                            .as_ref(),
                        cx,
                        |_, _| ()
                    )
                    .is_some()
            );
            window.toggle_inspector(cx);
            assert!(
                !window
                    .with_inspector_state::<DivInspectorState, _>(
                        service::inspector_selection(window, cx)
                            .map(|s| s.id)
                            .as_ref(),
                        cx,
                        |_, _| ()
                    )
                    .is_some()
            );
            window.toggle_inspector(cx);
        });
        visual.run_until_parked();
    }
}

#[gpui::test]
fn fast_pick_does_not_click_host_and_closing_restores_input(cx: &mut TestAppContext) {
    struct ClickTarget(Rc<Cell<usize>>);
    impl Render for ClickTarget {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let clicks = self.0.clone();
            div().size_full().child(
                div()
                    .id("fast-pick")
                    .debug_selector(|| "fast-pick".into())
                    .w(px(120.))
                    .h(px(60.))
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
        }
    }
    let clicks = Rc::new(Cell::new(0));
    cx.update(|cx| {
        gpui::init(cx);
        install(RendererPolicy::Replace, cx);
    });
    let (_, visual) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| ClickTarget(clicks.clone()));
        gpui::base::Root::new(content, window, cx)
    });
    visual.update(|window, cx| window.toggle_inspector(cx));
    visual.run_until_parked();
    let position = visual.debug_bounds("fast-pick").unwrap().center();
    // No frame between movement, press and release: correction must preserve
    // the completed pick rather than leave the inspector in hover mode.
    visual.update(|window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                position,
                modifiers: Modifiers::none(),
                pressed_button: None,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                position,
                click_count: 1,
                ..Default::default()
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position,
                button: gpui::MouseButton::Left,
                modifiers: Modifiers::none(),
                click_count: 1,
            }),
            cx,
        );
        window.refresh();
    });
    visual.run_until_parked();
    visual.update(|window, cx| {
        assert!(!window.is_inspector_picking(cx));
        let picked = service::inspector_selection(window, cx).unwrap();
        assert!(
            picked
                .id
                .path
                .global_id
                .iter()
                .any(|id| id.to_string() == "fast-pick"),
            "{:?}",
            picked.id
        );
        window.toggle_inspector(cx);
    });
    visual.run_until_parked();
    assert_eq!(
        clicks.get(),
        0,
        "inspector events must never activate the host"
    );
    visual.simulate_click(position, Modifiers::none());
    assert_eq!(
        clicks.get(),
        1,
        "normal clicks must resume after inspection"
    );
}

#[gpui::test]
fn published_kit_dialog_stays_visible_and_pickable(cx: &mut TestAppContext) {
    use gpui::component::WindowExt;
    cx.update(|cx| {
        gpui::init(cx);
        install(RendererPolicy::Replace, cx);
    });
    let (_, visual) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Sample);
        gpui::base::Root::new(content, window, cx)
    });
    visual.update(|window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.child(
                div()
                    .id("dialog-pick")
                    .debug_selector(|| "dialog-pick".into())
                    .w(px(100.))
                    .h(px(40.))
                    .bg(gpui::rgb(0xcc6633)),
            )
        });
        window.toggle_inspector(cx);
    });
    visual.run_until_parked();
    visual
        .background_executor
        .advance_clock(std::time::Duration::from_secs(1));
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let position = visual.debug_bounds("dialog-pick").unwrap().center();
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.run_until_parked();
    visual.simulate_click(position, Modifiers::none());
    visual.run_until_parked();
    assert!(visual.debug_bounds("dialog-pick").is_some());
    visual.update(|window, cx| {
        let picked = service::inspector_selection(window, cx).unwrap();
        assert!(
            picked
                .id
                .path
                .global_id
                .iter()
                .any(|id| id.to_string() == "dialog-pick"),
            "{:?}",
            picked.id
        );
    });
}
