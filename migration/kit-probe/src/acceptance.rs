//! Deterministic checks on native windows, not macOS candidate-panel automation.
use super::*;
use gpui::EntityInputHandler;
use std::time::Duration;

pub(super) fn run(windows: Vec<(AnyWindowHandle, Entity<Probe>)>, cx: &mut App) {
    let (host, view) = windows[0].clone();
    let companion = windows[1].0;
    let input = view.read(cx).input.clone();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_millis(250)).await;
        let host_root = host.update(cx, |_, window, cx| {
            window.resize(size(px(420.), px(300.)));
            window.activate_window();
            input.update(cx, |state, cx| {
                state.focus(window, cx);
                state.set_value("host ", window, cx);
                state.replace_and_mark_text_in_range(None, "にほ", Some(2..2), window, cx);
                assert_eq!(state.marked_text_range(window, cx), Some(5..7));
                state.replace_text_in_range(None, "日本", window, cx);
                assert_eq!(state.value().as_ref(), "host 日本");
                assert_eq!(state.marked_text_range(window, cx), None);
                state.replace_and_mark_text_in_range(None, "x", Some(1..1), window, cx);
                state.replace_and_mark_text_in_range(None, "", None, window, cx);
                assert_eq!(state.value().as_ref(), "host 日本");
                assert_eq!(state.marked_text_range(window, cx), None);
            });
            window.root::<gpui::base::Root>().flatten().unwrap().entity_id()
        }).expect("native input composition");
        cx.background_executor().timer(Duration::from_millis(250)).await;
        host.update(cx, |_, window, cx| {
            assert_eq!(window.viewport_size(), size(px(420.), px(300.)));
            assert_eq!(window.focused(cx), Some(input.read(cx).focus_handle(cx)));
            Theme::change(ThemeMode::Dark, Some(window), cx);
        }).expect("small native host");
        cx.background_executor().timer(Duration::from_millis(250)).await;
        host.update(cx, |_, window, cx| {
            assert_eq!(cx.theme().mode, ThemeMode::Dark);
            Theme::change(ThemeMode::Light, Some(window), cx);
        }).expect("dark theme");
        let old_root = companion.update(cx, |_, window, _| {
            let id = window.root::<gpui::base::Root>().flatten().unwrap().entity_id();
            window.remove_window();
            id
        }).expect("close companion");
        cx.background_executor().timer(Duration::from_millis(250)).await;
        assert!(companion.update(cx, |_, _, _| ()).is_err());
        cx.update(|cx| assert_eq!(cx.windows().len(), 1));
        let reopened = cx.update(|cx| {
            assert_eq!(cx.theme().mode, ThemeMode::Light);
            gpui::open_window(WindowOptions::default(), cx, |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx));
                cx.new(|_| Probe { input })
            }).expect("reopen companion").0
        });
        reopened.update(cx, |_, window, _| {
            assert_ne!(window.root::<gpui::base::Root>().flatten().unwrap().entity_id(), old_root);
        }).expect("new companion root");
        host.update(cx, |_, window, cx| {
            window.activate_window();
            input.update(cx, |state, cx| state.focus(window, cx));
            assert_eq!(input.read(cx).value().as_ref(), "host 日本");
            assert_eq!(window.focused(cx), Some(input.read(cx).focus_handle(cx)));
            assert_eq!(window.root::<gpui::base::Root>().flatten().unwrap().entity_id(), host_root);
        }).expect("restore original host focus and root");
        cx.background_executor().timer(Duration::from_millis(250)).await;
        host.update(cx, |_, window, cx| {
            assert_eq!(input.read(cx).value().as_ref(), "host 日本");
            assert_eq!(window.focused(cx), Some(input.read(cx).focus_handle(cx)));
            assert_eq!(window.root::<gpui::base::Root>().flatten().unwrap().entity_id(), host_root);
        }).expect("host state after native event-loop turn");
        cx.update(|cx| {
            assert_eq!(cx.windows().len(), 2);
            println!("PASS native acceptance: 420x300 resize; composition commit/cancel; Dark→Light; companion close/reopen; original host root and text preserved, explicit host refocus");
            cx.quit();
        });
    }).detach();
}
