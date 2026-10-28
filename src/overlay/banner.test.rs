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
    let mut tools = palette(&overlay, cx);
    assert!(tools.debug_bounds("feedback-annotation-count").is_some());
    click_toolbar(&overlay, "feedback-floating-comments", cx);
    draw(cx);
    let mut comments = palette(&overlay, cx);
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

#[cfg(agentation_external_inspector)]
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
            this.choose_tool(None, window, cx);
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
        assert_eq!(this.session.annotations.len(), 1);
        assert!(this.session.annotations[0].feedback.inspection.is_some());
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
            this.choose_tool(None, window, cx);
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
        assert!(!window.is_inspector_picking(cx));
    });
    cx.update(|window, cx| assert!(!inspector::is_open(window, cx)));
}
