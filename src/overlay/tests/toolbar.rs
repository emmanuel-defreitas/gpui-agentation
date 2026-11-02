//! Companion toolbar window, inspection controls and surface actions.
use super::super::{
    surface::{row_action, tool_cursor},
    toolbar_window::dock_bounds,
};
use super::*;
use crate::ely::media::Tool;
use gpui::{
    Bounds, Context, FocusHandle, Modifiers, Render, TestAppContext, VisualTestContext,
    WindowVisibility, div, point, px, size,
};
use std::{cell::Cell, rc::Rc};

fn click_toolbar(
    overlay: &Entity<AnnotationOverlay>,
    selector: &'static str,
    cx: &mut VisualTestContext,
) {
    let mut tools = toolbar(overlay, cx);
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
    // GPUI draws the inspector in the host window; the host grows to hold it.
    assert!(cx.debug_bounds("inspector-picking-true").is_some());
    let mut tools = toolbar(&overlay, cx);
    assert!(tools.debug_bounds("inspector-picking-true").is_none());
    assert!(cx.debug_bounds("banner-devtools").is_none());
    assert!(cx.debug_bounds("floating-devtools").is_none());
    assert!(cx.debug_bounds("feedback-picker").is_none());
}

#[gpui::test]
fn comments_should_render_in_the_companion_with_a_filling_list(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|_, cx| crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
    cx.update(|_, cx| {
        overlay.update(cx, |this, _| {
            this.store
                .add(
                    &this.session.id,
                    AnnotationInput {
                        inspection: None,
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
    let mut tools = toolbar(&overlay, cx);
    assert!(tools.debug_bounds("feedback-annotation-count").is_some());
    click_toolbar(&overlay, "feedback-floating-comments", cx);
    draw(cx);
    let mut comments = toolbar(&overlay, cx);
    assert!(comments.debug_bounds("companion-comments-panel").is_some());
    let header = comments.debug_bounds("feedback-sheet-header").unwrap();
    let list = comments.debug_bounds("feedback-comments-list").unwrap();
    let add = comments.debug_bounds("feedback-comments-add").unwrap();
    let footer = comments.debug_bounds("feedback-comments-footer").unwrap();
    let body = comments.debug_bounds("feedback-sheet-body").unwrap();
    let scroll = comments.debug_bounds("feedback-comments-scroll").unwrap();
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
    assert!(footer.top() > header.top());
}

#[gpui::test]
fn escape_keeps_inspection_mode_active(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    draw(cx);
    click_toolbar(&overlay, "companion-tool-i", cx);
    draw(cx);
    cx.update(|window, cx| {
        assert!(
            !overlay.read(cx).active,
            "inspection is independent of drawing mode"
        );
        assert!(window.is_inspector_picking(cx));
    });
    cx.simulate_keystrokes("b");
    draw(cx);
    cx.update(|window, cx| {
        assert_eq!(overlay.read(cx).tool, Some(Tool::Box));
        assert!(inspector::is_open(window, cx));
        assert!(window.is_inspector_picking(cx));
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).active);
        assert!(window.is_inspector_picking(cx));
        assert!(inspector::is_open(window, cx));
    });
}

#[gpui::test]
fn closing_settings_should_resume_inspection_with_the_stop_control(cx: &mut TestAppContext) {
    let (overlay, cx) = overlay(cx);
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(crate::ToolbarPosition::Floating, cx);
            this.choose_tool(None, window, cx);
        })
    });
    draw(cx);
    let mut tools = toolbar(&overlay, cx);
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
        assert!(!window.is_inspector_picking(cx));
    });
    cx.update(|window, cx| assert!(!inspector::is_open(window, cx)));
}

