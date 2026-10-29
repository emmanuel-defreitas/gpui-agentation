//! Annotation editing, pins, drawing tools, shortcuts and @-mentions.
use super::super::mentions::{Motion, query};
use super::*;
use crate::{AnnotationShape, ToolbarPosition, ely::media::Tool};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

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
    let mut comments = toolbar(&overlay, cx);
    assert!(comments.debug_bounds("feedback-comments-list").is_some());
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

#[cfg(unix)]
#[test]
fn store_identity_resolves_aliases_before_and_after_the_notes_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original");
    let alias = dir.path().join("alias");
    std::fs::create_dir(&original).unwrap();
    std::os::unix::fs::symlink(&original, &alias).unwrap();
    let path = original.join("new-directory/notes.json");
    let aliased = alias.join("new-directory/notes.json");
    assert_eq!(store_path(&path).unwrap(), store_path(&aliased).unwrap());
    std::fs::create_dir(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(store_path(&path).unwrap(), store_path(&aliased).unwrap());
    let file_alias = dir.path().join("notes.json");
    std::os::unix::fs::symlink(&path, &file_alias).unwrap();
    assert_eq!(store_path(&path).unwrap(), store_path(&file_alias).unwrap());
}

fn text_draft(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            if !this.active {
                this.toggle(window, cx);
            }
            this.choose_tool(Some(Tool::Select), window, cx);
            this.color = 2;
            this.stroke = vec![(0.2, 0.4)];
            this.finish_stroke(point(px(200.), px(240.)), window, cx);
        })
    });
    draw(cx);
}

#[gpui::test]
fn textbox_saves_inline_and_reopens_for_editing_without_a_comment(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    text_draft(&overlay, cx);
    let width = cx.debug_bounds("feedback-editor-panel").unwrap().size.width;
    cx.update(|window, _| assert_eq!(width, gpui::rems(16.0).to_pixels(window.rem_size())));
    cx.simulate_input("Plain textbox");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    let id = cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.draft.is_none(), "saving closes the inline editor");
        let note = &this.session.annotations[0];
        assert!(matches!(&note.feedback.shape, Some(AnnotationShape::Text { text, color: 2, .. }) if text == "Plain textbox"));
        note.id.clone()
    });
    assert!(cx.debug_bounds("feedback-editor-panel").is_none());
    let text = cx.debug_bounds("feedback-textbox").unwrap();
    cx.simulate_click(text.center(), Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-textbox-selected").is_some());
    cx.update(|_, cx| assert_eq!(overlay.read(cx).selected_note.as_deref(), Some(id.as_str())));
    cx.simulate_keystrokes("enter");
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert_eq!(
            this.draft.as_ref().unwrap().id.as_deref(),
            Some(id.as_str())
        );
        assert!(this.draft.as_ref().unwrap().editing);
        assert!(this.input.focus_handle(cx).is_focused(window));
        assert_eq!(this.draft.as_ref().unwrap().color, 2);
    });
    cx.simulate_input(" edited");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    cx.update(|_, cx| {
        let note = overlay.read(cx).store.annotation(&id).unwrap();
        assert_eq!(note.feedback.comment, "Plain textbox edited");
        assert!(matches!(note.feedback.shape, Some(AnnotationShape::Text { text, .. }) if text == "Plain textbox edited"));
    });
    cx.simulate_keystrokes("ctrl-z");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            overlay
                .read(cx)
                .store
                .annotation(&id)
                .unwrap()
                .feedback
                .comment,
            "Plain textbox"
        )
    });
}

#[gpui::test]
fn deletion_undo_and_redo_keep_annotation_identity_and_number(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    text_draft(&overlay, cx);
    cx.simulate_input("Keep identity");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    let note = cx.update(|_, cx| overlay.read(cx).session.annotations[0].clone());
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.selected_note = Some(note.id.clone());
            window.focus(&this.focus, cx);
        })
    });
    draw(cx);
    cx.simulate_keystrokes("delete");
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).store.annotation(&note.id).is_err()));
    cx.simulate_keystrokes("ctrl-z");
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).store.annotation(&note.id).unwrap(), note));
    cx.simulate_keystrokes("ctrl-shift-z");
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).store.annotation(&note.id).is_err()));
}

