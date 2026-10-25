use super::*;

fn feedback() -> AnnotationInput {
    AnnotationInput {
        comment: "Add vertical padding".into(),
        element_path: "titlebar".into(),
        element: "Title bar".into(),
        x: 20.0,
        y: 10.0,
        bounding_box: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 54.0,
        },
        source_file: Some("src/shell.rs:12:3".into()),
        shape: None,
    }
}

#[test]
fn feedback_should_survive_restart_with_route_and_thread() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("feedback.json");
    let store = Store::open(&path).unwrap();
    let home = store.ensure_session("Corpora", "gpui://corpora/").unwrap();
    let search = store
        .ensure_session("Corpora", "gpui://corpora/explore")
        .unwrap();
    let note = store.add(&home.id, feedback()).unwrap();
    store.reply(&note.id, "agent", "Working on it").unwrap();
    store
        .update(
            &note.id,
            AnnotationPatch {
                status: Some(AnnotationStatus::Acknowledged),
                comment: None,
            },
        )
        .unwrap();
    drop(store);
    let reloaded = Store::open(path).unwrap();
    assert_eq!(
        reloaded
            .ensure_session("Corpora", "gpui://corpora/")
            .unwrap()
            .id,
        home.id
    );
    assert_eq!(
        reloaded.annotation(&note.id).unwrap().thread[0].message,
        "Working on it"
    );
    assert_eq!(
        reloaded.annotation(&note.id).unwrap().status,
        AnnotationStatus::Acknowledged
    );
    assert!(reloaded.session(&search.id).unwrap().annotations.is_empty());
    assert!(
        reloaded
            .markdown(&home.id)
            .unwrap()
            .contains("src/shell.rs:12:3")
    );
}

#[test]
fn failed_persistence_should_preserve_previous_feedback_and_event_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("store");
    let store = Store::open(parent.join("feedback.json")).unwrap();
    let session = store.ensure_session("Test", "/").unwrap();
    let revision = store.revision().unwrap();
    std::fs::remove_dir_all(&parent).unwrap();
    std::fs::write(&parent, "blocks the directory").unwrap();
    assert!(store.add(&session.id, feedback()).is_err());
    assert_eq!(store.revision().unwrap(), revision);
    assert!(store.session(&session.id).unwrap().annotations.is_empty());
}

#[test]
fn invalid_feedback_should_not_create_events() {
    let store = Store::memory();
    let session = store.ensure_session("Test", "/").unwrap();
    for bad in ["", "  ", &"x".repeat(8001)] {
        let mut input = feedback();
        input.comment = bad.into();
        assert!(matches!(
            store.add(&session.id, input),
            Err(Error::Invalid(_))
        ));
    }
    let mut input = feedback();
    input.x = f32::NAN;
    assert!(store.add(&session.id, input).is_err());
    let mut input = feedback();
    input.bounding_box.width = -1.0;
    assert!(store.add(&session.id, input).is_err());
    assert_eq!(store.revision().unwrap(), 1);
}

#[test]
fn statuses_should_control_pending_without_losing_the_comment() {
    let store = Store::memory();
    let session = store.ensure_session("Test", "/").unwrap();
    let note = store.add(&session.id, feedback()).unwrap();
    assert_eq!(store.pending(None).unwrap().len(), 1);
    for status in [
        AnnotationStatus::Acknowledged,
        AnnotationStatus::Resolved,
        AnnotationStatus::Dismissed,
    ] {
        store
            .update(
                &note.id,
                AnnotationPatch {
                    status: Some(status),
                    comment: None,
                },
            )
            .unwrap();
        assert!(store.pending(None).unwrap().is_empty());
        assert_eq!(
            store.annotation(&note.id).unwrap().feedback.comment,
            note.feedback.comment
        );
    }
    store
        .update(
            &note.id,
            AnnotationPatch {
                status: Some(AnnotationStatus::Pending),
                comment: None,
            },
        )
        .unwrap();
    assert_eq!(store.pending(Some(&session.id)).unwrap().len(), 1);
    store.delete(&note.id).unwrap();
    assert!(matches!(store.annotation(&note.id), Err(Error::NotFound)));
}

#[test]
fn concurrent_writers_should_keep_all_notes_in_event_order() {
    let store = Store::memory();
    let session = store.ensure_session("Test", "/").unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..10 {
                    store.add(&session.id, feedback()).unwrap();
                }
            });
        }
    });
    assert_eq!(store.session(&session.id).unwrap().annotations.len(), 80);
    let events = store.replay(0).unwrap();
    assert!(events.windows(2).all(|pair| pair[1].id == pair[0].id + 1));
}

