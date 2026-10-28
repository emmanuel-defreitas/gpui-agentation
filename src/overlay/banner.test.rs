use super::*;
use crate::ely::media::Tool;
use gpui::{Modifiers, TestAppContext, VisualTestContext};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

fn palette(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) -> VisualTestContext {
    let handle = cx.update(|_, cx| {
        overlay
            .read(cx)
            .companion_toolbar
            .expect("inspection palette")
    });
    let mut palette = VisualTestContext::from_window(handle.into(), &cx.cx);
    draw(&mut palette);
    palette
}

fn click_toolbar(
    overlay: &Entity<AnnotationOverlay>,
    selector: &'static str,
    cx: &mut VisualTestContext,
) {
    let mut tools = palette(overlay, cx);
    let target = tools.debug_bounds(selector).unwrap();
    tools.simulate_click(target.center(), Modifiers::none());
    draw(cx);
}

fn overlay(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let store = Store::memory();
    let session = store
        .ensure_session("UI integration test", "gpui://test")
        .unwrap();
    let (overlay, cx) =
        cx.add_window_view(|window, cx| AnnotationOverlay::new(store, session, 0, window, cx));
    cx.update(|window, _| window.activate_window());
    (overlay, cx)
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
fn settings_trigger_should_open_sheet_and_escape_should_restore_focus(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|window, cx| window.focus(&overlay.read(cx).focus.clone(), cx));
    draw(cx);
    click_toolbar(&overlay, "feedback-settings-trigger-target", cx);
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
fn inspect_command_should_open_the_single_sidebar(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.dispatch_command(
                &commands::AnnotationCommand {
                    name: "devtools".into(),
                },
                window,
                cx,
            );
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("inspector-picking-true").is_some());
    assert!(cx.debug_bounds("banner-devtools").is_none());
    assert!(cx.debug_bounds("floating-devtools").is_none());
    assert!(cx.debug_bounds("feedback-picker").is_none());
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
                        gpui_configuration: None,
                        shape: None,
                    },
                )
                .unwrap();
        })
    });
    draw(cx);
    let mut tools = palette(&overlay, cx);
    assert!(tools.debug_bounds("feedback-annotation-count").is_some());
    click_toolbar(&overlay, "feedback-floating-comments", cx);
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
fn inspect_should_start_picking_and_escape_should_exit(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    draw(cx);
    click_toolbar(&overlay, "companion-tool-i", cx);
    draw(cx);
    cx.update(|window, cx| {
        assert!(overlay.read(cx).active);
        assert!(window.is_inspector_picking(cx));
    });
    cx.simulate_keystrokes("b");
    draw(cx);
    cx.update(|window, cx| {
        assert_eq!(overlay.read(cx).tool, Some(Tool::Box));
        assert!(!inspector::is_open(window, cx));
    });
    cx.simulate_keystrokes("i");
    draw(cx);
    cx.update(|window, cx| assert!(window.is_inspector_picking(cx)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).active);
        assert!(!inspector::is_open(window, cx));
    });
}

#[gpui::test]
fn inspect_selection_should_show_properties_and_optional_comment_in_sidebar(
    cx: &mut TestAppContext,
) {
    cx.update(crate::ely::init_for_tests);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Sidebar test", "gpui://sidebar")
            .unwrap();
        Shell {
            focus: cx.focus_handle(),
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|_, cx| shell.read(cx).overlay.clone());
    draw(cx);
    click_toolbar(&overlay, "companion-tool-i", cx);
    draw(cx);
    let content = cx.debug_bounds("app-content").unwrap();
    cx.simulate_mouse_move(content.center(), None, Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-inspector-overlay").is_none());
    cx.simulate_click(content.center(), Modifiers::none());
    draw(cx);
    assert!(cx.debug_bounds("feedback-inspector-sidebar").is_some());
    assert!(cx.debug_bounds("feedback-editor-panel").is_none());
    assert!(
        cx.debug_bounds("inspector-config-ely-inspector-width")
            .is_some()
    );
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert!(this.inspected_id.is_some());
        assert!(
            this.draft
                .as_ref()
                .unwrap()
                .target
                .source
                .contains("banner.test.rs"),
            "inspect the host, not annotation overlays"
        );
        assert!(this.draft.as_ref().unwrap().target.id.starts_with("gpui:"));
        assert!(
            !this.input.focus_handle(cx).is_focused(window),
            "comment is optional"
        );
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("feedback-inspector-sidebar").is_none());
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).active);
        assert!(!inspector::is_open(window, cx));
    });
}