#[gpui::test]
fn textbox_pin_selection_pointer_drag_and_context_actions_preserve_identity(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    text_draft(&overlay, cx);
    cx.simulate_input("Draggable textbox");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    let before = cx.update(|_, cx| overlay.read(cx).session.annotations[0].clone());
    assert!(cx.debug_bounds("feedback-marker-number-1").is_some());
    let body = cx.debug_bounds("feedback-textbox").unwrap();
    cx.update(|_, cx| {
        let measured = overlay.read(cx).textbox_bounds[&before.id];
        assert!(measured.size.width > gpui::px(40.));
        assert!(measured.size.height > gpui::px(20.));
    });
    cx.simulate_click(body.center(), Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-textbox-selected").is_some());
    let start = cx.debug_bounds("feedback-textbox").unwrap().center();
    cx.simulate_mouse_move(start, None, Modifiers::none());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::none());
    let end = start + gpui::point(px(92.), px(40.));
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Modifiers::none());
    draw(cx);
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        let after = overlay.read(cx).store.annotation(&before.id).unwrap();
        assert_eq!(after.index, before.index);
        assert_eq!(after.feedback.comment, before.feedback.comment);
        assert_ne!(after.feedback.shape, before.feedback.shape);
    });
    cx.simulate_keystrokes("ctrl-z");
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            assert_eq!(this.store.annotation(&before.id).unwrap(), before);
            this.note_action(
                &before.id,
                super::annotation_menu::NoteAction::Color(1),
                window,
                cx,
            );
            assert_eq!(this.store.annotation(&before.id).unwrap().color, Some(1));
            this.undo(window, cx);
            assert_eq!(this.store.annotation(&before.id).unwrap(), before);
        })
    });
    draw(cx);
    let at = cx.debug_bounds("feedback-textbox").unwrap().center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, gpui::MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, gpui::MouseButton::Right, Modifiers::none());
    draw(cx);
    let delete = cx
        .debug_bounds("menu-row-Delete")
        .expect("annotation context menu");
    cx.simulate_click(delete.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).store.annotation(&before.id).is_err()));
    cx.simulate_keystrokes("ctrl-z");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(
            overlay.read(cx).store.annotation(&before.id).unwrap(),
            before
        )
    });
}

#[gpui::test]
fn delete_shortcut_keeps_the_selected_note_after_editor_outside_click(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    text_draft(&overlay, cx);
    cx.simulate_input("Delete from toolbar");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    let note = cx.update(|_, cx| overlay.read(cx).session.annotations[0].clone());
    let text = cx.debug_bounds("feedback-textbox").unwrap();
    cx.simulate_click(text.center(), Modifiers::none());
    draw(cx);
    cx.simulate_keystrokes("delete");
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).store.annotation(&note.id).is_err()));
    cx.simulate_keystrokes("ctrl-z");
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).store.annotation(&note.id).unwrap(), note));
}

#[gpui::test]
fn tool_shortcuts_are_scoped_to_annotation_mode_and_preserve_typing(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|window, cx| window.focus(&overlay.read(cx).focus.clone(), cx));
    draw(cx);
    cx.simulate_keystrokes("b");
    cx.update(|_, cx| assert!(!overlay.read(cx).active));
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.toggle(window, cx)));
    draw(cx);
    for (key, tool) in [
        ("b", Tool::Box),
        ("a", Tool::Arrow),
        ("p", Tool::Pen),
        ("c", Tool::Pin),
        ("t", Tool::Select),
    ] {
        cx.simulate_keystrokes(key);
        draw(cx);
        cx.update(|_, cx| assert_eq!(overlay.read(cx).tool, Some(tool)));
    }
    text_draft(&overlay, cx);
    cx.simulate_keystrokes("b");
    cx.update(|_, cx| {
        assert_eq!(overlay.read(cx).tool, Some(Tool::Select));
        assert_eq!(overlay.read(cx).input.read(cx).text(), "b");
    });
}