fn setup(cx: &mut TestAppContext) -> (Entity<AnnotationOverlay>, &mut VisualTestContext) {
    cx.update(crate::ely::init_for_tests);
    let (overlay, cx) = cx.add_window_view(|window, cx| {
        let store = Store::memory();
        let session = store.ensure_session("Companion", "gpui://toolbar").unwrap();
        AnnotationOverlay::new(store, session, 0, window, cx)
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(size(px(900.), px(650.)));
    draw(cx);
    (overlay, cx)
}

#[test]
fn dock_tracks_host_edges_and_height_with_fourteen_pixel_gap() {
    for host in [
        Bounds::new(point(px(100.), px(80.)), size(px(900.), px(650.))),
        Bounds::new(point(px(-1250.), px(310.)), size(px(700.), px(500.))),
    ] {
        let dock = dock_bounds(host, px(50.));
        assert_eq!(dock.left() - host.right(), px(14.));
        assert_eq!(dock.top(), host.top());
        assert_eq!(dock.bottom(), host.bottom());
        assert_eq!(dock.size.width, px(50.));
    }
}

#[gpui::test]
fn expanded_comments_panel_is_380_wide(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    cx.update(|window, cx| {
        overlay.update(cx, |this, cx| {
            this.review_open = true;
            this.sync_toolbar_window(window, cx);
        });
    });
    draw(cx);
    let mut tools = toolbar(&overlay, cx);
    let rail = tools.debug_bounds("feedback-native-toolbar").unwrap();
    let panel = tools.debug_bounds("companion-comments-panel").unwrap();
    assert_eq!(panel.size.width, px(380.));
    tools.update(|window, _| {
        assert_eq!(
            window.bounds().size.width,
            rail.size.width + px(2.) + px(380.)
        );
    });
}

#[gpui::test]
fn companion_is_permanent_vertical_and_changes_mode_in_the_host(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    assert!(cx.debug_bounds("feedback-floating-toolbar").is_none());
    assert!(cx.debug_bounds("feedback-banner").is_none());
    let mut tools = toolbar(&overlay, cx);
    let rail = tools.debug_bounds("feedback-native-toolbar").unwrap();
    let inspect = tools.debug_bounds("companion-tool-i").unwrap();
    let toggle = tools.debug_bounds("feedback-annotation-toggle").unwrap();
    let divider = tools.debug_bounds("feedback-tool-divider").unwrap();
    let box_tool = tools.debug_bounds("companion-tool-b").unwrap();
    let comments = tools.debug_bounds("feedback-floating-comments").unwrap();
    let settings = tools
        .debug_bounds("feedback-settings-trigger-target")
        .unwrap();
    assert!(tools.debug_bounds("feedback-toolbar-close").is_none());
    assert!(toggle.bottom() <= inspect.top());
    // The switch stands apart from the tools below it.
    assert!(
        inspect.top() - toggle.bottom() >= px(12.),
        "space below the mode switch"
    );
    // A visible rule separates Inspect from the drawing tools.
    assert!(divider.size.width >= rail.size.width / 2.);
    assert!(divider.size.height > px(0.));
    assert!(inspect.bottom() <= box_tool.top());
    assert!(inspect.bottom() <= divider.top());
    assert!(divider.bottom() <= box_tool.top());
    assert!(comments.bottom() <= settings.top());
    // Disabled drawing tools must not enable annotation mode.
    tools.simulate_click(box_tool.center(), Modifiers::none());
    draw(cx);
    cx.update(|_, cx| assert!(!overlay.read(cx).active));
    tools.simulate_click(inspect.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(window.is_inspector_picking(cx));
        assert!(!overlay.read(cx).active);
    });
    // While inspecting, the Inspect button stays as the way out: GPUI swallows
    // host clicks during picking, so Stop must live in the companion.
    let mut tools = toolbar(&overlay, cx);
    let stop = tools.debug_bounds("companion-tool-i").unwrap();
    assert!(tools.debug_bounds("companion-tool-b").is_some());
    tools.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(!window.is_inspector_picking(cx));
        assert!(!overlay.read(cx).active);
        assert!(overlay.read(cx).companion_toolbar.is_some());
    });
}

