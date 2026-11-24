//! Opt-in native acceptance against the production consumer and tool windows.
use gpui::*;
use std::time::Duration;

fn click(handle: AnyWindowHandle, id: impl Into<ElementId>, cx: &mut App) {
    let id = id.into();
    handle
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            let bounds = gpui::base::test_support::find(window, &[], &id)
                .unwrap_or_else(|| panic!("missing Kit button {id:?}"))
                .bounds();
            let position = bounds.center();
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position,
                    pressed_button: None,
                    modifiers: Modifiers::none(),
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::none(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    position,
                    button: MouseButton::Left,
                    modifiers: Modifiers::none(),
                    click_count: 1,
                }),
                cx,
            );
        })
        .unwrap();
}

pub(super) fn run(host: WindowHandle<gpui::base::Root>, cx: &mut App) {
    let original = host
        .update(cx, |root, window, cx| {
            (
                window.bounds().size,
                cx.entity_id(),
                root.view().entity_id(),
            )
        })
        .unwrap();
    if std::env::args().any(|arg| arg == "--from-in-host") {
        host.update(cx, |_, window, cx| window.toggle_inspector(cx))
            .unwrap();
    }
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_millis(700)).await;
        let toolbar = cx.update(|cx| {
            assert_eq!(cx.windows().len(), 2, "one host and one companion");
            let toolbar = cx.windows().into_iter().find(|w| *w != host.into()).unwrap();
            toolbar.update(cx, |_, window, _| assert!(window.root::<gpui::base::Root>().flatten().is_some())).unwrap();
            click(toolbar, ("feedback-tool", 0_usize), cx);
            toolbar
        });
        cx.background_executor().timer(Duration::from_millis(500)).await;
        cx.update(|cx| {
            host.update(cx, |root, window, cx| {
                assert!(window.is_inspector_picking(cx), "native Inspect starts picking");
                assert_eq!(window.bounds().size, original.0, "visible host size");
                assert_eq!(cx.entity_id(), original.1, "host Root retained");
                assert_eq!(root.view().entity_id(), original.2, "host content retained");
            }).unwrap();
            click(toolbar, ("feedback-tool", 0_usize), cx);
        });
        cx.background_executor().timer(Duration::from_millis(900)).await;
        cx.update(|cx| {
            host.update(cx, |_, window, cx| assert!(!window.is_inspector_picking(cx), "Stop closes picking")).unwrap();
            click(toolbar, "feedback-settings-trigger", cx);
        });
        cx.background_executor().timer(Duration::from_millis(900)).await;
        cx.update(|cx| {
            if cx.windows().len() == 2 {
                host.update(cx, |_, window, _| {
                    assert!(gpui::base::test_support::find(window, &[], &"feedback-settings-close".into()).is_some(), "Settings opens the Kit fallback sheet");
                }).unwrap();
            } else {
                assert_eq!(cx.windows().len(), 3, "Settings opens its own Kit window");
            }
            for handle in cx.windows() {
                handle.update(cx, |_, window, _| assert!(window.root::<gpui::base::Root>().flatten().is_some())).unwrap();
            }
            host.update(cx, |_, window, _| window.remove_window()).unwrap();
        });
        cx.background_executor().timer(Duration::from_millis(350)).await;
        cx.update(|cx| {
            assert!(cx.windows().is_empty(), "host close removes all tool windows");
            println!("Native production acceptance passed: Kit init, Root, Inspect/Stop, visible size, Settings and host-close cleanup");
            cx.quit();
        });
    }).detach();
}
