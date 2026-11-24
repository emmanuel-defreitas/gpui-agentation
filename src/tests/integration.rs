use super::*;
use gpui::component::{ActiveTheme, ThemeMode as Mode};
use gpui::{
    AppContext, Context, IntoElement, Modifiers, Render, StatefulInteractiveElement, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

struct HostAssets;
impl AssetSource for HostAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok((path == "host/logo.svg").then_some(Cow::Borrowed(b"host-logo".as_slice())))
    }

    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(vec!["host/logo.svg".into()])
    }
}

#[test]
fn asset_composition_preserves_host_assets_and_supplies_tooling_icons() {
    let assets = Assets::before(HostAssets);
    assert_eq!(
        assets.load("host/logo.svg").unwrap().unwrap().as_ref(),
        b"host-logo"
    );
    let names = assets.list("").unwrap();
    assert!(names.iter().any(|name| name.as_ref() == "host/logo.svg"));
    let icon = names
        .iter()
        .find(|name| name.ends_with("/settings.svg"))
        .unwrap();
    assert!(!assets.load(icon).unwrap().unwrap().is_empty());
}

#[gpui::test]
fn initialization_reports_missing_agentation_assets(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let error = init(cx).unwrap_err().to_string();
        assert!(error.contains("gpui_agentation::Assets"));
        assert!(!cx.has_global::<Initialized>());
    });
}

#[gpui::test]
fn initialization_preserves_existing_host_theme_and_is_idempotent(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        Theme::change(Mode::Dark, None, cx);
        let background = cx.theme().background;
        init(cx).unwrap();
        init(cx).unwrap();
        assert!(cx.theme().is_dark());
        assert_eq!(cx.theme().background, background);
    });
}

struct InspectorSample;

impl Render for InspectorSample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("inspector-sample")
                .debug_selector(|| "inspector-sample".into())
                .w(px(120.0))
                .h(px(40.0))
                .py(px(8.0))
                .px(px(16.0))
                .child("Sample"),
        )
    }
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn inspector_visually_edits_and_publishes_selected_gpui_configuration(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        init(cx).unwrap();
    });
    let (_, cx) = cx.add_window_view(|_, _| InspectorSample);
    cx.update(|window, cx| window.toggle_inspector(cx));
    cx.run_until_parked();
    let sample = cx.debug_bounds("inspector-sample").unwrap();
    cx.simulate_mouse_move(sample.center(), None, Modifiers::none());
    cx.simulate_click(sample.center(), Modifiers::none());
    redraw(cx);

    let width = cx
        .debug_bounds("inspector-slider-Width")
        .expect("width slider");
    cx.simulate_click(width.center(), Modifiers::none());
    redraw(cx);
    cx.update(|window, cx| {
        let configuration = crate::inspection::active_inspector_configuration(window, cx)
            .expect("active inspector configuration");
        assert_eq!(configuration.changes[0].property, "Width");
        let after: f32 = configuration.changes[0]
            .after
            .trim_end_matches("px")
            .parse()
            .unwrap();
        assert_ne!(after, 120.0);
        let selection = crate::inspection::inspector_selection(window, cx).unwrap();
        assert!((after - f32::from(selection.bounds.size.width)).abs() < 0.1);
        assert!(configuration.rust.contains(".w(px("));
    });
    // Returning the top side to 8 still changes the original 16px sides.
    let padding = cx.debug_bounds("inspector-slider-Padding").unwrap();
    cx.simulate_click(padding.center(), Modifiers::none());
    redraw(cx);
    cx.simulate_keystrokes("home");
    redraw(cx);
    for _ in 0..8 {
        cx.simulate_keystrokes("right");
        redraw(cx);
    }
    cx.update(|window, cx| {
        let configuration = crate::inspection::active_inspector_configuration(window, cx).unwrap();
        let padding = configuration
            .changes
            .iter()
            .find(|change| change.property == "Padding")
            .unwrap();
        assert_eq!(padding.before, "8 / 16 / 8 / 16");
        assert_eq!(padding.after, "8 / 8 / 8 / 8");
        assert!(configuration.rust.contains(".p(px(8.0))"));
    });
}

