use super::*;

#[test]
fn legacy_shape_json_keeps_palette_indexes_and_optional_text_frames() {
    // Fixed wire fixtures: do not generate these from the current Rust types.
    let fixture = r#"[
        {"type":"box","from":[0.75,0.5],"to":[0.25,0.25],"color":0},
        {"type":"arrow","from":[0.25,0.25],"to":[0.75,0.5],"color":1},
        {"type":"pen","points":[[0.25,0.25],[0.5,0.75]],"color":2},
        {"type":"pin","at":[0.5,0.5],"color":3},
        {"type":"text","at":[0.25,0.25],"text":"日本語 @#12","color":0},
        {"type":"text","at":[0.25,0.25],"text":"Framed","color":3,"width":0.5,"height":0.25}
    ]"#;
    let shapes: Vec<AnnotationShape> = serde_json::from_str(fixture).unwrap();
    assert!(shapes.iter().all(AnnotationShape::valid));
    assert_eq!(
        shapes
            .iter()
            .map(AnnotationShape::color)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 0, 3]
    );
    assert!(
        matches!(&shapes[4], AnnotationShape::Text { width: None, height: None, text, .. } if text == "日本語 @#12")
    );
    assert!(matches!(
        &shapes[5],
        AnnotationShape::Text {
            width: Some(0.5),
            height: Some(0.25),
            ..
        }
    ));
    assert_eq!(
        serde_json::to_value(&shapes).unwrap(),
        serde_json::from_str::<serde_json::Value>(fixture).unwrap()
    );
}

#[test]
fn reverse_box_should_use_its_actual_center_in_logical_pixels() {
    let shape =
        AnnotationShape::from_stroke(AnnotationTool::Box, &[(0.8, 0.7), (0.2, 0.3)], 0).unwrap();
    let bounds = shape.bounds(1000.0, 800.0);
    assert!((bounds.x + bounds.width / 2.0 - 500.0).abs() < 0.001);
    assert!((bounds.y + bounds.height / 2.0 - 400.0).abs() < 0.001);
    assert_eq!(
        centered_panel((500.0, 400.0), (384.0, 240.0), (1000.0, 800.0), 8.0),
        (308.0, 280.0)
    );
}

#[test]
fn editor_should_stay_visible_at_window_edges() {
    assert_eq!(
        centered_panel((990.0, 790.0), (384.0, 240.0), (1000.0, 800.0), 8.0),
        (608.0, 552.0)
    );
    assert_eq!(
        centered_panel((0.0, 0.0), (384.0, 240.0), (1000.0, 800.0), 8.0),
        (8.0, 8.0)
    );
    assert_eq!(
        centered_panel((0.0, 0.0), (384.0, 240.0), (100.0, 100.0), 8.0),
        (8.0, 8.0)
    );
}

#[test]
fn shapes_should_round_trip_and_reject_invalid_geometry() {
    for tool in [
        AnnotationTool::Box,
        AnnotationTool::Arrow,
        AnnotationTool::Pen,
        AnnotationTool::Pin,
    ] {
        let shape = AnnotationShape::from_stroke(tool, &[(0.1, 0.2), (0.6, 0.8)], 2).unwrap();
        assert!(shape.valid());
        let json = serde_json::to_string(&shape).unwrap();
        assert_eq!(
            serde_json::from_str::<AnnotationShape>(&json).unwrap(),
            shape
        );
    }
    assert!(
        !AnnotationShape::Pin {
            at: (f32::NAN, 0.0),
            color: 0
        }
        .valid()
    );
    assert!(
        !AnnotationShape::Pin {
            at: (0.5, 0.5),
            color: 4
        }
        .valid()
    );
    assert!(
        !AnnotationShape::Pen {
            points: vec![],
            color: 0
        }
        .valid()
    );
    assert!(
        !AnnotationShape::Pen {
            points: vec![(0.5, 0.5); 4097],
            color: 0
        }
        .valid()
    );
}

#[test]
fn selection_should_follow_the_shape_edge_instead_of_its_bounding_rectangle() {
    let arrow = AnnotationShape::Arrow {
        from: (0.0, 0.0),
        to: (1.0, 1.0),
        color: 0,
    };
    assert!(arrow.hit((500.0, 500.0), (1000.0, 1000.0), 8.0));
    assert!(!arrow.hit((100.0, 900.0), (1000.0, 1000.0), 8.0));
    let boxed = AnnotationShape::Box {
        from: (0.2, 0.2),
        to: (0.8, 0.8),
        color: 0,
    };
    assert!(boxed.hit((200.0, 500.0), (1000.0, 1000.0), 8.0));
    assert!(!boxed.hit((500.0, 500.0), (1000.0, 1000.0), 8.0));
}

