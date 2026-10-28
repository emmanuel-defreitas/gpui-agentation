use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, WindowVisibility};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

fn setup(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let (overlay, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Companion", "gpui://toolbar").unwrap();
        AnnotationOverlay::new(store, session, 0, window, cx)
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(size(px(900.), px(650.)));
    draw(cx);
    (overlay, cx)
}

fn toolbar(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) -> VisualTestContext {
    let handle = cx.update(|_, cx| overlay.read(cx).companion_toolbar.unwrap());
    let mut tools = VisualTestContext::from_window(handle.into(), &cx.cx);
    draw(&mut tools);
    tools
}

#[test]
fn dock_tracks_host_edges_and_height_with_fourteen_pixel_gap() {
    for host in [
        Bounds::new(point(px(100.), px(80.)), size(px(900.), px(650.))),
        Bounds::new(point(px(-1250.), px(310.)), size(px(700.), px(500.))),
    ] {
        let dock = dock_bounds(host, px(50.));
        assert_eq!(dock.left() - host.right(), px(14.));
        assert_eq!(dock.top(), host.top());
        assert_eq!(dock.bottom(), host.bottom());
        assert_eq!(dock.size.width, px(50.));
    }
}

#[gpui::test]
fn companion_is_permanent_vertical_and_changes_mode_in_the_host(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    assert!(cx.debug_bounds("feedback-floating-toolbar").is_none());
    assert!(cx.debug_bounds("feedback-banner").is_none());
    let mut tools = toolbar(&overlay, cx);
    let inspect = tools.debug_bounds("companion-tool-i").unwrap();
    let box_tool = tools.debug_bounds("companion-tool-b").unwrap();
    let comments = tools.debug_bounds("feedback-floating-comments").unwrap();
    let settings = tools
        .debug_bounds("feedback-settings-trigger-target")
        .unwrap();
    assert!(tools.debug_bounds("feedback-toolbar-close").is_none());
    assert!(inspect.bottom() <= box_tool.top());
    assert!(comments.bottom() <= settings.top());
    // Disabled drawing tools must not enable annotation mode.
    tools.simulate_click(box_tool.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(!overlay.read(cx).active));
    tools.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(window.is_inspector_picking(cx));
        assert!(!overlay.read(cx).active);
    });
    let mut tools = toolbar(&overlay, cx);
    assert!(tools.debug_bounds("companion-tool-i").is_none());
    assert!(tools.debug_bounds("companion-tool-b").is_some());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.stop_inspection(window, cx)));
    draw(cx);
    cx.update(|window, cx| {
        assert!(!window.is_inspector_picking(cx));
        assert!(!overlay.read(cx).active);
        assert!(overlay.read(cx).companion_toolbar.is_some());
    });
}

#[gpui::test]
fn companion_follows_resize_visibility_fullscreen_and_host_close(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.simulate_resize(size(px(760.), px(530.)));
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        let spec = this.companion_toolbar_spec.as_ref().unwrap();
        assert_eq!(
            spec.bounds,
            dock_bounds(window.bounds(), spec.bounds.size.width)
        );
    });
    let mut tools = toolbar(&overlay, cx);
    tools.update(|window, _| assert_eq!(window.viewport_size().height, px(530.)));
    cx.simulate_visibility_change(WindowVisibility::Hidden);
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(
            !overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.simulate_visibility_change(WindowVisibility::Visible);
    draw(cx);
    cx.update(|_, cx| {
        assert!(
            overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.update(|window, cx| {
        window.toggle_fullscreen();
        overlay.update(cx, |this, cx| this.sync_toolbar_window(window, cx));
    });
    cx.executor().advance_clock(Duration::from_millis(31));
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(
            !overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.update(|window, cx| {
        window.toggle_fullscreen();
        overlay.update(cx, |this, cx| this.sync_toolbar_window(window, cx));
    });
    draw(cx);
    assert!(cx.update(|_, cx| {
        overlay
            .read(cx)
            .companion_toolbar_spec
            .as_ref()
            .unwrap()
            .visible
    }));
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    cx.cx.update(|cx| {
        assert!(overlay.read(cx).companion_toolbar.is_none());
        assert!(cx.windows().is_empty());
    });
}

#[cfg(agentation_external_inspector)]
#[gpui::test]
fn external_inspector_width_reverses_without_jumping_and_cancels_on_close(cx: &mut TestAppContext) {
    fn assert_nested(tools: &mut VisualTestContext) {
        // Deliver the native resize callback omitted by TestWindow::resize.
        let requested = tools.update(|window, _| window.bounds().size);
        tools.simulate_resize(requested);
        draw(tools);
        let surface = tools
            .debug_bounds("feedback-companion-window-surface")
            .unwrap();
        let rail = tools.debug_bounds("feedback-native-toolbar").unwrap();
        let panel = tools.debug_bounds("external-inspector-panel").unwrap();
        let pixel = tools.update(|window, _| px(1. / window.scale_factor()));
        assert_eq!(surface.origin, point(px(0.), px(0.)));
        assert!((surface.size.width - requested.width).abs() <= pixel);
        assert!((surface.size.height - requested.height).abs() <= pixel);
        assert!(rail.left() >= surface.left() && rail.right() <= surface.right());
        assert!(panel.left() >= rail.right());
        assert!(panel.right() <= surface.right() && panel.bottom() <= surface.bottom());
    }
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = false);
        cx.set_reduce_motion(false);
    });
    let mut tools = toolbar(&overlay, cx);
    let handle = tools.update(|window, _| window.window_handle());
    let narrow = tools.update(|window, _| window.bounds().size.width);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    tools.update(|window, _| assert_eq!(window.bounds().size.width, narrow));
    // A reversal before the first animation frame has no distance to animate.
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.close(window, cx)));
    draw(cx);
    draw(&mut tools);
    assert!(tools.debug_bounds("external-inspector-panel").is_none());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    draw(&mut tools);
    tools.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(80));
    cx.run_until_parked();
    let middle = tools.update(|window, _| window.bounds().size.width);
    assert!(middle > narrow && middle < narrow + px(420.));
    assert_nested(&mut tools);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.close(window, cx)));
    draw(cx);
    tools.update(|window, _| {
        assert_eq!(
            window.bounds().size.width,
            middle,
            "reverse from presented width"
        )
    });
    draw(&mut tools);
    tools.update(|window, cx| window.simulate_next_frame(cx));
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(50));
    cx.run_until_parked();
    let closing = tools.update(|window, _| window.bounds().size.width);
    assert!(closing > narrow && closing < middle);
    assert_nested(&mut tools);
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    tools.update(|window, _| {
        assert_eq!(window.bounds().size.width, narrow);
        assert_eq!(window.window_handle(), handle);
    });
    // Teardown wins over outstanding animation ticks.
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    cx.cx.update(|cx| assert!(cx.windows().is_empty()));
}

