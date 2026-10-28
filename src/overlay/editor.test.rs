use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

fn click_toolbar(
    overlay: &Entity<AnnotationOverlay>,
    selector: &'static str,
    cx: &mut VisualTestContext,
) {
    cx.update(|window, cx| {
        window.activate_window();
        overlay.update(cx, |this, cx| this.sync_toolbar_window(window, cx));
    });
    draw(cx);
    let handle = cx.update(|_, cx| overlay.read(cx).companion_toolbar.unwrap());
    let mut tools = VisualTestContext::from_window(handle.into(), &cx.cx);
    draw(&mut tools);
    let selector = if selector == "banner-notes" {
        "feedback-floating-comments"
    } else {
        selector
    };
    let target = tools.debug_bounds(selector).unwrap();
    tools.simulate_click(target.center(), Modifiers::none());
    draw(cx);
}

fn setup(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let store = Store::memory();
    let session = store
        .ensure_session("Editor test", "gpui://editor")
        .unwrap();
    cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx))
}

fn outside(cx: &mut VisualTestContext) {
    let point = cx.update(|window, _| {
        let size = window.viewport_size();
        point(size.width - px(4.), size.height - px(4.))
    });
    cx.simulate_click(point, Modifiers::none());
    draw(cx);
}

fn saved(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) -> String {
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            let note = this
                .store
                .add(
                    &this.session.id,
                    AnnotationInput {
                        inspection: None,
                        comment: "Original".into(),
                        element_path: "counter".into(),
                        element: "Counter".into(),
                        bounding_box: crate::Rect {
                            x: 40.,
                            y: 100.,
                            width: 180.,
                            height: 40.,
                        },
                        x: 20.,
                        y: 120.,
                        source_file: None,
                        gpui_configuration: None,
                        shape: None,
                    },
                )
                .unwrap();
            this.edit(&note.id, window, cx);
            note.id
        })
    })
}

#[gpui::test]
fn existing_note_should_require_edit_then_autosave_on_outside_click(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    let id = saved(&overlay, cx);
    draw(cx);
    cx.update(|_, cx| assert!(!overlay.read(cx).draft.as_ref().unwrap().editing));
    let edit = cx.debug_bounds("feedback-editor-primary").unwrap();
    cx.simulate_click(edit.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).draft.as_ref().unwrap().editing);
        assert!(overlay.read(cx).input.focus_handle(cx).is_focused(window));
    });
    cx.simulate_input(" updated");
    draw(cx);
    outside(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.draft.is_none());
        assert_eq!(
            this.store.annotation(&id).unwrap().feedback.comment,
            "Original updated"
        );
        assert_eq!(
            this.store
                .app_annotations(&this.session.app_name)
                .unwrap()
                .len(),
            1
        );
    });
}

#[gpui::test]
fn new_note_should_hug_one_line_grow_when_wrapped_and_save_when_clicked_away(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.active = true;
            this.select(
                Target {
                    id: "counter".into(),
                    label: "Counter".into(),
                    source: String::new(),
                    bounds: crate::Rect {
                        x: 40.,
                        y: 100.,
                        width: 180.,
                        height: 40.,
                    },
                },
                20.,
                120.,
                window,
                cx,
            );
        })
    });
    draw(cx);
    let short = cx
        .debug_bounds("feedback-editor-panel")
        .unwrap()
        .size
        .height;
    assert!(short < px(140.), "empty editor should hug one line");
    let text = "A comment that should wrap as the user writes more. ".repeat(8);
    cx.simulate_input(&text);
    draw(cx);
    let tall = cx
        .debug_bounds("feedback-editor-panel")
        .unwrap()
        .size
        .height;
    assert!(tall > short + px(40.), "wrapped lines grow the editor");
    outside(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.draft.is_none());
        let notes = this.store.app_annotations(&this.session.app_name).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].feedback.comment, text.trim());
        assert!(this.stroke.is_empty());
    });
}

