use super::*;
use crate::overlay::tests::draw;
use gpui::{Modifiers, TestAppContext, VisualTestContext};

fn host(cx: &mut TestAppContext, name: &str) -> (Entity<AnnotationOverlay>, VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let (owner, visual) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session(name, "gpui://settings").unwrap();
        AnnotationOverlay::new(store, session, 0, window, cx)
    });
    visual.update(|window, _| window.activate_window());
    let mut visual = VisualTestContext::from_window(
        visual.update(|window, _| window.window_handle()),
        &visual.cx,
    );
    draw(&mut visual);
    (owner, visual)
}

fn show(cx: &mut VisualTestContext) -> VisualTestContext {
    let id = cx.update(|window, _| window.window_handle().window_id());
    let handle = cx.cx.update(|cx| {
        let manager = manager(cx);
        manager.update(cx, |this, cx| {
            this.open(
                id,
                Some(Anchor {
                    button: Bounds::new(point(px(850.), px(0.)), size(px(24.), px(24.))),
                    work_area: Bounds::new(point(px(0.), px(24.)), size(px(1000.), px(760.))),
                    display: None,
                }),
                cx,
            );
            this.popup.expect("settings popup")
        })
    });
    cx.run_until_parked();
    let mut popup = VisualTestContext::from_window(handle.into(), &cx.cx);
    draw(&mut popup);
    popup
}

#[test]
fn popup_fits_its_menu_bar_display_without_crossing_edges() {
    let anchor = Anchor {
        button: Bounds::new(point(px(980.), px(0.)), size(px(20.), px(24.))),
        work_area: Bounds::new(point(px(0.), px(24.)), size(px(1000.), px(720.))),
        display: None,
    };
    let bounds = popup_bounds(anchor, size(px(380.), px(620.)));
    assert_eq!(bounds.right(), px(992.));
    assert_eq!(bounds.top(), px(26.));
    assert!(bounds.bottom() <= px(736.));
    let small = Anchor {
        button: Bounds::new(point(px(0.), px(0.)), size(px(20.), px(20.))),
        work_area: Bounds::new(point(px(0.), px(20.)), size(px(300.), px(400.))),
        display: None,
    };
    let bounds = popup_bounds(small, size(px(380.), px(620.)));
    assert_eq!(bounds.size, size(px(284.), px(384.)));
}

#[gpui::test]
fn rich_popup_has_own_input_copies_collapsed_config_and_preserves_consumer(
    cx: &mut TestAppContext,
) {
    let (owner, mut host) = host(cx, "First consumer");
    host.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    host.update(|window, cx| owner.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(&mut host);
    let mut popup = show(&mut host);
    assert!(popup.debug_bounds("feedback-settings-popup").is_some());
    for selector in [
        "feedback-settings-brand",
        "feedback-settings-mcp",
        "feedback-settings-about",
    ] {
        assert!(popup.debug_bounds(selector).is_some(), "missing {selector}");
    }
    let panel = popup.debug_bounds("feedback-settings-popup").unwrap();
    let about = popup.debug_bounds("feedback-settings-about").unwrap();
    for (selector, height) in [
        ("feedback-settings-mcp", 116.),
        ("feedback-settings-about", 98.),
    ] {
        assert_eq!(
            popup.debug_bounds(selector).unwrap().size.height,
            px(height)
        );
    }
    assert_eq!(
        popup
            .debug_bounds("feedback-mcp-server-toggle")
            .unwrap()
            .size,
        size(px(36.), px(16.)),
    );
    assert!(
        about.bottom() <= panel.bottom(),
        "About card {about:?} must fit the compact popup {panel:?}"
    );
    assert!(host.debug_bounds("feedback-sheet-header").is_none());
    host.update(|window, cx| {
        assert!(window.is_inspector_picking(cx));
        assert!(!owner.read(cx).settings_open);
    });
    popup.update(|window, cx| {
        let view = window.root::<SettingsPopup>().unwrap().unwrap();
        assert_ne!(
            view.read(cx).config.entity_id(),
            owner.read(cx).mcp_config_input.entity_id()
        );
    });
    let copy = popup
        .debug_bounds("feedback-mcp-copy-config-target")
        .unwrap();
    popup.simulate_click(copy.center(), Modifiers::none());
    draw(&mut popup);
    assert!(popup.debug_bounds("feedback-mcp-config-code").is_none());
    popup.update(|_, cx| {
        let text = cx.read_from_clipboard().unwrap().text().unwrap();
        assert_eq!(text, owner.read(cx).mcp_config_input.read(cx).text());
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("before shortcut".into()));
    });
    popup.simulate_keystrokes("ctrl-cmd-c");
    draw(&mut popup);
    popup.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            owner.read(cx).mcp_config_input.read(cx).text(),
        );
    });
    let disclosure = popup.debug_bounds("feedback-mcp-config-toggle").unwrap();
    popup.simulate_click(disclosure.center(), Modifiers::none());
    draw(&mut popup);
    assert!(popup.debug_bounds("feedback-mcp-config-code").is_some());
    let disclosure = popup.debug_bounds("feedback-mcp-config-toggle").unwrap();
    popup.simulate_click(disclosure.center(), Modifiers::none());
    draw(&mut popup);
    assert!(popup.debug_bounds("feedback-mcp-config-code").is_none());
    popup.simulate_keystrokes("escape");
    host.run_until_parked();
    host.update(|window, cx| {
        assert!(!is_open_for(window.window_handle().window_id(), cx));
        assert!(window.is_inspector_picking(cx));
    });
}

