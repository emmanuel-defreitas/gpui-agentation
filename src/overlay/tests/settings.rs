//! Settings sheet behavior. The popup manager and native menu tests live beside
//! this file but attach to their own modules to reach private state.
use super::super::settings::DisclosureButton;
use super::*;

#[gpui::test]
#[ignore = "Kit 0.10's Button has no aria_expanded; this branch drops the attribute"]
fn disclosure_exposes_expanded_state_on_the_actual_button(cx: &mut gpui::TestAppContext) {
    struct Disclosure(bool);
    impl Render for Disclosure {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            DisclosureButton {
                button: Button::new("test-config-toggle").label("MCP config"),
                expanded: self.0,
            }
        }
    }
    cx.update(crate::integration::init_for_tests);
    let (view, visual) = cx.add_window_view(|_, _| Disclosure(false));
    for expanded in [false, true, false] {
        view.update(visual, |view, cx| {
            view.0 = expanded;
            cx.notify();
        });
        draw(visual);
        visual.update(|window, _| {
            let root =
                gpui::base::test_support::find(window, &[], &"test-config-toggle".into()).unwrap();
            assert_eq!(root.role(), Some(gpui::Role::Button));
            assert_eq!(root.label(), Some("MCP config"));
            assert_eq!(root.expanded(), Some(expanded));
        });
    }
}

#[gpui::test]
fn settings_scroll_should_reach_sheet_edges_with_padding_inside_content(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        gpui::component::Theme::update(cx, |theme| {
            theme.motion.duration_fast = std::time::Duration::ZERO;
            theme.motion.duration_normal = std::time::Duration::ZERO;
        });
    });
    let store = Store::memory();
    let session = store
        .ensure_session("Scroll test", "gpui://scroll")
        .unwrap();
    let (overlay, cx) =
        cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
    cx.update(|window, cx| {
        gpui::component::Theme::update(cx, |theme| {
            theme.motion.duration_fast = std::time::Duration::ZERO;
            theme.motion.duration_normal = std::time::Duration::ZERO;
        });
        overlay.update(cx, |this, cx| this.open_sheet(true, window, cx));
    });
    for height in [560., 900.] {
        cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(height)));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        let header = cx.debug_bounds("feedback-sheet-header").unwrap();
        let body = cx.debug_bounds("feedback-sheet-body").unwrap();
        let scroll = cx.debug_bounds("feedback-settings-scroll").unwrap();
        assert_eq!(body.top(), header.bottom());
        assert_eq!(body.bottom(), gpui::px(height));
        assert_eq!(scroll, body, "scroll viewport must reach all body edges");
        // Return to the start after resizing; spacing belongs to the
        // content, not a stationary mask around the scroll viewport.
        cx.simulate_event(gpui::ScrollEvent {
            position: scroll.origin + gpui::point(gpui::px(2.), gpui::px(2.)),
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(2000.))),
            modifiers: gpui::Modifiers::none(),
            touch_phase: gpui::TouchPhase::Moved,
            source: gpui::ScrollSource::default(),
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        let position = cx.debug_bounds("feedback-settings-mcp").unwrap();
        let margin = position.left() - scroll.left();
        assert!(margin > gpui::px(0.));
        assert_eq!(scroll.right() - position.right(), margin);
        assert_eq!(position.top() - scroll.top(), margin);
        // Scroll in the gutter, outside the intentionally locked JSON input.
        cx.simulate_event(gpui::ScrollEvent {
            position: scroll.origin + gpui::point(gpui::px(2.), gpui::px(2.)),
            delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-2000.))),
            modifiers: gpui::Modifiers::none(),
            touch_phase: gpui::TouchPhase::Moved,
            source: gpui::ScrollSource::default(),
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        let about = cx.debug_bounds("feedback-settings-about").unwrap();
        assert!(about.top() >= scroll.top());
        assert!(about.bottom() <= scroll.bottom() - margin + gpui::px(1.));
    }
}