#[gpui::test]
fn inspector_edits_surface_and_text_colors_and_reset_reverts_them(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::integration::init_for_tests(cx);
        init(cx).unwrap();
    });
    let (_, cx) = cx.add_window_view(|_, _| InspectorSample);
    cx.update(|window, cx| window.toggle_inspector(cx));
    cx.run_until_parked();
    let sample = cx.debug_bounds("inspector-sample").unwrap();
    cx.simulate_mouse_move(sample.center(), None, Modifiers::none());
    cx.simulate_click(sample.center(), Modifiers::none());
    redraw(cx);
    cx.update(|window, cx| {
        let selection = crate::inspection::inspector_selection(window, cx).unwrap();
        assert_eq!(selection.properties["Background"], "none");
        assert_eq!(selection.properties["Text color"], "none");
    });
    let swatch = cx
        .debug_bounds("inspector-color-Surface")
        .expect("surface swatch");
    cx.simulate_click(swatch.center(), Modifiers::none());
    redraw(cx);
    // Kit's palette swatches are plain interactive elements; the first swatch
    // begins one gap below the Palette tab and has a 20px square hit target.
    let palette = cx.update(|window, _| {
        gpui::base::test_support::snapshots(window)
            .into_iter()
            .find(|item| item.label() == Some("Palette"))
            .expect("Kit palette opens")
            .bounds()
    });
    cx.simulate_click(
        gpui::point(palette.left() + px(10.), palette.bottom() + px(22.)),
        Modifiers::none(),
    );
    redraw(cx);
    cx.update(|window, cx| {
        let configuration = crate::inspection::active_inspector_configuration(window, cx)
            .expect("a color edit publishes a configuration");
        let background = configuration
            .changes
            .iter()
            .find(|change| change.property == "Background")
            .expect("background change");
        assert_eq!(background.before, "none");
        assert!(background.after.starts_with('#'));
        assert!(configuration.rust.contains(".bg(rgb") || configuration.rust.contains(".bg(rgba"));
        let selection = crate::inspection::inspector_selection(window, cx).unwrap();
        assert_eq!(selection.properties["Background"], background.after);
    });
    let reset = cx.debug_bounds("inspector-reset").unwrap();
    cx.simulate_click(reset.center(), Modifiers::none());
    redraw(cx);
    cx.update(|window, cx| {
        assert!(crate::inspection::active_inspector_configuration(window, cx).is_none());
        let selection = crate::inspection::inspector_selection(window, cx).unwrap();
        assert_eq!(selection.properties["Background"], "none");
    });
}

#[test]
fn rust_color_constructors_follow_gpui_hex_conventions() {
    assert_eq!(
        crate::inspection::style::rust_color(gpui::rgb(0x336699).into()),
        "rgb(0x336699)"
    );
    assert_eq!(
        crate::inspection::style::rust_color(gpui::rgba(0x33669980).into()),
        "rgba(0x33669980)"
    );
}

#[gpui::test]
fn mounting_requires_initialization_and_preserves_existing_host_focus(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_, cx) = cx.add_window_view(|window, cx| {
        assert!(
            Agentation::mount(
                "Test",
                "gpui://test",
                dir.path().join("notes.json"),
                0,
                window,
                cx
            )
            .is_err()
        );
        crate::integration::init_for_tests(cx);
        init(cx).unwrap();
        let host = cx.focus_handle();
        window.focus(&host, cx);
        let _tools = Agentation::mount(
            "Test",
            "gpui://test",
            dir.path().join("notes.json"),
            0,
            window,
            cx,
        )
        .unwrap();
        assert!(host.is_focused(window));
        gpui::EmptyView
    });
    cx.run_until_parked();
}

#[test]
fn inspector_breadcrumb_starts_at_source_package_regardless_of_launch_directory() {
    use crate::inspection::source::SourceLocation;
    use std::path::Path;
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("example/src/main.rs");
    for cwd in [Path::new("/"), Path::new("/tmp"), file.parent().unwrap()] {
        let source = SourceLocation::new(file.to_str().unwrap(), 42, 7, cwd);
        assert_eq!(source.display_path(), Path::new("example/src/main.rs"));
        assert_eq!(source.path, file);
    }
}