#[gpui::test]
fn one_manager_targets_last_consumer_and_ignores_stale_dismissal(cx: &mut TestAppContext) {
    let (_one, mut first) = host(cx, "First consumer");
    let (_two, mut second) = host(&mut first.cx, "Second consumer");
    let second_id = second.update(|window, _| window.window_handle().window_id());
    let mut popup = show(&mut second);
    let old_generation = popup.update(|_, cx| {
        let manager = cx.global::<Manager>().0.read(cx);
        assert_eq!(manager.hosts.len(), 2);
        assert_eq!(manager.last_active, Some(second_id));
        assert_eq!(manager.selected, Some(second_id));
        manager.generation
    });
    popup.update(|_, cx| close(cx, false));
    popup.run_until_parked();
    let mut reopened = show(&mut second);
    reopened.update(|_, cx| {
        let manager = cx.global::<Manager>().0.clone();
        manager.update(cx, |this, cx| {
            this.dismiss_generation(old_generation, cx);
            assert!(this.popup.is_some());
        });
    });
    second.update(|window, _| window.remove_window());
    second.run_until_parked();
    first.update(|_, cx| {
        let manager = cx.global::<Manager>().0.read(cx);
        assert_eq!(manager.hosts.len(), 1);
        assert!(manager.popup.is_none());
        assert!(manager.selected.is_none());
    });
}

#[gpui::test]
fn closing_consumer_uses_activation_history_and_unmount_is_identity_guarded(
    cx: &mut TestAppContext,
) {
    let (one, mut first) = host(cx, "One");
    let (_two, mut second) = host(&mut first.cx, "Two");
    let (_three, mut third) = host(&mut first.cx, "Three");
    let first_id = first.update(|window, _| window.window_handle().window_id());
    let second_id = second.update(|window, _| window.window_handle().window_id());
    let third_id = third.update(|window, _| window.window_handle().window_id());
    first.cx.update(|cx| {
        activate(third_id, cx);
        activate(first_id, cx);
        activate(second_id, cx);
    });
    first.run_until_parked();
    second.update(|window, _| window.remove_window());
    first.run_until_parked();
    first.update(|_, cx| {
        assert_eq!(
            cx.global::<Manager>().0.read(cx).last_active,
            Some(first_id)
        )
    });
    first.update(|_, cx| unregister(third_id, one.entity_id(), cx));
    first.run_until_parked();
    first.update(|_, cx| assert_eq!(cx.global::<Manager>().0.read(cx).hosts.len(), 2));
    // Even a rendered popup must not keep an unmounted consumer's overlay alive.
    let _popup = show(&mut first);
    first.update(|window, cx| {
        window.replace_root(cx, |_, _| gpui::EmptyView);
    });
    drop(one);
    draw(&mut first);
    third.update(|_, cx| {
        let manager = cx.global::<Manager>().0.read(cx);
        assert!(manager.popup.is_none());
        assert_eq!(manager.hosts.len(), 1);
        assert_eq!(manager.last_active, Some(third_id));
    });
}
