use super::*;
use gpui::{
    AppContext, Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, WindowHandle, WindowOptions, div, px,
};

struct Sample {
    width: f32,
    color: u32,
}
impl Render for Sample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("owned-inspector-sample")
                .debug_selector(|| "owned-inspector-sample".into())
                .w(px(self.width))
                .h(px(40.))
                .bg(gpui::rgb(self.color))
                .child("Sample"),
        )
    }
}

fn window(cx: &mut App, width: f32, color: u32) -> (WindowHandle<Sample>, Entity<Sample>) {
    let view = cx.new(|_| Sample { width, color });
    let bounds = gpui::Bounds::maximized(None, cx);
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, _| view.clone(),
        )
        .unwrap();
    (handle, view)
}

fn pick(handle: WindowHandle<Sample>, cx: &TestAppContext) -> VisualTestContext {
    let mut visual = VisualTestContext::from_window(handle.into(), cx);
    visual.update(|window, cx| window.toggle_inspector(cx));
    visual.run_until_parked();
    let bounds = visual.debug_bounds("owned-inspector-sample").unwrap();
    visual.simulate_mouse_move(bounds.center(), None, Modifiers::none());
    visual.simulate_click(bounds.center(), Modifiers::none());
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    visual
}

#[gpui::test]
fn companion_edits_reset_and_restore_keep_each_hosts_baseline(cx: &mut TestAppContext) {
    let ((a, _a_view), (b, _b_view), (panel_a, a_owner), (panel_b, b_owner)) = cx.update(|cx| {
        crate::init_inspection_tests(cx);
        let a = window(cx, 120., 0x336699);
        let b = window(cx, 200., 0xcc3333);
        let panel_a = window(cx, 60., 0);
        let panel_b = window(cx, 60., 0);
        register_target(panel_a.0.window_id(), a.0.into(), panel_a.1.entity_id(), cx);
        register_target(panel_b.0.window_id(), b.0.into(), panel_b.1.entity_id(), cx);
        (a, b, panel_a, panel_b)
    });
    let mut a_visual = pick(a, cx);
    let mut b_visual = pick(b, cx);
    let (a_id, a_edits, b_id, b_edits) = cx.update(|cx| {
        let state = cx.global::<State>();
        (
            state.selections[&a.window_id()].id.clone(),
            state.sessions[&a.window_id()].upgrade().unwrap(),
            state.selections[&b.window_id()].id.clone(),
            state.sessions[&b.window_id()].upgrade().unwrap(),
        )
    });
    // The same local element name in two views must keep separate baselines.
    assert_eq!(
        a_id.path.global_id.iter().last(),
        b_id.path.global_id.iter().last()
    );
    panel_a
        .update(cx, |_, window, cx| {
            style::apply_edit(
                &a_id,
                style::EditableStyle::Width,
                180.,
                &a_edits,
                window,
                cx,
            );
            style::apply_edit(
                &a_id,
                style::EditableStyle::Opacity,
                0.4,
                &a_edits,
                window,
                cx,
            );
            style::apply_color(
                &a_id,
                style::ColorTarget::Background,
                gpui::rgb(0x00cc00).into(),
                &a_edits,
                window,
                cx,
            );
        })
        .unwrap();
    panel_b
        .update(cx, |_, window, cx| {
            style::apply_edit(
                &b_id,
                style::EditableStyle::Width,
                260.,
                &b_edits,
                window,
                cx,
            );
            style::apply_color(
                &b_id,
                style::ColorTarget::Text,
                gpui::rgb(0x663399).into(),
                &b_edits,
                window,
                cx,
            );
        })
        .unwrap();
    a_visual.run_until_parked();
    b_visual.run_until_parked();
    let saved_b = cx.update(|cx| {
        let state = cx.global::<State>();
        let a = &state.configurations[&a.window_id()];
        let b = &state.configurations[&b.window_id()];
        assert_eq!(a.changes[0].before, "120px");
        assert_eq!(b.changes[0].before, "200px");
        assert!(
            a.changes
                .iter()
                .any(|change| change.property == "Opacity" && change.after == "0.4")
        );
        b.clone()
    });
    panel_a
        .update(cx, |_, window, cx| {
            style::reset_edits(&a_id, &a_edits, window, cx)
        })
        .unwrap();
    a_visual.run_until_parked();
    a_visual.update(|window, cx| {
        assert!(active_inspector_configuration(window, cx).is_none());
        let selected = inspector_selection(window, cx).unwrap();
        assert_eq!(selected.properties["Width"], "120 px");
        assert_eq!(selected.properties["Background"], "#336699");
        assert_eq!(selected.properties["Opacity"], "1");
    });
    b_visual.update(|window, cx| {
        assert_eq!(
            active_inspector_configuration(window, cx),
            Some(saved_b.clone())
        )
    });
    panel_b
        .update(cx, |_, window, cx| {
            style::reset_edits(&b_id, &b_edits, window, cx)
        })
        .unwrap();
    b_visual.run_until_parked();
    b_visual.update(|window, cx| {
        assert!(active_inspector_configuration(window, cx).is_none());
        assert_eq!(
            inspector_selection(window, cx).unwrap().properties["Width"],
            "200 px"
        );
    });
    panel_b
        .update(cx, |_, window, cx| {
            style::restore_inspector_configuration(&b_id, saved_b.clone(), window, cx)
        })
        .unwrap();
    b_visual.run_until_parked();
    b_visual.update(|window, cx| {
        assert_eq!(active_inspector_configuration(window, cx), Some(saved_b));
        let selection = inspector_selection(window, cx).unwrap();
        assert_eq!(selection.properties["Width"], "260 px");
        assert_eq!(selection.properties["Text color"], "#663399");
    });
    // Closing a retained companion removes its route, not its live host's edits.
    panel_a
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let state = cx.global::<State>();
        assert!(!state.targets.contains_key(&panel_a.window_id()));
        assert!(state.selections.contains_key(&a.window_id()));
        assert!(state.configurations.contains_key(&b.window_id()));
    });
    // Closing a host removes incoming routes/snapshots and clears retained edit state.
    b.update(cx, |_, window, _| window.remove_window()).unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let state = cx.global::<State>();
        assert!(!state.targets.contains_key(&panel_b.window_id()));
        assert!(!state.selections.contains_key(&b.window_id()));
        assert!(!state.configurations.contains_key(&b.window_id()));
        assert!(state.selections.contains_key(&a.window_id()));
        assert!(b_edits.read(cx).original_for(&b_id).is_none());
        forget_window(b.window_id(), cx); // idempotent after the close observer
    });
    drop((a_owner, b_owner));
}

#[gpui::test]
fn replaced_route_and_renderer_ownership_survive_stale_cleanup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let (host, _) = window(cx, 100., 0);
        let (panel, old) = window(cx, 50., 0);
        let replacement = cx.new(|_| Sample {
            width: 50.,
            color: 0,
        });
        register_target(panel.window_id(), host.into(), old.entity_id(), cx);
        register_target(panel.window_id(), host.into(), replacement.entity_id(), cx);
        unregister_target(panel.window_id(), old.entity_id(), cx);
        assert_eq!(
            cx.global::<State>().targets[&panel.window_id()].target,
            host.into()
        );
        unregister_target(panel.window_id(), replacement.entity_id(), cx);
        assert!(cx.global::<State>().targets.is_empty());
        assert!(claim_renderer(RendererOwner::Default, cx));
        assert!(!claim_renderer(RendererOwner::Default, cx));
        assert!(claim_renderer(RendererOwner::Agentation, cx));
        assert!(!claim_renderer(RendererOwner::Default, cx));
        assert!(!claim_renderer(RendererOwner::Agentation, cx));
    });
}