#[test]
fn inspector_source_paths_resolve_owned_and_kit_sources() {
    use crate::inspection::source::SourceLocation;
    let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let normal = SourceLocation::new("src/overlay.rs", 42, 7, cwd);
    assert_eq!(normal.relative, std::path::Path::new("src/overlay.rs"));
    assert_eq!(normal.clipboard_path(), "src/overlay.rs:42:7");
    let source = SourceLocation::new("vendor/gpui-component/src/button/button.rs", 42, 7, cwd);
    assert_eq!(
        source.relative,
        std::path::Path::new("vendor/gpui-component/src/button/button.rs")
    );
    assert_eq!(source.line, 42);
    assert_eq!(source.column, 7);
    let sibling = SourceLocation::new(
        "/workspace/shared/component.rs",
        10,
        2,
        std::path::Path::new("/workspace/app"),
    );
    assert_eq!(
        sibling.relative,
        std::path::Path::new("../shared/component.rs")
    );
}

#[test]
fn inspector_editor_commands_should_preserve_quoted_arguments_and_location() {
    use crate::inspection::source::{SourceLocation, editor_command};
    let source = SourceLocation::new(
        "src/file with spaces.rs",
        42,
        7,
        std::path::Path::new("/workspace/app"),
    );
    let zed = editor_command(None, &source).unwrap();
    assert_eq!(zed.program, "zed");
    assert_eq!(
        zed.arguments,
        [std::ffi::OsString::from(
            "/workspace/app/src/file with spaces.rs:42:7"
        )]
    );
    let code = editor_command(
        Some("'/Applications/VS Code/bin/code' --reuse-window"),
        &source,
    )
    .unwrap();
    assert_eq!(code.program, "/Applications/VS Code/bin/code");
    assert_eq!(
        code.arguments,
        [
            "--reuse-window",
            "--goto",
            "/workspace/app/src/file with spaces.rs:42:7"
        ]
        .map(std::ffi::OsString::from)
    );
    assert!(editor_command(Some("vim"), &source).is_none());
    assert!(editor_command(Some("'unclosed"), &source).is_none());
    assert!(editor_command(Some("zed; touch /tmp/should-not-exist"), &source).is_none());
}

#[gpui::test]
fn unsupported_editor_should_copy_source_with_inline_feedback(cx: &mut TestAppContext) {
    use crate::inspection::source::{SourceLocation, activate_source};
    cx.update(|cx| {
        let source = SourceLocation::new(
            "src/overlay.rs",
            42,
            7,
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
        );
        let notice = cx.new(|_| String::new());
        activate_source(&source, &notice, Some("unsupported-ide"), cx);
        assert_eq!(notice.read(cx), "Path copied to clipboard");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "src/overlay.rs:42:7"
        );
    });
}

struct SourceBreadcrumbSample(std::rc::Rc<std::cell::Cell<usize>>);

impl Render for SourceBreadcrumbSample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
        let clicks = self.0.clone();
        div()
            .debug_selector(|| "source-test".into())
            .flex()
            .w(px(120.))
            .child(
                Breadcrumb::new().child(
                    BreadcrumbItem::new("sample.rs:42")
                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                ),
            )
    }
}

#[gpui::test]
fn source_breadcrumb_last_segment_should_be_clickable(cx: &mut TestAppContext) {
    cx.update(crate::integration::init_for_tests);
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let (_, cx) = cx.add_window_view(|_, _| SourceBreadcrumbSample(clicks.clone()));
    redraw(cx);
    let file = cx.debug_bounds("source-test").unwrap();
    cx.simulate_click(file.center(), Modifiers::none());
    redraw(cx);
    assert_eq!(clicks.get(), 1);
}

#[test]
fn own_source_should_resolve_from_compile_root_when_runtime_cwd_differs() {
    use crate::inspection::source::SourceLocation;
    let other = tempfile::tempdir().unwrap();
    std::fs::create_dir(other.path().join("src")).unwrap();
    std::fs::write(other.path().join("src/overlay.rs"), "unrelated file").unwrap();
    let source = SourceLocation::new("src/overlay.rs", 42, 7, other.path());
    assert_eq!(
        source.path,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/overlay.rs")
    );
    assert!(!source.relative.is_absolute());
    assert!(source.relative.starts_with(".."));
}

#[gpui::test]
fn failed_editor_launch_should_copy_source_and_finish_inline_feedback(cx: &mut TestAppContext) {
    use crate::inspection::source::{SourceLocation, activate_source};
    let notice = cx.update(|cx| {
        let notice = cx.new(|_| String::new());
        let source = SourceLocation::new(
            "src/overlay.rs",
            42,
            7,
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
        );
        activate_source(
            &source,
            &notice,
            Some("/nonexistent-agentation-test/zed"),
            cx,
        );
        notice
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(notice.read(cx), "Path copied to clipboard");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "src/overlay.rs:42:7"
        );
    });
}