#[gpui::test]
fn copying_config_should_use_live_endpoint_and_restart_two_second_feedback(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        gpui::component::Theme::update(cx, |theme| {
            theme.motion.duration_fast = std::time::Duration::ZERO;
            theme.motion.duration_normal = std::time::Duration::ZERO;
        });
    });
    let store = Store::memory();
    let session = store
        .ensure_session("Config test", "gpui://config")
        .unwrap();
    let (overlay, cx) =
        cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
    cx.update(|window, cx| {
        gpui::component::Theme::update(cx, |theme| {
            theme.motion.duration_fast = std::time::Duration::ZERO;
            theme.motion.duration_normal = std::time::Duration::ZERO;
        });
        overlay.update(cx, |this, cx| this.open_sheet(true, window, cx));
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(cx.debug_bounds("feedback-mcp-config-code").is_none());
    let toggle = cx.debug_bounds("feedback-mcp-config-toggle").unwrap();
    let copy = cx.debug_bounds("feedback-mcp-copy-config-target").unwrap();
    let icon = cx.debug_bounds("feedback-mcp-config-icon").unwrap();
    assert!(
        icon.left() < toggle.left() + gpui::px(32.),
        "MCP starts at the disclosure's left edge"
    );
    assert!(
        toggle.right() <= copy.left(),
        "Copy trails the left-aligned disclosure"
    );
    cx.simulate_click(copy.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(cx.debug_bounds("feedback-mcp-config-code").is_none());
    cx.update(|_, cx| {
        assert!(!overlay.read(cx).mcp_config_expanded);
        assert!(overlay.read(cx).mcp_config_copied);
        assert!(
            cx.read_from_clipboard()
                .unwrap()
                .text()
                .unwrap()
                .contains("mcpServers")
        );
    });
    cx.simulate_click(toggle.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(cx.update(|_, cx| overlay.read(cx).mcp_config_expanded));
    let field = cx.debug_bounds("feedback-mcp-config-code").unwrap();
    assert!(
        field.size.height > gpui::px(100.),
        "formatted JSON grows to multiple lines"
    );
    let before = cx.update(|_, cx| {
        overlay
            .read(cx)
            .mcp_config_input
            .read(cx)
            .value()
            .to_string()
    });
    cx.simulate_click(field.center(), gpui::Modifiers::none());
    cx.simulate_input("must not edit JSON");
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).mcp_config_input.read(cx).value(), before));
    cx.simulate_event(gpui::ScrollEvent {
        position: field.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(-80.), gpui::px(-160.))),
        modifiers: gpui::Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
        source: gpui::ScrollSource::default(),
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    assert_eq!(
        cx.debug_bounds("feedback-mcp-config-code").unwrap(),
        field,
        "scrolling over JSON must not move the field"
    );
    let target = cx.debug_bounds("feedback-mcp-copy-config-target").unwrap();
    cx.simulate_click(target.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.mcp_config_copied);
        let copied = cx.read_from_clipboard().unwrap().text().unwrap();
        let config: serde_json::Value = serde_json::from_str(&copied).unwrap();
        assert_eq!(copied, serde_json::to_string_pretty(&config).unwrap());
        assert_eq!(copied, this.mcp_config_input.read(cx).value());
        assert_eq!(
            config["mcpServers"]["gpui_agentation"]["url"],
            format!("http://{}/mcp", this._server.as_ref().unwrap().address())
        );
        assert_eq!(config["mcpServers"]["gpui_agentation"]["type"], "http");
    });
    cx.executor().advance_clock(Duration::from_millis(1500));
    cx.run_until_parked();
    cx.update(|_, cx| overlay.update(cx, |this, cx| this.copy_mcp_config(cx)));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(1000));
    cx.run_until_parked();
    cx.update(|_, cx| assert!(overlay.read(cx).mcp_config_copied));
    cx.executor().advance_clock(Duration::from_millis(1001));
    cx.run_until_parked();
    cx.update(|_, cx| assert!(!overlay.read(cx).mcp_config_copied));
}

#[gpui::test]
fn starting_mcp_should_retain_an_off_request_until_start_finishes(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        gpui::component::Theme::update(cx, |theme| {
            theme.motion.duration_fast = std::time::Duration::ZERO;
            theme.motion.duration_normal = std::time::Duration::ZERO;
        });
    });
    let store = Store::memory();
    let session = store
        .ensure_session("Startup test", "gpui://startup")
        .unwrap();
    let (overlay, cx) =
        cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
    cx.update(|_, cx| {
        overlay.update(cx, |this, cx| {
            this._server.take();
            this.set_mcp_enabled(true, cx);
            assert!(this.mcp_starting);
            this.set_mcp_enabled(false, cx);
        })
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(!this.mcp_starting);
        assert!(
            !this
                ._server
                .as_ref()
                .expect("server started")
                .is_mcp_enabled()
        );
    });
}