#[gpui::test]
fn sidebar_style_only_feedback_should_save_update_reset_and_stay_out_of_drawings(
    cx: &mut TestAppContext,
) {
    cx.update(crate::ely::init_for_tests);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("Style feedback", "gpui://style")
            .unwrap();
        Shell {
            focus: cx.focus_handle(),
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|window, cx| {
        let overlay = shell.read(cx).overlay.clone();
        overlay.update(cx, |this, cx| this.choose_tool(None, window, cx));
        overlay
    });
    draw(cx);
    let content = cx.debug_bounds("app-content").unwrap();
    cx.simulate_mouse_move(content.center(), None, Modifiers::none());
    cx.simulate_click(content.center(), Modifiers::none());
    draw(cx);
    let alternate = std::cell::Cell::new(false);
    let increment_width = |cx: &mut VisualTestContext| {
        let track = cx
            .debug_bounds("inspector-slider-ely-inspector-width")
            .unwrap();
        let fraction = if alternate.replace(!alternate.get()) {
            0.5
        } else {
            0.25
        };
        cx.simulate_click(
            gpui::point(track.left() + track.size.width * fraction, track.center().y),
            Modifiers::none(),
        );
        draw(cx);
    };
    increment_width(cx);
    cx.update(|_, cx| {
        assert!(overlay.read(cx).input.read(cx).text().is_empty());
        assert!(overlay.read(cx).inspected_configuration.is_some());
    });
    let save = cx.debug_bounds("feedback-inspector-save").unwrap();
    cx.simulate_click(save.center(), Modifiers::none());
    draw(cx);
    let id = cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(this.session.annotations.len(), 1);
        let note = &this.session.annotations[0];
        assert!(note.feedback.comment.is_empty());
        assert!(
            note.feedback
                .gpui_configuration
                .as_ref()
                .unwrap()
                .rust
                .contains(".w(px(")
        );
        note.id.clone()
    });
    let reset = cx.debug_bounds("inspector-reset").unwrap();
    cx.simulate_click(reset.center(), Modifiers::none());
    draw(cx);
    assert!(
        cx.debug_bounds("feedback-inspector-delete").is_some(),
        "style-only feedback has an explicit removal action after Reset"
    );
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.inspected_configuration.is_none());
        assert!(
            this.store
                .annotation(&id)
                .unwrap()
                .feedback
                .gpui_configuration
                .is_some(),
            "Reset changes the preview; saved feedback persists until explicitly changed"
        );
    });
    increment_width(cx);
    let comment = cx.debug_bounds("feedback-inspector-comment").unwrap();
    cx.simulate_click(comment.center(), Modifiers::none());
    cx.update(|window, cx| {
        assert!(
            overlay.read(cx).input.focus_handle(cx).is_focused(window),
            "clicking the larger field focuses its input"
        )
    });
    cx.simulate_input("Wider content");
    draw(cx);
    let save = cx.debug_bounds("feedback-inspector-save").unwrap();
    cx.simulate_click(save.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert_eq!(
            this.session.annotations.len(),
            1,
            "update the same feedback"
        );
        let note = this.store.annotation(&id).unwrap();
        assert_eq!(note.feedback.comment, "Wider content");
        assert_eq!(
            note.feedback.gpui_configuration,
            this.inspected_configuration
        );
        assert!(
            this.store
                .markdown(&this.session.id)
                .unwrap()
                .contains("GPUI configuration")
        );
    });
    let reset = cx.debug_bounds("inspector-reset").unwrap();
    cx.simulate_click(reset.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).inspected_configuration.is_none()));
    let save = cx.debug_bounds("feedback-inspector-save").unwrap();
    cx.simulate_click(save.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| {
        assert!(
            overlay
                .read(cx)
                .store
                .annotation(&id)
                .unwrap()
                .feedback
                .gpui_configuration
                .is_none()
        )
    });
    increment_width(cx);
    // Property inputs retain typed keys; explicitly choose the drawing tool.
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.dispatch_command(
                &commands::AnnotationCommand { name: "box".into() },
                window,
                cx,
            )
        })
    });
    draw(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.select(
                Target {
                    id: "drawing".into(),
                    label: "Drawing".into(),
                    source: String::new(),
                    bounds: crate::Rect::default(),
                },
                20.,
                20.,
                window,
                cx,
            );
            this.input
                .update(cx, |input, cx| input.set_text("Independent feedback", cx));
            assert!(this.save(window, cx));
            assert!(
                this.session
                    .annotations
                    .last()
                    .unwrap()
                    .feedback
                    .gpui_configuration
                    .is_none()
            );
        })
    });
}