fn mount_in_light_window(cx: &mut TestAppContext) -> &mut VisualTestContext {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let (_, cx) = cx.add_window_view(|window, cx| {
        let tools = Agentation::mount("Appearance", "gpui://appearance", path, 0, window, cx);
        assert!(tools.is_ok());
        std::mem::forget(tools);
        InspectorSample
    });
    cx
}

#[gpui::test]
fn mounted_tools_adopt_the_window_appearance_when_agentation_owns_the_theme(
    cx: &mut TestAppContext,
) {
    cx.update(init_for_tests);
    assert!(cx.update(|cx| cx.has_global::<FollowSystemAppearance>()));
    // A stale dark theme is replaced by the (light) appearance of the window.
    cx.update(|cx| Theme::change(Mode::Dark, None, cx));
    let cx = mount_in_light_window(cx);
    cx.update(|_, cx| assert!(!cx.theme().is_dark()));
}

#[gpui::test]
fn mounted_tools_leave_a_host_owned_theme_alone(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui::init(cx);
        Theme::change(Mode::Dark, None, cx);
        init_for_tests(cx);
        assert!(!cx.has_global::<FollowSystemAppearance>());
    });
    let cx = mount_in_light_window(cx);
    cx.update(|_, cx| assert!(cx.theme().is_dark()));
}

#[test]
fn kit_host_assets_fall_back_to_agentation_assets() {
    let assets = Assets::before(gpui::assets::Assets);
    assert!(assets.load("agentation/mcp.svg").unwrap().is_some());
    assert!(assets.load("icons/settings.svg").unwrap().is_some());
    assert!(assets.load("missing.svg").unwrap().is_none());
}

#[test]
fn host_assets_take_precedence_on_collisions() {
    struct Host;
    impl AssetSource for Host {
        fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
            Ok(vec!["icons/settings.svg".into()])
        }
        fn load(&self, _: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
            Ok(Some(Cow::Borrowed(b"host-settings")))
        }
    }
    assert_eq!(
        Assets::before(Host)
            .load("icons/settings.svg")
            .unwrap()
            .unwrap()
            .as_ref(),
        b"host-settings"
    );
    assert!(
        Assets::before(Host)
            .load("agentation/mcp.svg")
            .unwrap()
            .unwrap()
            .starts_with(b"<svg")
    );
}

struct ScrollingAnnotationsHost {
    tools: Agentation,
    scroll: gpui::ScrollHandle,
    oversized: bool,
}
impl Render for ScrollingAnnotationsHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if self.oversized {
            crate::target(
                "content",
                "Content",
                div().w_full().h(px(1400.)).child("Tall content"),
            )
            .into_any_element()
        } else {
            div()
                .w_full()
                .h(px(1400.))
                .flex()
                .flex_col()
                .child(div().h(px(100.)).flex_none())
                .child(crate::target(
                    "first",
                    "First",
                    div().w_full().h(px(80.)).flex_none().child("First card"),
                ))
                .child(div().h(px(320.)).flex_none())
                .child(crate::target(
                    "second",
                    "Second",
                    div().w_full().h(px(80.)).flex_none().child("Second card"),
                ))
                .into_any_element()
        };
        self.tools.root(
            "screen",
            "Screen",
            div()
                .id("scrolling-host")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(content),
            window,
            cx,
        )
    }
}

// Deliver only requested native-frame callbacks. Avoid unconditional duplicate
// draws, which used to conceal missing invalidation after a scroll.
fn settle_annotation_frames(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    for _ in 0..8 {
        let callbacks = cx.update(|window, cx| window.simulate_next_frame(cx));
        if callbacks == 0 {
            break;
        }
        cx.run_until_parked();
    }
}

