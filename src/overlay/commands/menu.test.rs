use super::*;
use gpui::{TestAppContext, VisualTestContext};

fn menu<'a>(root: &'a Menu, name: &str) -> &'a Menu {
    root.items
        .iter()
        .find_map(|item| match item {
            MenuItem::Submenu(menu) if menu.name.as_ref() == name => Some(menu),
            _ => None,
        })
        .unwrap()
}

#[test]
fn native_menu_groups_use_real_disabled_states_and_checkmarks() {
    let off = build_menu(MenuState::default());
    assert!(off.items[0].is_disabled());
    assert!(off.items[1].is_disabled());
    assert!(menu(&off, "New").disabled);
    assert!(menu(&off, "Color").disabled);
    assert!(menu(&off, "Position").items[1].is_checked());
    let on = build_menu(MenuState {
        active: true,
        undo: true,
        floating: true,
        hidden: true,
        ..Default::default()
    });
    assert!(!on.items[0].is_disabled());
    assert!(on.items[1].is_disabled());
    assert!(!menu(&on, "New").disabled);
    assert!(menu(&on, "Position").disabled);
    assert!(menu(&on, "Position").items[0].is_checked());
    assert!(
        matches!(&off.items[0], MenuItem::Action { action, .. } if action.as_any().is::<UnavailableCommand>())
    );
    assert!(
        matches!(&on.items[0], MenuItem::Action { action, .. } if action.as_any().is::<AnnotationCommand>())
    );
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

#[gpui::test]
fn menu_tracks_window_state_and_preserves_host_menus(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::ely::init_for_tests(cx);
        cx.set_menus([
            Menu::new("Host").items([MenuItem::action("Host Action", gpui::NoAction)]),
            debug_menu(),
        ]);
    });
    let (overlay, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Menu", "gpui://menu").unwrap();
        AnnotationOverlay::new(store, session, 0, window, cx)
    });
    draw(cx);
    cx.update(|window, _| window.activate_window());
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.toggle(window, cx);
            this.sync_debug_menu(window.window_handle().window_id(), cx);
        });
    });
    draw(cx);
    cx.update(|_, cx| {
        let menus = cx.get_menus().unwrap();
        assert_eq!(menus[0].name.as_ref(), "Host");
        assert_eq!(menus[0].items.len(), 1);
        let debug = restore_menu(menus[1].clone());
        assert!(!menu(&debug, "New").disabled);
        assert!(menu(&debug, "Settings").items[2].is_checked());
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(ToolbarPosition::Floating, cx);
            this.dispatch_command(
                &AnnotationCommand {
                    name: "toolbar".into(),
                },
                window,
                cx,
            );
            assert!(this.toolbar_hidden);
            this.dispatch_command(
                &AnnotationCommand {
                    name: "banner".into(),
                },
                window,
                cx,
            );
            assert!(
                this.toolbar_hidden,
                "disabled position command does not execute"
            );
            this.dispatch_command(
                &AnnotationCommand {
                    name: "toolbar".into(),
                },
                window,
                cx,
            );
            assert!(!this.toolbar_hidden);
            assert!(!this.banner, "show restores floating position");
        });
    });
    let other = cx.update(|_, cx| {
        cx.open_window(gpui::WindowOptions::default(), |window, cx| {
            let store = Store::memory();
            let session = store.ensure_session("Other", "gpui://other").unwrap();
            cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx))
        })
        .unwrap()
    });
    cx.update(|_, cx| {
        other
            .update(cx, |_, window, _| window.activate_window())
            .unwrap()
    });
    draw(cx);
    cx.update(|_, cx| {
        let menus = cx.get_menus().unwrap();
        assert!(
            menu(&restore_menu(menus[1].clone()), "New").disabled,
            "the second window has annotation mode off"
        );
    });
    cx.update(|window, _| window.activate_window());
    draw(cx);
    cx.update(|_, cx| {
        let menus = cx.get_menus().unwrap();
        assert!(
            !menu(&restore_menu(menus[1].clone()), "New").disabled,
            "reactivating the first window restores its menu state"
        );
    });
}