#[test]
fn replay_should_bound_history_and_report_an_expired_cursor() {
    let store = Store::memory();
    let session = store.ensure_session("Test", "/").unwrap();
    let note = store.add(&session.id, feedback()).unwrap();
    for _ in 0..520 {
        store
            .update(
                &note.id,
                AnnotationPatch {
                    comment: Some("Padding".into()),
                    status: None,
                },
            )
            .unwrap();
    }
    assert!(store.replay(0).is_err());
    assert_eq!(store.replay(10).unwrap().len(), 512);
    assert_eq!(store.replay(521).unwrap().len(), 1);
}

#[test]
fn corrupt_storage_should_be_reported_without_overwriting_it() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("feedback.json");
    std::fs::write(&path, b"broken json").unwrap();
    assert!(Store::open(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"broken json");
}

#[test]
fn annotation_numbers_should_survive_deletion_and_restart_without_reuse() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("feedback.json");
    let store = Store::open(&path).unwrap();
    let session = store.ensure_session("Test", "/").unwrap();
    let first = store.add(&session.id, feedback()).unwrap();
    let second = store.add(&session.id, feedback()).unwrap();
    assert_eq!((first.index, second.index), (1, 2));
    store.delete(&second.id).unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    assert_eq!(store.next_index(&session.id).unwrap(), 3);
    assert_eq!(store.add(&session.id, feedback()).unwrap().index, 3);
    assert_eq!(store.annotation(&first.id).unwrap().index, 1);
}

#[test]
fn legacy_feedback_should_get_numbers_without_losing_comments_or_threads() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("feedback.json");
    let store = Store::open(&path).unwrap();
    let session = store.ensure_session("Test", "/").unwrap();
    let first = store.add(&session.id, feedback()).unwrap();
    store.reply(&first.id, "agent", "Existing reply").unwrap();
    store.add(&session.id, feedback()).unwrap();
    drop(store);
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    legacy.as_object_mut().unwrap().remove("next_indexes");
    for note in legacy["sessions"][0]["annotations"].as_array_mut().unwrap() {
        note.as_object_mut().unwrap().remove("index");
        note.as_object_mut().unwrap().remove("shape");
    }
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let store = Store::open(path).unwrap();
    let annotations = store.session(&session.id).unwrap().annotations;
    assert_eq!(
        annotations.iter().map(|a| a.index).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(annotations[0].thread[0].message, "Existing reply");
    assert_eq!(annotations[0].feedback.comment, "Add vertical padding");
    assert_eq!(store.next_index(&session.id).unwrap(), 3);
}

#[test]
fn saved_shapes_should_be_available_to_api_clients_and_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("feedback.json");
    let store = Store::open(&path).unwrap();
    let session = store.ensure_session("Test", "/").unwrap();
    let mut input = feedback();
    input.shape = Some(crate::AnnotationShape::Arrow {
        from: (0.1, 0.2),
        to: (0.8, 0.7),
        color: 2,
    });
    let note = store.add(&session.id, input).unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    let value = serde_json::to_value(store.annotation(&note.id).unwrap()).unwrap();
    assert_eq!(value["shape"]["type"], "arrow");
    assert_eq!(value["index"], 1);
    assert!(
        store
            .markdown(&session.id)
            .unwrap()
            .contains("\"type\":\"arrow\"")
    );
}

#[test]
fn app_feedback_should_include_every_route_and_status_without_other_apps() {
    let store = Store::memory();
    let home = store.ensure_session("Corpora", "gpui://corpora/").unwrap();
    let explore = store
        .ensure_session("Corpora", "gpui://corpora/explore")
        .unwrap();
    let other = store
        .ensure_session("Another app", "gpui://other/")
        .unwrap();
    let mut input = feedback();
    input.comment = "Home spacing".into();
    let home_note = store.add(&home.id, input).unwrap();
    let mut input = feedback();
    input.comment = "Search spacing".into();
    input.shape = Some(crate::AnnotationShape::Arrow {
        from: (0.1, 0.2),
        to: (0.5, 0.6),
        color: 2,
    });
    let search_note = store.add(&explore.id, input).unwrap();
    store
        .update(
            &home_note.id,
            AnnotationPatch {
                status: Some(AnnotationStatus::Resolved),
                comment: None,
            },
        )
        .unwrap();
    store
        .update(
            &search_note.id,
            AnnotationPatch {
                status: Some(AnnotationStatus::Dismissed),
                comment: None,
            },
        )
        .unwrap();
    store
        .reply(&search_note.id, "agent", "Keep the original target")
        .unwrap();
    let mut input = feedback();
    input.comment = "Unrelated application note".into();
    store.add(&other.id, input).unwrap();

    let notes = store.app_annotations("Corpora").unwrap();
    assert_eq!(
        notes
            .iter()
            .map(|note| note.id.as_str())
            .collect::<Vec<_>>(),
        vec![home_note.id.as_str(), search_note.id.as_str()]
    );
    let markdown = store.app_markdown("Corpora").unwrap();
    for expected in [
        "## Route: gpui://corpora/",
        "## Route: gpui://corpora/explore",
        "Home spacing",
        "Search spacing",
        "Resolved",
        "Dismissed",
        "src/shell.rs:12:3",
        "Keep the original target",
        "Shape: `{\"type\":\"arrow\"",
    ] {
        assert!(markdown.contains(expected), "Missing {expected}");
    }
    assert!(!markdown.contains("Unrelated application note"));
    assert!(!store.markdown(&home.id).unwrap().contains("Search spacing"));

    store
        .update(
            &search_note.id,
            AnnotationPatch {
                comment: Some("Updated from another route".into()),
                status: None,
            },
        )
        .unwrap();
    assert_eq!(
        store.annotation(&search_note.id).unwrap().session_id,
        explore.id
    );
    assert_eq!(
        store.annotation(&search_note.id).unwrap().index,
        search_note.index
    );
    assert_eq!(store.session(&home.id).unwrap().annotations.len(), 1);
    assert!(
        store
            .app_markdown("Corpora")
            .unwrap()
            .contains("Updated from another route")
    );
    store.delete(&search_note.id).unwrap();
    assert_eq!(store.app_annotations("Corpora").unwrap().len(), 1);
    assert!(
        !store
            .app_markdown("Corpora")
            .unwrap()
            .contains("Updated from another route")
    );
}

