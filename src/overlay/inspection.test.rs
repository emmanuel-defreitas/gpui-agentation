use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};

struct Host {
    overlay: Entity<AnnotationOverlay>,
    offset: f32,
    shown: bool,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.overlay.clone())
            .children(self.shown.then(|| {
                div()
                    .id("anchored-card")
                    .debug_selector(|| "inspection-target".into())
                    .ml(px(self.offset))
                    .mt(px(40.))
                    .w(px(220.))
                    .h(px(120.))
                    .p_3()
                    .border_1()
                    .child("Inspect this element")
            }))
    }
}
/// The host's visible size; the hidden margin adds GPUI's sidebar to the viewport.
const HOST: gpui::Size<gpui::Pixels> = size(px(900.), px(650.));

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    // Test windows have no native content view to oversize: emulate the hidden
    // margin by widening the viewport by the sidebar while GPUI's inspector is on.
    let wanted = cx.update(|window, cx| {
        let mut wanted = HOST;
        if inspector::sidebar_open(window, cx) {
            wanted.width += inspector::sidebar_width(window);
        }
        wanted
    });
    if cx.update(|window, _| window.viewport_size()) != wanted {
        cx.simulate_resize(wanted);
    }
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn tools(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) -> VisualTestContext {
    let handle = cx.update(|_, cx| overlay.read(cx).companion_toolbar.unwrap());
    let mut tools = VisualTestContext::from_window(handle.into(), &cx.cx);
    let requested = tools.update(|window, _| window.bounds().size);
    tools.simulate_resize(requested);
    draw(&mut tools);
    tools
}
fn setup(
    cx: &mut TestAppContext,
) -> (
    Entity<Host>,
    Entity<AnnotationOverlay>,
    &mut VisualTestContext,
) {
    cx.update(crate::ely::init_for_tests);
    cx.update(|cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        cx.set_global(toolbar_window::SimulateHiddenMargin);
    });
    let (host, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Inspection", "gpui://inspection")
            .unwrap();
        Host {
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
            offset: 20.,
            shown: true,
        }
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(HOST);
    draw(cx);
    let overlay = cx.update(|_, cx| host.read(cx).overlay.clone());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    let mut palette = tools(&overlay, cx);
    draw(&mut palette);
    let bounds = cx.debug_bounds("inspection-target").unwrap();
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
    draw(cx);
    cx.simulate_click(bounds.center(), Modifiers::none());
    draw(cx);
    let mut palette = tools(&overlay, cx);
    draw(&mut palette);
    draw(cx);
    (host, overlay, cx)
}