fn check_native_annotation_scrolling(cx: &mut TestAppContext, oversized: bool) {
    use crate::{AnnotationInput, AnnotationShape, Rect, Store};
    use gpui::{ScrollDelta, ScrollEvent, ScrollSource, TouchPhase, point, size};
    cx.update(crate::integration::init_for_tests);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("annotations.json");
    let store = Store::open(&path).unwrap();
    let session = store
        .ensure_session("Native scrolling", "gpui://native-scroll")
        .unwrap();
    // Both notes exist before the host has registered any geometry. The box is
    // in a gap between cards, where the stationary screen root used to win.
    for shape in [
        AnnotationShape::Box {
            from: (0.2, 200. / 600.),
            to: (0.4, 280. / 600.),
            color: 0,
        },
        AnnotationShape::Text {
            at: (0.5, 240. / 600.),
            text: "Scrolling text".into(),
            color: 0,
            width: Some(0.3),
            height: Some(0.1),
        },
    ] {
        store
            .add(
                &session.id,
                AnnotationInput {
                    inspection: None,
                    comment: "Scrolling note".into(),
                    element_path: "drawing:test".into(),
                    element: "Drawing".into(),
                    x: 30.,
                    y: 240.,
                    bounding_box: Rect::default(),
                    source_file: None,
                    gpui_configuration: None,
                    shape: Some(shape),
                },
            )
            .unwrap();
    }
    let (host, cx) = cx.add_window_view(|window, cx| ScrollingAnnotationsHost {
        tools: Agentation::mount(
            "Native scrolling",
            "gpui://native-scroll",
            &path,
            0,
            window,
            cx,
        )
        .unwrap(),
        scroll: gpui::ScrollHandle::new(),
        oversized,
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, _| window.activate_window());
    settle_annotation_frames(cx);
    let pin = cx.debug_bounds("feedback-marker-number-1").unwrap();
    let text = cx.debug_bounds("feedback-textbox").unwrap();
    // Open the box's read-only preview before scrolling.
    cx.simulate_click(pin.center(), Modifiers::none());
    settle_annotation_frames(cx);
    let panel = cx.debug_bounds("feedback-editor-panel").unwrap();
    let wheel = || ScrollEvent {
        position: point(px(750.), px(400.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-30.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::default(),
    };
    cx.simulate_event(wheel());
    settle_annotation_frames(cx);
    let offset = cx.update(|_, cx| host.read(cx).scroll.offset().y);
    assert_eq!(offset, px(-30.), "the real scroll container moved");
    let moved_pin = cx.debug_bounds("feedback-marker-number-1").unwrap();
    let moved_text = cx.debug_bounds("feedback-textbox").unwrap();
    let moved_panel = cx.debug_bounds("feedback-editor-panel").unwrap();
    assert_eq!(
        moved_pin.center().y,
        pin.center().y + offset,
        "drawing pin follows content"
    );
    assert_eq!(
        moved_text.top(),
        text.top() + offset,
        "textbox follows content"
    );
    assert_eq!(
        moved_panel.top(),
        panel.top() + offset,
        "unfocused note preview follows content"
    );
    // Enter editing with annotation mode still off. The editor can scroll
    // internally, but wheel events over the host must not move its content.
    let edit = cx.debug_bounds("feedback-editor-primary").unwrap();
    cx.simulate_click(edit.center(), Modifiers::none());
    settle_annotation_frames(cx);
    cx.simulate_event(wheel());
    settle_annotation_frames(cx);
    assert_eq!(
        cx.update(|_, cx| host.read(cx).scroll.offset().y),
        offset,
        "typing locks host scrolling"
    );
    cx.update(|window, cx| {
        let focus = host.read(cx).tools.focus.clone();
        window.focus(&focus, cx);
    });
    settle_annotation_frames(cx);
    cx.simulate_event(wheel());
    settle_annotation_frames(cx);
    let unfocused_offset = cx.update(|_, cx| host.read(cx).scroll.offset().y);
    assert!(
        unfocused_offset < offset,
        "leaving editor focus restores scrolling"
    );
    cx.simulate_click(point(px(750.), px(400.)), Modifiers::none());
    settle_annotation_frames(cx);
    assert!(
        cx.debug_bounds("feedback-editor-panel").is_none(),
        "clicking outside dismisses the editor"
    );
    cx.simulate_event(wheel());
    settle_annotation_frames(cx);
    assert!(
        cx.update(|_, cx| host.read(cx).scroll.offset().y) < unfocused_offset,
        "closing editor restores scrolling"
    );
}

#[gpui::test]
fn saved_drawings_text_and_note_preview_follow_native_scrolling(cx: &mut TestAppContext) {
    check_native_annotation_scrolling(cx, false);
}

#[gpui::test]
fn annotations_follow_oversized_content_even_when_clipped_bounds_stay_the_same(
    cx: &mut TestAppContext,
) {
    check_native_annotation_scrolling(cx, true);
}
