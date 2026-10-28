use super::*;
use crate::{AnnotationShape, ToolbarPosition, ely::media::Tool};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

fn setup(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let store = Store::memory();
    let session = store
        .ensure_session("Release test", "gpui://release")
        .unwrap();
    cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx))
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
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