#[gpui::test]
fn pin_drag_moves_only_the_pin_and_can_be_undone(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    text_draft(&overlay, cx);
    cx.simulate_input("Move pin");
    cx.simulate_keystrokes("ctrl-s");
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            let mut before = this.session.annotations[0].clone();
            before.feedback.shape = Some(AnnotationShape::Box {
                from: (0.1, 0.2),
                to: (0.4, 0.5),
                color: 2,
            });
            let old = this.store.annotation(&before.id).unwrap();
            this.store.replace_note(Some(&old), Some(&before)).unwrap();
            this.session = this.store.session(&this.session.id).unwrap();
            this.pin_drag = Some((before.clone(), (0., 0.)));
            this.finish_stroke(point(px(300.), px(300.)), window, cx);
            let after = this.store.annotation(&before.id).unwrap();
            assert_eq!(after.feedback.shape, before.feedback.shape);
            assert_ne!(after.feedback.x, before.feedback.x);
            this.undo(window, cx);
            assert_eq!(this.store.annotation(&before.id).unwrap(), before);
        })
    });
}

#[gpui::test]
fn none_position_removes_the_toolbar_and_menu_commands_restore_it(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|window, _| window.activate_window());
    cx.update(|_, cx| {
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(ToolbarPosition::None, cx)
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("banner-inspect").is_none());
    assert!(cx.debug_bounds("feedback-floating-comments").is_none());
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.dispatch_command(
                &commands::AnnotationCommand {
                    name: "toolbar".into(),
                },
                window,
                cx,
            );
        })
    });
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).companion_toolbar.is_some()));
}

gpui::actions!(release_test, [HostBox]);

struct ShortcutHost {
    overlay: Entity<AnnotationOverlay>,
    focus: FocusHandle,
    presses: usize,
}
impl Render for ShortcutHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("shortcut-host")
            .track_focus(&self.focus)
            .size_full()
            .on_action(cx.listener(|this, _: &HostBox, _, cx| {
                this.presses += 1;
                cx.notify();
            }))
            .child(self.overlay.clone())
    }
}

#[gpui::test]
fn annotation_shortcut_overrides_the_host_binding_only_while_enabled(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::ely::init_for_tests(cx);
        cx.bind_keys([gpui::KeyBinding::new("b", HostBox, None)]);
    });
    let (host, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Shortcuts", "gpui://shortcuts")
            .unwrap();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        ShortcutHost {
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
            focus,
            presses: 0,
        }
    });
    draw(cx);
    cx.simulate_keystrokes("b");
    cx.update(|_, cx| assert_eq!(host.read(cx).presses, 1));
    let overlay = cx.update(|_, cx| host.read(cx).overlay.clone());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.toggle(window, cx)));
    draw(cx);
    cx.update(|window, cx| assert!(overlay.read(cx).focus.is_focused(window)));
    cx.simulate_keystrokes("b");
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(host.read(cx).presses, 1);
        assert_eq!(overlay.read(cx).tool, Some(Tool::Box));
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.simulate_keystrokes("b");
    cx.update(|_, cx| assert_eq!(host.read(cx).presses, 2));
}

#[test]
fn queries_respect_caret_boundaries_and_numbered_references() {
    let q = query("see @#12 next", 8).unwrap();
    assert_eq!(q.search, "12");
    assert!(q.dropdown);
    assert_eq!(q.start, 4);
    assert_eq!(query("note #2", 7).unwrap().search, "2");
    assert!(query("mail@example", 12).is_none());
    assert!(query("@#2 ", 4).is_none());
    assert!(query("é @", 1).is_none());
}