#[gpui::test]
fn companion_follows_resize_visibility_fullscreen_and_host_close(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.simulate_resize(size(px(760.), px(530.)));
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        let spec = this.companion_toolbar_spec.as_ref().unwrap();
        assert_eq!(
            spec.bounds,
            dock_bounds(window.bounds(), spec.bounds.size.width)
        );
    });
    let mut tools = toolbar(&overlay, cx);
    tools.update(|window, _| assert_eq!(window.viewport_size().height, px(530.)));
    cx.simulate_visibility_change(WindowVisibility::Hidden);
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(
            !overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.simulate_visibility_change(WindowVisibility::Visible);
    draw(cx);
    cx.update(|_, cx| {
        assert!(
            overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.update(|window, cx| {
        window.toggle_fullscreen();
        overlay.update(cx, |this, cx| this.sync_toolbar_window(window, cx));
    });
    cx.executor().advance_clock(Duration::from_millis(31));
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert!(
            !overlay
                .read(cx)
                .companion_toolbar_spec
                .as_ref()
                .unwrap()
                .visible
        )
    });
    cx.update(|window, cx| {
        window.toggle_fullscreen();
        overlay.update(cx, |this, cx| this.sync_toolbar_window(window, cx));
    });
    draw(cx);
    assert!(cx.update(|_, cx| {
        overlay
            .read(cx)
            .companion_toolbar_spec
            .as_ref()
            .unwrap()
            .visible
    }));
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    cx.cx.update(|cx| {
        assert!(overlay.read(cx).companion_toolbar.is_none());
        assert!(cx.windows().is_empty());
    });
}

struct ActionRow {
    focus: FocusHandle,
    actions: Rc<Cell<usize>>,
    row_presses: Rc<Cell<usize>>,
    row_keys: Rc<Cell<usize>>,
}

impl Render for ActionRow {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        let actions = self.actions.clone();
        let row = self.row_presses.clone();
        let down = self.row_keys.clone();
        let up = self.row_keys.clone();
        div()
            .id("selectable-row")
            .size_full()
            .on_click(move |_, _, _| row.set(row.get() + 1))
            .on_key_down(move |_, _, _| down.set(down.get() + 1))
            .on_key_up(move |_, _, _| up.set(up.get() + 1))
            .child(row_action(
                div()
                    .id("row-action")
                    .size(px(32.0))
                    .track_focus(&self.focus)
                    .on_click(move |_, _, _| actions.set(actions.get() + 1)),
            ))
    }
}

#[gpui::test]
fn trailing_actions_should_keep_mouse_and_keyboard_activation_out_of_the_row(
    cx: &mut TestAppContext,
) {
    let actions = Rc::new(Cell::new(0));
    let row = Rc::new(Cell::new(0));
    let keys = Rc::new(Cell::new(0));
    let (view, cx) = cx.add_window_view(|_, cx| ActionRow {
        focus: cx.focus_handle().tab_stop(true),
        actions: actions.clone(),
        row_presses: row.clone(),
        row_keys: keys.clone(),
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(16.0), px(16.0)), Modifiers::none());
    assert_eq!(actions.get(), 1);
    assert_eq!(row.get(), 0);
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    for key in ["enter", "space"] {
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse(key).unwrap(),
        });
    }
    assert_eq!(actions.get(), 3);
    assert_eq!(row.get(), 0);
    assert_eq!(keys.get(), 0);
}

#[test]
fn inspect_should_use_normal_cursor_and_arrow_drawing_should_use_crosshair() {
    assert_eq!(tool_cursor(None), gpui::CursorStyle::Arrow);
    assert_eq!(
        tool_cursor(Some(crate::ely::media::Tool::Arrow)),
        gpui::CursorStyle::Crosshair
    );
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
        window.activate_window();
        let overlay = host.read(cx).overlay.clone();
        overlay.update(cx, |this, cx| {
            this.set_toolbar_position(crate::ToolbarPosition::Floating, cx);
            this.tool = Some(Tool::Pen);
            this.choose_tool(None, window, cx);
        });
        overlay
    });
    draw(cx);
    let target = cx.debug_bounds("host-matching-toolbar-id").unwrap();
    cx.simulate_mouse_move(target.center(), None, Modifiers::none());
    cx.simulate_click(target.center(), Modifiers::none());
    draw(cx);
    // The inspector panel renders in the host window, which syncs the selection.
    draw(cx);
    cx.update(|window, cx| {
        let this = overlay.read(cx);
        assert_eq!(
            this.tool,
            Some(Tool::Pen),
            "a host element with a toolbar-like id must not change the tool"
        );
        assert!(this.inspected_id.is_some());
        assert_eq!(this.session.annotations.len(), 1);
        assert!(this.session.annotations[0].feedback.inspection.is_some());
        assert!(inspector::is_open(window, cx));
        assert!(!window.is_inspector_picking(cx));
    });
}

