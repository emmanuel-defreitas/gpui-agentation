use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

fn overlay(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let store = Store::memory();
    let session = store
        .ensure_session("UI integration test", "gpui://test")
        .unwrap();
    cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx))
}

#[gpui::test]
fn settings_trigger_should_open_sheet_and_escape_should_restore_focus(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|window, cx| window.focus(&overlay.read(cx).focus.clone(), cx));
    draw(cx);
    let trigger = cx.debug_bounds("feedback-settings-trigger-target").unwrap();
    cx.simulate_click(trigger.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).settings_open);
        assert!(overlay.read(cx).sheet_focus.contains_focused(window, cx));
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).settings_open);
        assert!(overlay.read(cx).focus.is_focused(window));
    });
}

#[gpui::test]
fn banner_tools_and_trailing_actions_should_remain_reachable_while_annotating(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = overlay(cx);
    draw(cx);
    for (id, tool) in [
        ("banner-box", Some(Tool::Box)),
        ("banner-arrow", Some(Tool::Arrow)),
        ("banner-pen", Some(Tool::Pen)),
        ("banner-inspect", None),
    ] {
        let button = cx.debug_bounds(id).unwrap();
        cx.simulate_click(button.center(), Modifiers::none());
        draw(cx);
        cx.update(|window, cx| {
            assert!(overlay.read(cx).active);
            assert_eq!(overlay.read(cx).tool, tool);
            assert!(overlay.read(cx).focus.is_focused(window));
        });
        assert!(cx.debug_bounds("banner-notes").is_some());
        assert!(
            cx.debug_bounds("feedback-settings-trigger-target")
                .is_some()
        );
    }
    let notes = cx.debug_bounds("banner-notes").unwrap();
    cx.simulate_click(notes.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).review_open));
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|_, cx| assert!(!overlay.read(cx).active));
    assert!(
        cx.debug_bounds("banner-box").is_some(),
        "tools stay visible after Escape"
    );
    cx.update(|_, cx| assert_eq!(cx.windows().len(), 1));
}

struct Shell {
    overlay: Entity<AnnotationOverlay>,
    focus: gpui::FocusHandle,
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::ely::primitives::FocusScope::new(&self.focus)
            .root()
            .size_full()
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h_8()
                            .flex_none()
                            .debug_selector(|| "custom-titlebar".into()),
                    )
                    .child(self.overlay.clone())
                    .child(target::target(
                        "test-content",
                        "Application content",
                        div().flex_1().debug_selector(|| "app-content".into()),
                    )),
            )
    }
}

#[gpui::test]
fn banner_should_reserve_space_below_custom_titlebar_and_reflow_when_position_changes(
    cx: &mut TestAppContext,
) {
    cx.update(crate::ely::init_for_tests);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Layout test", "gpui://layout")
            .unwrap();
        Shell {
            focus: cx.focus_handle(),
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|_, cx| shell.read(cx).overlay.clone());
    for width in [900., 640.] {
        cx.simulate_resize(gpui::size(gpui::px(width), gpui::px(620.)));
        draw(cx);
        let title = cx.debug_bounds("custom-titlebar").unwrap();
        let banner = cx.debug_bounds("feedback-inline-root").unwrap();
        let content = cx.debug_bounds("app-content").unwrap();
        assert_eq!(title.bottom(), banner.top());
        assert_eq!(banner.bottom(), content.top());
        assert!(banner.size.height > gpui::px(0.));
        for id in [
            "banner-inspect",
            "banner-box",
            "banner-arrow",
            "banner-pen",
            "banner-notes",
            "feedback-settings-trigger-target",
        ] {
            let button = cx.debug_bounds(id).unwrap();
            assert!(button.origin.x >= banner.origin.x && button.right() <= banner.right());
            assert!(button.origin.y >= banner.origin.y && button.bottom() <= banner.bottom());
        }
    }
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| this.set_toolbar_banner(false, window, cx))
    });
    draw(cx);
    assert_eq!(
        cx.debug_bounds("custom-titlebar").unwrap().bottom(),
        cx.debug_bounds("app-content").unwrap().top()
    );
    cx.update(|_, cx| assert_eq!(cx.windows().len(), 1));
}

#[gpui::test]
fn palette_should_open_from_single_swatch_select_color_and_dismiss(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    draw(cx);
    assert!(cx.debug_bounds("banner-palette-color-2").is_none());
    let trigger = cx.debug_bounds("banner-palette-trigger").unwrap();
    cx.simulate_click(trigger.center(), Modifiers::none());
    draw(cx);
    let color = cx.debug_bounds("banner-palette-color-2").unwrap();
    cx.simulate_click(color.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).color, 2));
    assert!(cx.debug_bounds("banner-palette-color-2").is_none());
    assert!(cx.debug_bounds("banner-palette-trigger").is_some());
}

#[gpui::test]
fn inverse_banner_buttons_should_support_keyboard_activation(cx: &mut TestAppContext) {
    cx.update(crate::ely::init_for_tests);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Keyboard test", "gpui://keyboard")
            .unwrap();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Shell {
            focus,
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|_, cx| shell.read(cx).overlay.clone());
    draw(cx);
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    draw(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).active);
        assert_eq!(overlay.read(cx).tool, None);
    });
}

