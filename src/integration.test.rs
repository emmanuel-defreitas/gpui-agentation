use super::*;
use crate::ely::theme::{ActiveTheme, Mode};
use gpui::TestAppContext;

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