#[gpui::test]
fn without_a_native_margin_the_panel_stays_in_the_host_and_the_window_keeps_its_size(
    cx: &mut TestAppContext,
) {
    let (overlay, cx) = setup(cx);
    let before = cx.update(|window, _| window.bounds());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    cx.update(|window, cx| {
        assert!(window.is_inspector_picking(cx));
        assert!(!inspector::is_external(window, cx));
        assert_eq!(window.bounds(), before);
    });
    assert!(cx.debug_bounds("inspector-header").is_some());
    let mut tools = toolbar(&overlay, cx);
    assert!(tools.debug_bounds("companion-panel").is_none());
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.close(window, cx)));
    draw(cx);
    cx.update(|window, cx| assert!(!window.is_inspector_picking(cx)));
}

/// The companion after TestWindow::resize, which skips the native resize
/// callback: deliver it so its viewport matches the requested width.
fn grown_toolbar(
    overlay: &Entity<AnnotationOverlay>,
    cx: &mut VisualTestContext,
) -> VisualTestContext {
    let mut tools = toolbar(overlay, cx);
    let requested = tools.update(|window, _| window.bounds().size);
    tools.simulate_resize(requested);
    draw(&mut tools);
    tools
}

#[gpui::test]
fn with_the_sidebar_out_of_sight_the_companion_renders_the_inspector(cx: &mut TestAppContext) {
    let (overlay, cx) = setup(cx);
    cx.update(|_, cx| {
        crate::ely::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
        cx.set_global(super::super::toolbar_window::SimulateHiddenMargin);
    });
    let narrow = {
        let mut tools = toolbar(&overlay, cx);
        tools.update(|window, _| window.bounds().size.width)
    };
    cx.update(|window, cx| overlay.update(cx, |this, cx| this.choose_tool(None, window, cx)));
    draw(cx);
    draw(cx);
    cx.update(|window, cx| {
        assert!(window.is_inspector_picking(cx));
        assert!(inspector::is_external(window, cx));
    });
    // The host draws nothing in GPUI's sidebar; the companion shows the panel.
    assert!(cx.debug_bounds("inspector-header").is_none());
    let mut tools = grown_toolbar(&overlay, cx);
    assert!(tools.debug_bounds("companion-panel").is_some());
    assert!(tools.debug_bounds("inspector-picking-true").is_some());
    tools.update(|window, _| assert_eq!(window.bounds().size.width, narrow + px(420.)));
    // Picking an element in the host updates the companion and records a note.
    let host = cx.debug_bounds("feedback-inline-root").unwrap();
    let at = point(host.left() + px(40.), host.top() + px(40.));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    draw(cx);
    draw(cx);
    let mut tools = grown_toolbar(&overlay, cx);
    assert!(tools.debug_bounds("inspector-picking-false").is_some());
    assert!(tools.debug_bounds("feedback-inspect-stop").is_some());
    cx.update(|_, cx| {
        let this = overlay.read(cx);
        assert!(this.inspected_id.is_some());
        assert_eq!(this.session.annotations.len(), 1);
    });
    // Stop from the companion's panel header closes the panel.
    let stop = tools.debug_bounds("feedback-inspect-stop").unwrap();
    tools.simulate_click(stop.center(), Modifiers::none());
    draw(cx);
    cx.update(|window, cx| {
        assert!(!overlay.read(cx).inspecting);
        assert!(!window.is_inspector_picking(cx));
        assert!(!inspector::is_external(window, cx));
    });
    let mut tools = toolbar(&overlay, cx);
    assert!(tools.debug_bounds("companion-panel").is_none());
    tools.update(|window, _| assert_eq!(window.bounds().size.width, narrow));
}