#[gpui::test]
fn comments_sheet_should_share_header_fill_width_and_keep_copy_at_bottom(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|_, cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
    cx.update(|_, cx| {
        overlay.update(cx, |this, _| {
            this.store
                .add(
                    &this.session.id,
                    AnnotationInput {
                        comment: "A saved comment".into(),
                        element_path: "counter".into(),
                        element: "Counter".into(),
                        x: 20.,
                        y: 80.,
                        bounding_box: crate::Rect::default(),
                        source_file: None,
                        shape: None,
                    },
                )
                .unwrap();
        })
    });
    draw(cx);
    let comments = cx.debug_bounds("banner-notes").unwrap();
    cx.simulate_click(comments.center(), Modifiers::none());
    draw(cx);
    let header = cx.debug_bounds("feedback-sheet-header").unwrap();
    let list = cx.debug_bounds("feedback-comments-list").unwrap();
    let add = cx.debug_bounds("feedback-comments-add").unwrap();
    let footer = cx.debug_bounds("feedback-comments-footer").unwrap();
    let body = cx.debug_bounds("feedback-sheet-body").unwrap();
    let scroll = cx.debug_bounds("feedback-comments-scroll").unwrap();
    assert_eq!(scroll.top(), header.bottom());
    assert_eq!(scroll.left(), body.left());
    assert_eq!(scroll.right(), body.right());
    assert_eq!(scroll.bottom(), footer.top());
    assert_eq!(footer.bottom(), body.bottom());
    let margin = list.left() - scroll.left();
    assert!(margin > gpui::px(0.));
    assert_eq!(scroll.right() - list.right(), margin);
    assert_eq!(list.top() - scroll.top(), margin);
    assert_eq!(add.size.width, list.size.width);
    assert!(list.size.width > header.size.width * 0.8);
    assert!(list.top() >= header.bottom());
    assert!(add.top() >= list.bottom());
    assert!(footer.top() > add.bottom());
    cx.update(|window, _| assert!(footer.top() > window.viewport_size().height * 0.7));
    let copy = cx.debug_bounds("feedback-comments-copy").unwrap();
    cx.simulate_click(copy.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).notice, "Copied Markdown for agent"));
    let add = cx.debug_bounds("feedback-comments-add").unwrap();
    cx.simulate_click(add.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        assert!(!overlay.read(cx).review_open);
        assert!(overlay.read(cx).active);
    });
}

#[gpui::test]
fn inspect_should_toggle_to_stop_and_switch_back_from_drawing_tools(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    draw(cx);
    for active in [true, false, true, false] {
        let button = cx.debug_bounds("banner-inspect").unwrap();
        cx.simulate_click(button.center(), Modifiers::none());
        draw(cx);
        cx.update(|_, cx| {
            assert_eq!(overlay.read(cx).active, active);
            assert!(overlay.read(cx).tool.is_none());
        });
    }
    let box_tool = cx.debug_bounds("banner-box").unwrap();
    cx.simulate_click(box_tool.center(), Modifiers::none());
    draw(cx);
    let inspect = cx.debug_bounds("banner-inspect").unwrap();
    cx.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).active);
        assert!(overlay.read(cx).tool.is_none());
    });
}

#[gpui::test]
fn hover_inspector_should_preserve_click_to_comment_and_clear_on_stop(cx: &mut TestAppContext) {
    cx.update(crate::ely::init_for_tests);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Hover test", "gpui://hover").unwrap();
        Shell {
            focus: cx.focus_handle(),
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|_, cx| shell.read(cx).overlay.clone());
    draw(cx);
    let inspect = cx.debug_bounds("banner-inspect").unwrap();
    cx.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    let content = cx.debug_bounds("app-content").unwrap();
    cx.simulate_mouse_move(content.center(), None, Modifiers::none());
    draw(cx);
    let card = cx.debug_bounds("feedback-inspector-overlay").unwrap();
    assert!(card.size.height < gpui::px(176.), "card hugs its content");
    let source = cx.debug_bounds("feedback-inspector-source").unwrap();
    let location = cx.debug_bounds("feedback-inspector-location").unwrap();
    assert!(source.top() >= card.top() && source.bottom() <= card.bottom());
    assert!(location.left() >= card.left() && location.right() <= card.right());
    cx.update(|_, cx| {
        let hovered = overlay.read(cx).hovered.as_ref().unwrap();
        assert_eq!(hovered.id, "test-content");
        assert_eq!(hovered.label, "Application content");
        assert!(hovered.source.contains("banner.test.rs:"));
        assert_eq!(hovered.bounds.width, f32::from(content.size.width));
    });
    // The card may cover the target, but must never consume its picking click.
    cx.simulate_click(source.center(), Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-inspector-overlay").is_none());
    cx.update(|_, cx| {
        assert_eq!(
            overlay.read(cx).draft.as_ref().unwrap().target.id,
            "test-content"
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    let inspect = cx.debug_bounds("banner-inspect").unwrap();
    cx.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        assert!(!overlay.read(cx).active);
        assert!(overlay.read(cx).hovered.is_none());
    });
    assert!(cx.debug_bounds("feedback-inspector-overlay").is_none());
}