#[gpui::test]
fn outside_click_should_discard_empty_new_note_and_retain_invalid_existing_edit(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    let id = saved(&overlay, cx);
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.start_editing(window, cx);
            this.input.update(cx, |input, cx| input.set_text("", cx));
        })
    });
    draw(cx);
    outside(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.draft.is_some());
        assert!(!this.notice.is_empty());
        assert_eq!(
            this.store.annotation(&id).unwrap().feedback.comment,
            "Original"
        );
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.draft = None;
            this.select(
                Target {
                    id: "new".into(),
                    label: "New".into(),
                    source: String::new(),
                    bounds: crate::Rect::default(),
                },
                20.,
                120.,
                window,
                cx,
            );
        })
    });
    draw(cx);
    outside(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).draft.is_none());
        assert_eq!(
            overlay
                .read(cx)
                .store
                .app_annotations("Editor test")
                .unwrap()
                .len(),
            1
        );
    });
}

#[gpui::test]
fn comments_and_settings_should_save_and_open_in_one_click_and_block_invalid_changes(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    let id = saved(&overlay, cx);
    cx.update(|_, cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
    for (selector, settings) in [
        ("banner-notes", false),
        ("feedback-settings-trigger-target", true),
    ] {
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                this.edit(&id, window, cx);
                this.start_editing(window, cx);
                this.input
                    .update(cx, |input, cx| input.set_text(selector, cx));
            })
        });
        draw(cx);
        click_toolbar(&overlay, selector, cx);
        draw(cx);
        cx.update(|window, cx| {
            overlay.update(cx, |this, cx| {
                assert!(this.draft.is_none());
                assert_eq!(this.settings_open, settings);
                assert_eq!(this.review_open, !settings);
                assert_eq!(
                    this.store.annotation(&id).unwrap().feedback.comment,
                    selector
                );
                this.dismiss_sheet(window, cx);
                this.edit(&id, window, cx);
                this.start_editing(window, cx);
                this.input.update(cx, |input, cx| input.set_text("", cx));
            })
        });
        draw(cx);
        click_toolbar(&overlay, selector, cx);
        draw(cx);
        cx.update(|_, cx| {
            let this = overlay.read(cx);
            assert_eq!(
                this.draft.as_ref().unwrap().id.as_deref(),
                Some(id.as_str())
            );
            assert!(!this.settings_open && !this.review_open);
            assert_eq!(
                this.store.annotation(&id).unwrap().feedback.comment,
                selector
            );
        });
        cx.update(|_, cx| {
            overlay.update(cx, |this, cx| {
                this.input
                    .update(cx, |input, cx| input.set_text(selector, cx));
            })
        });
    }
}

#[gpui::test]
fn another_canvas_pin_should_save_current_note_and_open_clicked_note_in_one_click(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    let ids = cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            let ids: Vec<_> = [(0.15, 0.2), (0.8, 0.7)]
                .into_iter()
                .map(|at| {
                    this.store
                        .add(
                            &this.session.id,
                            AnnotationInput {
                                inspection: None,
                                comment: "Original".into(),
                                element_path: format!("pin:{at:?}"),
                                element: "Pin".into(),
                                bounding_box: crate::Rect::default(),
                                x: at.0 * 100.,
                                y: at.1,
                                source_file: None,
                                gpui_configuration: None,
                                shape: Some(crate::AnnotationShape::Pin { at, color: 0 }),
                            },
                        )
                        .unwrap()
                        .id
                })
                .collect();
            this.session = this.store.session(&this.session.id).unwrap();
            this.edit(&ids[0], window, cx);
            this.start_editing(window, cx);
            this.input
                .update(cx, |input, cx| input.set_text("Saved before switching", cx));
            cx.notify();
            ids
        })
    });
    draw(cx);
    let pin = cx.debug_bounds("feedback-marker-number-2").unwrap();
    cx.simulate_click(pin.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            assert_eq!(
                this.store.annotation(&ids[0]).unwrap().feedback.comment,
                "Saved before switching"
            );
            assert_eq!(
                this.draft.as_ref().unwrap().id.as_deref(),
                Some(ids[1].as_str())
            );
            assert!(!this.draft.as_ref().unwrap().editing);
            this.start_editing(window, cx);
            this.input.update(cx, |input, cx| input.set_text("", cx));
        })
    });
    draw(cx);
    let pin = cx.debug_bounds("feedback-marker-number-1").unwrap();
    cx.simulate_click(pin.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            overlay.read(cx).draft.as_ref().unwrap().id.as_deref(),
            Some(ids[1].as_str())
        );
        assert_eq!(
            overlay
                .read(cx)
                .store
                .annotation(&ids[1])
                .unwrap()
                .feedback
                .comment,
            "Original"
        );
    });
}