struct HostWithMatchingToolbarIds {
    overlay: Entity<AnnotationOverlay>,
}

impl Render for HostWithMatchingToolbarIds {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                div()
                    .id("feedback-floating-toolbar")
                    .size(gpui::px(100.))
                    .child(
                        div()
                            .id(("feedback-tool", 1usize))
                            .size(gpui::px(80.))
                            .debug_selector(|| "host-matching-toolbar-id".into()),
                    ),
            )
            .child(self.overlay.clone())
    }
}

#[gpui::test]
fn host_elements_with_matching_toolbar_ids_should_remain_inspectable(cx: &mut TestAppContext) {
    cx.update(crate::ely::init_for_tests);
    let (host, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Host IDs", "gpui://ids").unwrap();
        HostWithMatchingToolbarIds {
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    let overlay = cx.update(|window, cx| {
        let overlay = host.read(cx).overlay.clone();
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(crate::ToolbarPosition::Floating, cx);
            this.toggle(window, cx);
        });
        overlay
    });
    draw(cx);
    let target = cx.debug_bounds("host-matching-toolbar-id").unwrap();
    cx.simulate_mouse_move(target.center(), None, Modifiers::none());
    cx.simulate_click(target.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert!(this.tool.is_none(), "a host element must not dispatch Box");
        assert!(this.inspected_id.is_some());
        assert!(this.draft.is_some());
        assert!(inspector::is_open(window, cx));
        assert!(!window.is_inspector_picking(cx));
    });
}

#[gpui::test]
fn closing_settings_should_resume_inspection_with_the_stop_control(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(crate::ToolbarPosition::Floating, cx);
            this.toggle(window, cx);
        })
    });
    draw(cx);
    let mut tools = palette(&overlay, cx);
    let settings = tools
        .debug_bounds("feedback-settings-trigger-target")
        .unwrap();
    tools.simulate_click(settings.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(overlay.read(cx).settings_open));
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).settings_open);
        assert!(window.is_inspector_picking(cx));
    });
    let mut tools = palette(&overlay, cx);
    let stop = tools.debug_bounds("companion-tool-i").unwrap();
    tools.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| assert!(!inspector::is_open(window, cx)));
}