#[gpui::test]
fn mention_dropdown_previews_hovered_and_typed_notes_and_inserts_reference(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        crate::ely::init_for_tests(cx);
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let (overlay, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Mentions", "gpui://mentions").unwrap();
        for index in 1..=12 {
            store
                .add(
                    &session.id,
                    AnnotationInput {
                        inspection: None,
                        comment: format!("Existing note {index}, see 12"),
                        element_path: format!("note-{index}"),
                        element: "Pin".into(),
                        x: 5.0 * index as f32,
                        y: 100.0,
                        bounding_box: crate::Rect::default(),
                        source_file: None,
                        gpui_configuration: None,
                        shape: Some(crate::AnnotationShape::Pin {
                            at: (0.05 * index as f32, 0.2),
                            color: 0,
                        }),
                    },
                )
                .unwrap();
        }
        let session = store.session(&session.id).unwrap();
        AnnotationOverlay::new(store, session, 0, window, cx)
    });
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.choose_tool(Some(crate::ely::media::Tool::Pin), window, cx);
            this.stroke = vec![(0.5, 0.4)];
            this.finish_stroke(gpui::point(px(400.), px(250.)), window, cx);
        })
    });
    draw(cx);
    cx.simulate_input("@");
    draw(cx);
    assert!(cx.debug_bounds("annotation-mentions").is_some());
    let row = cx.debug_bounds("mention-note-1").unwrap();
    cx.simulate_mouse_move(row.center(), None, gpui::Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert!(
            this.input.focus_handle(cx).is_focused(window),
            "hover keeps the input focused"
        );
        let id = &this.session.annotations[0].id;
        assert_eq!(this.mentions.preview.as_deref(), Some(id.as_str()));
        assert_eq!(this.pin_scale(id, window, cx), 1.35);
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("annotation-mentions").is_none());
    cx.update(|_, cx| {
        assert!(overlay.read(cx).draft.is_some());
        assert!(overlay.read(cx).mentions.preview.is_none());
    });
    cx.simulate_keystrokes("backspace");
    cx.simulate_input("@#2");
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(
            this.mentions.preview.as_deref(),
            Some(this.session.annotations[1].id.as_str())
        );
    });
    cx.simulate_keystrokes("enter");
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(this.input.read(cx).text(), "@#2 ");
        assert!(this.mentions.preview.is_none());
    });
    assert!(cx.debug_bounds("annotation-mentions").is_none());
    cx.update(|_, cx| {
        let input = overlay.read(cx).input.clone();
        input.update(cx, |input, cx| input.set_text("@#1", cx));
    });
    draw(cx);
    cx.simulate_keystrokes("down down");
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(
            this.mentions.preview.as_deref(),
            Some(this.session.annotations[1].id.as_str())
        );
    });
    cx.update(|_, cx| {
        let input = overlay.read(cx).input.clone();
        input.update(cx, |input, cx| input.set_text("@#12", cx));
    });
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(this.mention_candidates()[0].index, 12);
        assert_eq!(
            this.mentions.preview.as_deref(),
            Some(this.session.annotations[11].id.as_str())
        );
    });
    cx.simulate_keystrokes("enter");
    draw(cx);
    cx.update(|_, cx| assert_eq!(overlay.read(cx).input.read(cx).text(), "@#12 "));
}

#[test]
fn pin_motion_returns_to_normal_and_honors_reduced_motion() {
    let up = Motion {
        from: 1.0,
        to: 1.35,
        since: Instant::now() - Duration::from_secs(1),
    };
    assert_eq!(up.sample(Duration::from_millis(120)), (1.35, false));
    let down = Motion {
        from: 1.35,
        to: 1.0,
        since: Instant::now(),
    };
    assert_eq!(down.sample(Duration::ZERO), (1.0, false));
}
