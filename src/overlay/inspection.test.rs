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
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
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
    cx.update(|cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
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
    cx.simulate_resize(size(px(900.), px(650.)));
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
        assert_eq!(
            note.feedback.inspection.as_ref().unwrap().properties.len(),
            6
        );
        note
    });
    let mut palette = tools(&overlay, cx);
    assert!(
        palette
            .debug_bounds("external-inspector-feedback")
            .is_none()
    );
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
    let mut palette = tools(&overlay, cx);
    draw(&mut palette);
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
    let reset = palette.debug_bounds("inspector-reset").unwrap();
    palette.simulate_click(reset.center(), Modifiers::none());
    draw(&mut palette);
    draw(cx);
    draw(&mut palette);
    draw(cx);
    cx.update(|_, cx| {
        let note = overlay.read(cx).store.annotation(&original.id).unwrap();
        assert!(note.feedback.gpui_configuration.is_none());
        assert_eq!(
            note.feedback.inspection.as_ref().unwrap().properties["Opacity"],
            "1"
        );
        assert_eq!(note.feedback.comment, "Use a lighter appearance");
    });
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
}

#[gpui::test]
fn inspection_annotation_follows_element_layout_and_hides_when_element_disappears(
    cx: &mut TestAppContext,
) {
    let (host, overlay, cx) = setup(cx);
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.stop_inspection(window, cx)));
    draw(cx);
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.offset = 180.;
            cx.notify();
        })
    });
    draw(cx);
    let bounds = cx.debug_bounds("inspection-target").unwrap();
    let pin = cx.debug_bounds("feedback-marker-number-1").unwrap();
    assert_eq!(
        pin.center(),
        bounds.center(),
        "pin follows current element layout"
    );
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.shown = false;
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("feedback-marker-number-1").is_none());
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            let id = this.session.annotations[0].id.clone();
            this.open_inspection(&id, window, cx);
            assert!(!this.inspecting);
            assert!(this.notice.contains("not available"));
        })
    });
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
    assert!(palette.debug_bounds("external-inspector-panel").is_none());
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.toggle(window, cx);
            assert!(this.active);
            assert!(!this.inspecting);
            assert!(!window.is_inspector_picking(cx));
            this.toggle(window, cx);
            assert!(!this.active);
        })
    });
}