// The opt-in prototype keeps the host's native picker but renders its UI outside.
#[cfg(agentation_external_inspector)]
#[gpui::test]
fn external_inspector_expands_without_reflow_edits_consumer_and_collapses(cx: &mut TestAppContext) {
    fn external_palette(
        overlay: &Entity<AnnotationOverlay>,
        cx: &mut VisualTestContext,
    ) -> VisualTestContext {
        let mut view = palette(overlay, cx);
        // TestWindow::resize updates native bounds without dispatching a resize callback.
        let requested = view.update(|window, _| window.bounds().size);
        view.simulate_resize(requested);
        draw(&mut view);
        view
    }
    cx.update(crate::ely::init_for_tests);
    cx.update(|cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store
            .ensure_session("External inspector", "gpui://external")
            .unwrap();
        Shell {
            focus: cx.focus_handle(),
            overlay: cx.new(|cx| AnnotationOverlay::new(store, session, 0, window, cx)),
        }
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(650.)));
    let overlay = cx.update(|_, cx| shell.read(cx).overlay.clone());
    draw(cx);
    let original_content = cx.debug_bounds("app-content").unwrap();
    let mut tools = external_palette(&overlay, cx);
    let original_window = tools.update(|window, _| window.window_handle());
    let narrow_width = tools.update(|window, _| window.viewport_size().width);
    let inspect = tools.debug_bounds("companion-tool-i").unwrap();
    tools.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    let wide_width = tools.update(|window, _| {
        assert_eq!(window.window_handle(), original_window);
        window.viewport_size().width
    });
    assert_eq!(wide_width - narrow_width, gpui::px(420.));
    assert_eq!(cx.debug_bounds("app-content").unwrap(), original_content);
    assert!(cx.debug_bounds("inspector-header").is_none());
    assert!(tools.debug_bounds("inspector-picking-true").is_some());
    let content = cx.debug_bounds("app-content").unwrap();
    cx.simulate_mouse_move(content.center(), None, Modifiers::none());
    draw(cx);
    cx.simulate_click(content.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    assert!(tools.debug_bounds("external-inspector-feedback").is_some());
    assert!(tools.debug_bounds("inspector-picking-false").is_some());
    tools.update(|window, cx| {
        let overlay_color: gpui::Background =
            cx.theme().colors.hue(0, "inspector").alpha(0.25).into();
        assert!(
            !window
                .painted_quads()
                .iter()
                .any(|quad| quad.background == overlay_color),
            "consumer box highlight must not paint over companion controls"
        );
    });
    let comment = tools
        .debug_bounds("external-inspector-comment-scroll")
        .unwrap();
    let actions = tools
        .debug_bounds("external-inspector-feedback-actions")
        .unwrap();
    assert!(comment.size.height <= gpui::px(160.));
    tools.update(|window, _| assert!(actions.bottom() <= window.viewport_size().height));
    let track = tools
        .debug_bounds("inspector-slider-ely-inspector-opacity")
        .unwrap();
    tools.simulate_click(
        gpui::point(track.left() + track.size.width * 0.5, track.center().y),
        Modifiers::none(),
    );
    draw(&mut tools);
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    cx.update(|window, cx| {
        let selected = window
            .inspector_entity()
            .unwrap()
            .read(cx)
            .active_element_id()
            .unwrap()
            .clone();
        window
            .with_inspector_state::<gpui::DivInspectorState, _>(Some(&selected), cx, |state, _| {
                let opacity = state.as_ref().unwrap().base_style.opacity.unwrap();
                assert!(
                    opacity > 0. && opacity < 1.,
                    "slider writes the consumer style"
                );
            })
            .unwrap();
        assert!(overlay.read(cx).inspected_configuration.is_some());
    });
    let reset = tools.debug_bounds("inspector-reset").unwrap();
    tools.simulate_click(reset.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    cx.update(|_, cx| assert!(overlay.read(cx).inspected_configuration.is_none()));
    let pick = tools.debug_bounds("inspector-pick").unwrap();
    tools.simulate_click(pick.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    tools.update(|window, _| assert_eq!(window.viewport_size().width, wide_width));
    cx.update(|window, cx| assert!(window.is_inspector_picking(cx)));
    cx.simulate_mouse_move(content.center(), None, Modifiers::none());
    cx.simulate_click(content.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    cx.update(|_, cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = false));
    let (retained, host_handle) =
        cx.update(|window, _| (window.inspector_entity().unwrap(), window.window_handle()));
    let stop = tools.debug_bounds("companion-tool-i").unwrap();
    tools.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    let mut tools = external_palette(&overlay, cx);
    assert!(
        tools.debug_bounds("external-inspector-panel").is_some(),
        "keep exit content until collapsed"
    );
    cx.update(|window, cx| {
        assert!(
            overlay.read(cx).draft.is_none(),
            "exit rendering cannot recreate a draft"
        );
        assert!(crate::ely::tooling::inspector_selection(window, cx).is_none());
    });
    tools.update(|window, cx| window.simulate_next_frame(cx));
    tools.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(240));
    cx.run_until_parked();
    let mut tools = external_palette(&overlay, cx);
    tools.update(|window, _| {
        assert_eq!(window.viewport_size().width, narrow_width);
        assert_eq!(window.window_handle(), original_window);
    });
    assert!(tools.debug_bounds("external-inspector-panel").is_none());
    cx.update(|window, cx| assert!(!window.is_inspector_picking(cx)));

    // A queued frame from the old controller must stay inert after Inspect restarts.
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    struct RetainedInspector(Entity<gpui::Inspector>);
    impl Render for RetainedInspector {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.0.clone())
        }
    }
    let (_, probe) = cx.cx.add_window_view(|window, cx| {
        crate::ely::tooling::set_inspector_target(
            window.window_handle().window_id(),
            Some(host_handle),
            cx,
        );
        RetainedInspector(retained)
    });
    draw(probe);
    probe.update(|window, cx| {
        assert!(overlay.read(cx).draft.is_none());
        assert!(overlay.read(cx).inspected_id.is_none());
        host_handle
            .update(cx, |_, host, cx| {
                assert!(crate::ely::tooling::inspector_selection(host, cx).is_none());
            })
            .unwrap();
        crate::ely::tooling::set_inspector_target(window.window_handle().window_id(), None, cx);
        window.remove_window();
    });
}