#[gpui::test]
fn inspection_annotation_autosaves_properties_reopens_and_accepts_optional_comment(
    cx: &mut TestAppContext,
) {
    let (_, overlay, cx) = setup(cx);
    let original = cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(!this.active);
        assert!(this.draft.is_none());
        assert_eq!(this.session.annotations.len(), 1);
        let note = this.session.annotations[0].clone();
        assert_eq!(note.kind, "inspection");
        assert_eq!(note.index, 1);
        assert!(note.feedback.comment.is_empty());
        assert!(note.feedback.gpui_configuration.is_none());
        let properties = &note.feedback.inspection.as_ref().unwrap().properties;
        assert_eq!(properties.len(), 8);
        assert_eq!(properties["Background"], "none");
        assert_eq!(properties["Text color"], "none");
        note
    });
    // The panel lives in the companion; the host draws nothing in GPUI's sidebar.
    assert!(cx.debug_bounds("inspector-header").is_none());
    let mut palette = tools(&overlay, cx);
    assert!(palette.debug_bounds("inspector-header").is_some());
    assert!(palette.debug_bounds("feedback-toolbar-close").is_none());
    let track = palette
        .debug_bounds("inspector-slider-ely-inspector-opacity")
        .unwrap();
    palette.simulate_click(
        point(track.left() + track.size.width * 0.5, track.center().y),
        Modifiers::none(),
    );
    draw(&mut palette);
    draw(cx);
    let mut palette = tools(&overlay, cx);
    draw(&mut palette);
    draw(cx);
    cx.update(|_, cx| {
        let note = overlay.read(cx).store.annotation(&original.id).unwrap();
        assert_ne!(
            note.feedback.inspection.as_ref().unwrap().properties["Opacity"],
            "1"
        );
        assert!(note.feedback.gpui_configuration.is_some());
        assert_eq!(note.index, 1);
    });
    let pin = cx
        .debug_bounds("feedback-marker-number-1")
        .expect("anchored pin paints in consumer");
    cx.simulate_click(pin.center(), Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-editor-panel").is_some());
    assert!(cx.debug_bounds("feedback-editor-placeholder").is_none());
    cx.simulate_input("Use a lighter appearance");
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            assert!(this.save(window, cx));
            this.dismiss_note(window, cx);
        })
    });
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        let note = this.store.annotation(&original.id).unwrap();
        assert_eq!(note.feedback.comment, "Use a lighter appearance");
        assert!(note.feedback.inspection.is_some());
        let markdown = this.store.app_markdown(&this.session.app_name).unwrap();
        assert!(markdown.contains("Use a lighter appearance"));
        assert!(markdown.contains("Opacity"));
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.open_inspection(&original.id, window, cx)
        })
    });
    draw(cx);
    // Stock GPUI cannot reselect by identity: reopening re-enters picking and
    // the next pick of the same element resumes the note.
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert!(this.inspecting);
        assert!(window.is_inspector_picking(cx));
        assert_eq!(this.inspection_note.as_deref(), Some(original.id.as_str()));
        assert!(this.notice.contains("Pick the element again"));
    });
    let bounds = cx.debug_bounds("inspection-target").unwrap();
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
    draw(cx);
    cx.simulate_click(bounds.center(), Modifiers::none());
    draw(cx);
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.inspecting);
        assert_eq!(this.session.annotations.len(), 1);
        assert_eq!(this.inspection_note.as_deref(), Some(original.id.as_str()));
        assert!(
            this.store
                .annotation(&original.id)
                .unwrap()
                .feedback
                .gpui_configuration
                .is_some(),
            "reopening preserves saved overrides"
        );
    });
    let mut palette = tools(&overlay, cx);
    let reset = palette.debug_bounds("inspector-reset").unwrap();
    assert!(reset.size.width > px(0.));
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            assert!(this.delete(&original.id, window, cx));
        })
    });
    draw(cx);
    let mut palette = tools(&overlay, cx);
    draw(&mut palette);
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).session.annotations.is_empty()));
    let target = cx.debug_bounds("inspection-target").unwrap();
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    cx.simulate_mouse_move(target.center(), None, Modifiers::none());
    draw(cx);
    // GPUI swallows host clicks while picking; the companion's Inspect button
    // reads "Stop inspecting" and ends it.
    let mut palette = tools(&overlay, cx);
    let stop = palette.debug_bounds("companion-tool-i").unwrap();
    palette.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).inspecting);
        assert!(!window.is_inspector_picking(cx));
    });
}

#[gpui::test]
fn fresh_inspection_pick_preserves_saved_overrides(cx: &mut TestAppContext) {
    let (_, overlay, cx) = setup(cx);
    let original = cx.update(|_, cx| overlay.read(cx).session.annotations[0].clone());
    let mut palette = tools(&overlay, cx);
    let track = palette
        .debug_bounds("inspector-slider-ely-inspector-opacity")
        .unwrap();
    palette.simulate_click(
        point(track.left() + track.size.width * 0.5, track.center().y),
        Modifiers::none(),
    );
    draw(&mut palette);
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.stop_inspection(window, cx);
            this.choose_tool(None, window, cx);
        })
    });
    draw(cx);
    let bounds = cx.debug_bounds("inspection-target").unwrap();
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
    draw(cx);
    cx.simulate_click(bounds.center(), Modifiers::none());
    draw(cx);
    draw(cx);
    cx.update(|_, cx| {
        assert_eq!(overlay.read(cx).session.annotations.len(), 1);
        assert!(
            overlay
                .read(cx)
                .store
                .annotation(&original.id)
                .unwrap()
                .feedback
                .gpui_configuration
                .is_some()
        );
    });
}

#[gpui::test]
fn selected_inspection_reenters_picking_on_click_away_or_escape(cx: &mut TestAppContext) {
    let (_, overlay, cx) = setup(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).inspecting);
        assert!(!window.is_inspector_picking(cx));
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| this.toggle(window, cx));
        assert!(overlay.read(cx).active);
    });
    // Empty app content, away from the element and outside GPUI's sidebar.
    cx.simulate_click(point(px(380.), px(600.)), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).inspecting);
        assert!(window.is_inspector_picking(cx));
    });
    let target = cx.debug_bounds("inspection-target").unwrap();
    cx.simulate_click(target.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| assert!(!window.is_inspector_picking(cx)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).inspecting);
        assert!(window.is_inspector_picking(cx));
    });
    let target = cx.debug_bounds("inspection-target").unwrap();
    cx.simulate_click(target.center(), Modifiers::none());
    draw(cx);
    let mut palette = tools(&overlay, cx);
    palette.simulate_keystrokes("escape");
    draw(&mut palette);
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).inspecting);
        assert!(window.is_inspector_picking(cx));
    });
    // GPUI swallows host clicks while picking; the companion's Inspect button
    // reads "Stop inspecting" and ends it.
    let mut palette = tools(&overlay, cx);
    let stop = palette.debug_bounds("companion-tool-i").unwrap();
    palette.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).inspecting);
        assert!(!inspector::is_open(window, cx));
    });
}