#[cfg(agentation_external_inspector)]
#[gpui::test]
fn external_inspector_width_honors_reduced_motion(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    let mut tools = toolbar(&overlay, cx);
    let narrow = tools.update(|window, _| window.bounds().size.width);
    cx.update(|window, cx| {
        cx.set_reduce_motion(true);
        overlay.update(cx, |this, cx| this.choose_tool(None, window, cx));
    });
    draw(cx);
    tools.update(|window, _| assert_eq!(window.bounds().size.width, narrow + px(420.)));
    tools.update(|window, cx| {
        assert_eq!(
            window
                .root::<ToolbarWindow>()
                .unwrap()
                .unwrap()
                .read(cx)
                .width
                .opacity,
            1.
        );
    });
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.close(window, cx)));
    draw(cx);
    tools.update(|window, _| assert_eq!(window.bounds().size.width, narrow));
}

#[cfg(agentation_external_inspector)]
#[gpui::test]
fn external_inspector_content_waits_for_native_resize_and_stable_frame(cx: &mut TestAppContext) {
    fn opacity(tools: &mut VisualTestContext) -> f32 {
        tools.update(|window, cx| {
            window
                .root::<ToolbarWindow>()
                .unwrap()
                .unwrap()
                .read(cx)
                .width
                .opacity
        })
    }
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = false);
        cx.set_reduce_motion(false);
    });
    let mut tools = toolbar(&overlay, cx);
    let narrow = tools.update(|window, _| window.bounds().size.width);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    assert_eq!(opacity(&mut tools), 0.);
    draw(&mut tools);
    assert!(
        tools
            .debug_bounds("feedback-companion-window-surface")
            .is_some()
    );
    let hidden_stop = tools.debug_bounds("inspector-header").unwrap();
    tools.simulate_click(hidden_stop.center(), Modifiers::none());
    cx.update(|window, cx| {
        assert!(
            window.is_inspector_picking(cx),
            "hidden controls ignore clicks"
        )
    });
    tools.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(240));
    cx.run_until_parked();
    let final_size = tools.update(|window, _| window.bounds().size);
    assert_eq!(final_size.width, narrow + px(420.));
    assert_eq!(
        opacity(&mut tools),
        0.,
        "timer completion must not reveal before native resize"
    );
    tools.simulate_resize(final_size);
    draw(&mut tools);
    assert_eq!(
        opacity(&mut tools),
        0.,
        "wait for a stable frame at the final viewport"
    );
    tools.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(60));
    cx.run_until_parked();
    let halfway = opacity(&mut tools);
    assert!(halfway > 0. && halfway < 1.);
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    assert_eq!(opacity(&mut tools), 1.);

    cx.update(|window, cx| overlay.update(cx, |this, cx| this.close(window, cx)));
    draw(cx);
    assert_eq!(opacity(&mut tools), 0., "collapse also hides the interior");
    let expanded = tools.update(|window, _| window.bounds().size);
    cx.executor().advance_clock(Duration::from_millis(60));
    tools.run_until_parked();
    tools.update(|window, _| assert_eq!(window.bounds().size, expanded));
    draw(&mut tools);
    tools.update(|window, cx| window.simulate_next_frame(cx));
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(240));
    tools.run_until_parked();
    let collapsed = tools.update(|window, _| window.bounds().size);
    assert_eq!(collapsed.width, narrow);
    assert_eq!(opacity(&mut tools), 0., "collapse waits for native resize");
    tools.simulate_resize(collapsed);
    draw(&mut tools);
    assert_eq!(opacity(&mut tools), 0.);
    tools.update(|window, cx| window.simulate_next_frame(cx));
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(60));
    tools.run_until_parked();
    assert!(opacity(&mut tools) > 0. && opacity(&mut tools) < 1.);
    cx.executor().advance_clock(Duration::from_millis(100));
    tools.run_until_parked();
    assert_eq!(opacity(&mut tools), 1.);
    draw(&mut tools);
    assert!(tools.debug_bounds("external-inspector-panel").is_none());
}
