use super::*;
use crate::ely::theme::{ActiveTheme, Mode};
use gpui::{
    AppContext, Context, IntoElement, Modifiers, Render, Styled, TestAppContext, VisualTestContext,
    Window, div, px,
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
fn asset_composition_preserves_host_assets_and_supplies_tooling_icons_and_fonts() {
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
    let font = names.iter().find(|name| name.ends_with(".ttf")).unwrap();
    assert!(!assets.load(icon).unwrap().unwrap().is_empty());
    assert!(!assets.load(font).unwrap().unwrap().is_empty());
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
        crate::ely::init_for_tests(cx);
        Theme::set_mode_now(Mode::Dark, cx);
        let background = cx.theme().colors.bg;
        init(cx).unwrap();
        init(cx).unwrap();
        assert!(cx.theme().is_dark());
        assert_eq!(cx.theme().colors.bg, background);
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
        crate::ely::init_for_tests(cx);
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
        .debug_bounds("inspector-slider-ely-inspector-width")
        .expect("width slider");
    cx.simulate_click(width.center(), Modifiers::none());
    redraw(cx);
    cx.update(|window, cx| {
        let configuration = crate::ely::tooling::active_inspector_configuration(window, cx)
            .expect("active inspector configuration");
        assert_eq!(configuration.changes[0].property, "Width");
        let after: f32 = configuration.changes[0]
            .after
            .trim_end_matches("px")
            .parse()
            .unwrap();
        assert_ne!(after, 120.0);
        let selection = crate::ely::tooling::inspector_selection(window, cx).unwrap();
        assert!((after - f32::from(selection.bounds.size.width)).abs() < 0.1);
        assert!(configuration.rust.contains(".w(px("));
    });
    // Returning the top side to 8 still changes the original 16px sides.
    let padding = cx
        .debug_bounds("inspector-slider-ely-inspector-padding")
        .unwrap();
    cx.simulate_click(padding.center(), Modifiers::none());
    redraw(cx);
    cx.simulate_keystrokes("home");
    redraw(cx);
    for _ in 0..8 {
        cx.simulate_keystrokes("right");
        redraw(cx);
    }
    cx.update(|window, cx| {
        let configuration =
            crate::ely::tooling::active_inspector_configuration(window, cx).unwrap();
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
        crate::ely::init_for_tests(cx);
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
fn inspector_source_paths_should_be_relative_and_map_generated_ely_to_editable_files() {
    use crate::ely::tooling::inspector::source::SourceLocation;
    let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let normal = SourceLocation::new("src/overlay.rs", 42, 7, cwd);
    assert_eq!(normal.relative, std::path::Path::new("src/overlay.rs"));
    assert_eq!(normal.clipboard_path(), "src/overlay.rs:42:7");
    let generated = std::path::Path::new(env!("OUT_DIR")).join("ely/buttons/button.rs");
    let source = SourceLocation::new(generated.to_str().unwrap(), 42, 7, cwd);
    assert_eq!(
        source.relative,
        std::path::Path::new("vendor/ely/src/buttons/button.rs")
    );
    assert_eq!(source.path, cwd.join("vendor/ely/src/buttons/button.rs"));
    assert_eq!(source.line, 42);
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
    use crate::ely::tooling::inspector::source::{SourceLocation, editor_command};
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
    use crate::ely::tooling::inspector::source::{SourceLocation, activate_source};
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
        use crate::ely::navigation::{Breadcrumb, Crumb};
        let clicks = self.0.clone();
        Breadcrumb::new(
            "source-test",
            [Crumb::new("src", "src"), Crumb::new("file", "sample.rs:42")],
        )
        .on_current(move |_, _, _| clicks.set(clicks.get() + 1))
    }
}

#[gpui::test]
fn source_breadcrumb_last_segment_should_be_clickable(cx: &mut TestAppContext) {
    cx.update(crate::ely::init_for_tests);
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let (_, cx) = cx.add_window_view(|_, _| SourceBreadcrumbSample(clicks.clone()));
    redraw(cx);
    let file = cx.debug_bounds("inspector-source-open").unwrap();
    cx.simulate_click(file.center(), Modifiers::none());
    redraw(cx);
    assert_eq!(clicks.get(), 1);
}

#[test]
fn own_source_should_resolve_from_compile_root_when_runtime_cwd_differs() {
    use crate::ely::tooling::inspector::source::SourceLocation;
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
    use crate::ely::tooling::inspector::source::{SourceLocation, activate_source};
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