#[gpui::test]
fn floating_comments_should_save_and_open_in_one_click(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    let id = saved(&overlay, cx);
    cx.update(|window, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        overlay.update(cx, |this, cx| {
            this.set_toolbar_banner(false, window, cx);
            this.start_editing(window, cx);
            this.input
                .update(cx, |input, cx| input.set_text("Floating saved", cx));
        });
    });
    draw(cx);
    click_toolbar(&overlay, "feedback-floating-comments", cx);
    draw(cx);
    assert!(cx.debug_bounds("feedback-comments-list").is_some());
    cx.update(|_, cx| {
        assert!(overlay.read(cx).draft.is_none());
        assert_eq!(
            overlay
                .read(cx)
                .store
                .annotation(&id)
                .unwrap()
                .feedback
                .comment,
            "Floating saved"
        );
    });
}

#[gpui::test]
fn saved_multiline_note_should_render_without_mutating_or_dropping_line_breaks(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    let id = saved(&overlay, cx);
    let text = "First line\n\nThird line\n";
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.store
                .update(
                    &id,
                    AnnotationPatch {
                        comment: Some(text.into()),
                        status: None,
                    },
                )
                .unwrap();
            this.edit(&id, window, cx);
        })
    });
    draw(cx);
    cx.update(|_, cx| {
        assert!(!overlay.read(cx).draft.as_ref().unwrap().editing);
        assert_eq!(overlay.read(cx).input.read(cx).text(), text);
    });
    click_toolbar(&overlay, "banner-notes", cx);
    draw(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).review_open);
        assert_eq!(
            overlay
                .read(cx)
                .store
                .annotation(&id)
                .unwrap()
                .feedback
                .comment,
            text
        );
    });
}

#[gpui::test]
fn explicit_save_should_keep_new_and_existing_notes_open_in_read_mode(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.active = true;
            this.select(
                Target {
                    id: "counter".into(),
                    label: "Counter".into(),
                    source: String::new(),
                    bounds: crate::Rect {
                        x: 40.,
                        y: 100.,
                        width: 180.,
                        height: 40.,
                    },
                },
                20.,
                120.,
                window,
                cx,
            );
        })
    });
    draw(cx);
    cx.simulate_input("New note");
    draw(cx);
    let save = cx.debug_bounds("feedback-editor-primary").unwrap();
    cx.simulate_click(save.center(), Modifiers::none());
    draw(cx);
    let id = cx.update(|_, cx| {
        let this = overlay.read(cx);
        let draft = this.draft.as_ref().expect("Save keeps the panel open");
        assert!(!draft.editing);
        assert_eq!(draft.index, Some(1));
        assert_eq!(draft.original_comment, "New note");
        assert_eq!(this.store.app_annotations("Editor test").unwrap().len(), 1);
        draft.id.clone().unwrap()
    });
    let edit = cx.debug_bounds("feedback-editor-primary").unwrap();
    cx.simulate_click(edit.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).draft.as_ref().unwrap().editing));
    cx.simulate_input(" updated");
    draw(cx);
    let save = cx.debug_bounds("feedback-editor-primary").unwrap();
    cx.simulate_click(save.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        let draft = this.draft.as_ref().unwrap();
        assert!(!draft.editing);
        assert_eq!(draft.id.as_deref(), Some(id.as_str()));
        assert_eq!(
            this.store.annotation(&id).unwrap().feedback.comment,
            "New note updated"
        );
        assert_eq!(this.store.app_annotations("Editor test").unwrap().len(), 1);
    });
    outside(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).draft.is_none()));
}
