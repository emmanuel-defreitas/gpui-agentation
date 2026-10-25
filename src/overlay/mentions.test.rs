use super::*;

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

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
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
                        comment: format!("Existing note {index}, see 12"),
                        element_path: format!("note-{index}"),
                        element: "Pin".into(),
                        x: 5.0 * index as f32,
                        y: 100.0,
                        bounding_box: crate::Rect::default(),
                        source_file: None,
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