#[gpui::test]
fn editing_an_older_inspection_keeps_undo_transactions_valid(cx: &mut TestAppContext) {
    let (_, overlay, cx) = setup(cx);
    let (inspection, later) = cx.update(|_, cx| {
        overlay.update(cx, |this, _cx| {
            let inspection = this.session.annotations[0].clone();
            let later = this
                .store
                .add(
                    &this.session.id,
                    AnnotationInput {
                        inspection: None,
                        comment: "Later note".into(),
                        element_path: "later".into(),
                        element: "Later".into(),
                        x: 10.,
                        y: 20.,
                        bounding_box: crate::Rect::default(),
                        source_file: None,
                        gpui_configuration: None,
                        shape: None,
                    },
                )
                .unwrap();
            this.record_change(None, Some(later.clone()));
            let mut updated = inspection.clone();
            updated.feedback.comment = "Older inspection edited later".into();
            this.store
                .replace_note(Some(&inspection), Some(&updated))
                .unwrap();
            this.record_inspection_change(inspection.clone(), updated);
            (inspection, later)
        })
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| this.undo(window, cx));
        assert!(overlay.read(cx).store.annotation(&inspection.id).is_ok());
        assert!(overlay.read(cx).store.annotation(&later.id).is_ok());
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| this.undo(window, cx));
        assert!(overlay.read(cx).store.annotation(&inspection.id).is_ok());
        assert!(overlay.read(cx).store.annotation(&later.id).is_err());
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| this.undo(window, cx));
        assert!(overlay.read(cx).store.annotation(&inspection.id).is_err());
    });
}

#[gpui::test]
fn inspection_pin_stays_where_the_element_was_inspected(cx: &mut TestAppContext) {
    let (host, overlay, cx) = setup(cx);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.stop_inspection(window, cx)));
    draw(cx);
    let inspected = cx.debug_bounds("inspection-target").unwrap();
    let pin = cx.debug_bounds("feedback-marker-number-1").unwrap();
    assert!(inspected.contains(&pin.center()), "pin sits on the element");
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.offset = 180.;
            cx.notify();
        })
    });
    draw(cx);
    // Stock GPUI exposes no element identity outside picking, so the pin keeps
    // the saved position rather than following the moved element.
    assert_eq!(
        cx.debug_bounds("feedback-marker-number-1")
            .unwrap()
            .center(),
        pin.center()
    );
}

#[gpui::test]
fn inspection_annotation_comments_use_companion_and_drawing_switch_is_independent(
    cx: &mut TestAppContext,
) {
    let (_, overlay, cx) = setup(cx);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.open_sheet(false, window, cx)));
    draw(cx);
    let mut palette = tools(&overlay, cx);
    assert!(cx.debug_bounds("feedback-comments-list").is_none());
    assert!(palette.debug_bounds("companion-comments-panel").is_some());
    assert!(palette.debug_bounds("feedback-comments-list").is_some());
    assert!(palette.debug_bounds("inspector-header").is_none());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.dismiss_sheet(window, cx)));
    draw(cx);
    let mut palette = tools(&overlay, cx);
    assert!(palette.debug_bounds("companion-panel").is_none());
    let arrow = palette.debug_bounds("companion-tool-a").unwrap();
    palette.simulate_click(arrow.center(), Modifiers::none());
    draw(&mut palette);
    cx.update(|_, cx| {
        assert!(!overlay.read(cx).active);
        assert_eq!(overlay.read(cx).tool, Some(crate::ely::media::Tool::Box));
    });
    let mode = palette.debug_bounds("feedback-annotation-toggle").unwrap();
    palette.simulate_click(mode.center(), Modifiers::none());
    draw(&mut palette);
    draw(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).active);
        assert!(!overlay.read(cx).inspecting);
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.toggle(window, cx);
            assert!(!this.active);
            assert!(!this.inspecting);
            assert!(!window.is_inspector_picking(cx));
        })
    });
}