#[test]
fn undo_rejects_concurrent_replies_and_restores_deleted_identity() {
    let store = Store::memory();
    let session = store.ensure_session("Undo", "gpui://undo").unwrap();
    let before = store.add(&session.id, feedback()).unwrap();
    let mut after = before.clone();
    after.feedback.comment = "Edited".into();
    store.replace_note(Some(&before), Some(&after)).unwrap();
    store.reply(&after.id, "agent", "Concurrent reply").unwrap();
    assert!(store.replace_note(Some(&after), Some(&before)).is_err());
    let latest = store.annotation(&after.id).unwrap();
    assert_eq!(latest.thread.len(), 1);
    store.replace_note(Some(&latest), None).unwrap();
    store.replace_note(None, Some(&latest)).unwrap();
    assert_eq!(store.annotation(&latest.id).unwrap(), latest);
    assert!(store.next_index(&session.id).unwrap() > latest.index);
}

#[test]
fn toolbar_preference_survives_restart_and_is_shared_across_routes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let store = Store::open(&path).unwrap();
    let preference = ToolbarPreference {
        position: ToolbarPosition::None,
        at: Some((123., 456.)),
    };
    store.set_toolbar_preference("App", preference).unwrap();
    let other_window = store.clone();
    other_window
        .ensure_session("App", "gpui://another-route")
        .unwrap();
    assert_eq!(other_window.toolbar_preference("App").unwrap(), preference);
    assert_eq!(
        store.toolbar_preference("Other app").unwrap(),
        ToolbarPreference::default()
    );
    drop(store);
    drop(other_window);
    assert_eq!(
        Store::open(path)
            .unwrap()
            .toolbar_preference("App")
            .unwrap(),
        preference
    );
}

#[test]
fn textbox_validation_and_restart_preserve_plain_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.json");
    let store = Store::open(&path).unwrap();
    let session = store.ensure_session("Text", "gpui://text").unwrap();
    let mut input = feedback();
    input.shape = Some(crate::AnnotationShape::Text {
        at: (0.2, 0.4),
        text: "Line 1\nLine 2".into(),
        color: 3,
    });
    let note = store.add(&session.id, input.clone()).unwrap();
    assert_eq!(
        Store::open(&path).unwrap().annotation(&note.id).unwrap(),
        note
    );
    input.shape = Some(crate::AnnotationShape::Text {
        at: (f32::NAN, 0.4),
        text: "Invalid".into(),
        color: 0,
    });
    assert!(store.add(&session.id, input).is_err());
}

#[test]
fn api_comment_updates_change_the_visible_textbox_text() {
    let store = Store::memory();
    let session = store.ensure_session("Text", "gpui://text").unwrap();
    let mut input = feedback();
    input.comment = "Original".into();
    input.shape = Some(crate::AnnotationShape::Text {
        at: (0.2, 0.4),
        text: "Original".into(),
        color: 2,
    });
    let note = store.add(&session.id, input).unwrap();
    let updated = store
        .update(
            &note.id,
            AnnotationPatch {
                comment: Some("Updated via MCP".into()),
                status: None,
            },
        )
        .unwrap();
    assert_eq!(updated.feedback.comment, "Updated via MCP");
    assert!(
        matches!(&updated.feedback.shape, Some(crate::AnnotationShape::Text {
        text, at: (0.2, 0.4), color: 2,
    }) if text == "Updated via MCP")
    );
    assert_eq!(store.annotation(&note.id).unwrap(), updated);
}