#[test]
fn only_pins_should_allow_a_click_without_a_drag() {
    for tool in [
        AnnotationTool::Box,
        AnnotationTool::Arrow,
        AnnotationTool::Pen,
        AnnotationTool::Text,
    ] {
        assert!(AnnotationShape::from_stroke(tool, &[(0.5, 0.5)], 0).is_none());
    }
    assert!(AnnotationShape::from_stroke(AnnotationTool::Pin, &[(0.5, 0.5)], 0).is_some());
}

#[test]
fn a_fast_drag_should_include_the_release_position_before_drag_move_activates() {
    for tool in [
        AnnotationTool::Box,
        AnnotationTool::Arrow,
        AnnotationTool::Pen,
        AnnotationTool::Pin,
    ] {
        let shape = finish_shape(tool, vec![(0.1, 0.2)], (0.8, 0.7), 0).unwrap();
        assert!(shape.points().contains(&(0.8, 0.7)));
    }
    assert!(finish_shape(AnnotationTool::Box, vec![], (0.8, 0.7), 0).is_none());
}

#[test]
fn note_editor_should_open_below_the_annotation_including_near_the_top_edge() {
    let bounds = Rect {
        x: 450.0,
        y: 8.0,
        width: 100.0,
        height: 80.0,
    };
    assert_eq!(
        note_panel_position(bounds, (384.0, 160.0), (1000.0, 800.0), 8.0),
        (308.0, 96.0)
    );
}

#[test]
fn note_editor_should_flip_above_when_the_annotation_is_near_the_bottom() {
    let bounds = Rect {
        x: 450.0,
        y: 700.0,
        width: 100.0,
        height: 80.0,
    };
    assert_eq!(
        note_panel_position(bounds, (384.0, 160.0), (1000.0, 800.0), 8.0),
        (308.0, 532.0)
    );
}

#[test]
fn textbox_click_places_an_auto_sized_box_and_a_drag_frames_one() {
    let viewport = (1000.0, 800.0);
    let clicked = text_box((0.2, 0.4), (0.21, 0.41), viewport, 1);
    assert_eq!(
        clicked,
        AnnotationShape::Text {
            at: (0.2, 0.4),
            text: String::new(),
            color: 1,
            width: None,
            height: None,
        }
    );
    // A reverse drag frames the same box as a forward one.
    let framed = text_box((0.6, 0.5), (0.2, 0.3), viewport, 1);
    let AnnotationShape::Text {
        at, width, height, ..
    } = &framed
    else {
        panic!("textbox");
    };
    assert_eq!(*at, (0.2, 0.3));
    assert!((width.unwrap() - 0.4).abs() < 1e-6);
    assert!((height.unwrap() - 0.2).abs() < 1e-6);
    let bounds = framed.bounds(viewport.0, viewport.1);
    assert!((bounds.width - 400.0).abs() < 0.01 && (bounds.height - 160.0).abs() < 0.01);
    // A drag that is short on one axis is still a click.
    assert!(matches!(
        text_box((0.2, 0.4), (0.6, 0.41), viewport, 1),
        AnnotationShape::Text {
            width: None,
            height: None,
            ..
        }
    ));
    // The frame hit-tests across its drawn area, not only its text.
    let mut framed = framed;
    if let AnnotationShape::Text { text, .. } = &mut framed {
        *text = "Hi".into();
    }
    assert!(framed.hit((550.0, 380.0), viewport, 0.0));
    assert!(!framed.hit((650.0, 380.0), viewport, 0.0));
    let json = serde_json::to_string(&framed).unwrap();
    assert_eq!(
        serde_json::from_str::<AnnotationShape>(&json).unwrap(),
        framed
    );
}

#[test]
fn note_editor_should_stay_in_the_viewport_when_neither_side_has_room() {
    let bounds = Rect {
        x: 900.0,
        y: 0.0,
        width: 100.0,
        height: 800.0,
    };
    let at = note_panel_position(bounds, (384.0, 160.0), (1000.0, 800.0), 8.0);
    assert_eq!(at, (608.0, 632.0));
}
