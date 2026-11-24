//! Executable compatibility contract, not the migrated Agentation library.
use gpui::{App, AssetSource, Global, SharedString, component::Theme};
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub enum HostMode {
    Standalone,
    /// The host has called Kit's full init, not merely installed a Theme.
    KitInitialized,
}

struct Prepared;
impl Global for Prepared {}

/// Model the explicit initialization ownership required at the cutover.
/// Do not infer complete Kit initialization from Theme presence alone.
pub fn prepare(mode: HostMode, cx: &mut App) {
    if cx.has_global::<Prepared>() {
        return;
    }
    match mode {
        HostMode::Standalone => {
            assert!(
                !cx.has_global::<Theme>(),
                "choose KitInitialized for a Kit host"
            );
            gpui::init(cx);
        }
        HostMode::KitInitialized => {
            assert!(
                cx.has_global::<Theme>(),
                "the host must initialize Kit first"
            );
        }
    }
    // In particular, do not install or replace the host's inspector renderer.
    cx.set_global(Prepared);
}

/// Host assets win collisions. Missing Kit paths must retain AssetSource's
/// Ok(None) fallback behavior even though Kit Assets returns an error for them.
pub struct ComposedAssets<T>(pub T);

impl<T: AssetSource> AssetSource for ComposedAssets<T> {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(asset) = self.0.load(path)? {
            return Ok(Some(asset));
        }
        if gpui::assets::Assets
            .list(path)?
            .iter()
            .any(|name| name.as_ref() == path)
        {
            gpui::assets::Assets.load(path)
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut names = self.0.list(path)?;
        names.extend(gpui::assets::Assets.list(path)?);
        names.sort();
        names.dedup();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::component::{ActiveTheme, ThemeMode};
    use gpui::{
        AppContext, Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window,
        div,
    };
    use std::{cell::Cell, rc::Rc};

    struct HostAssets;
    impl AssetSource for HostAssets {
        fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
            Ok((path == "host/logo.svg").then_some(Cow::Borrowed(b"host")))
        }
        fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
            Ok(if "host/logo.svg".starts_with(path) {
                vec!["host/logo.svg".into()]
            } else {
                vec![]
            })
        }
    }

    #[test]
    fn assets_preserve_host_kit_and_missing_paths() {
        let assets = ComposedAssets(HostAssets);
        assert_eq!(
            assets.load("host/logo.svg").unwrap().unwrap().as_ref(),
            b"host"
        );
        let icon = gpui::assets::Assets.list("icons/").unwrap().remove(0);
        assert!(assets.load(&icon).unwrap().is_some());
        assert!(assets.load("missing/probe.svg").unwrap().is_none());
        assert!(
            assets
                .list("host/")
                .unwrap()
                .iter()
                .any(|p| p == "host/logo.svg")
        );
    }

    #[gpui::test]
    fn standalone_initializes_once(cx: &mut TestAppContext) {
        cx.update(|cx| {
            prepare(HostMode::Standalone, cx);
            Theme::change(ThemeMode::Dark, None, cx);
            let background = cx.theme().background;
            prepare(HostMode::Standalone, cx);
            assert_eq!(cx.theme().mode, ThemeMode::Dark);
            assert_eq!(cx.theme().background, background);
        });
    }

    struct HostView;
    impl Render for HostView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child("Host content")
        }
    }

    #[gpui::test]
    fn existing_host_keeps_theme_root_focus_and_inspector(cx: &mut TestAppContext) {
        let rendered = Rc::new(Cell::new(false));
        cx.update(|cx| {
            gpui::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            let rendered = rendered.clone();
            cx.set_inspector_renderer(Box::new(move |_, _, _| {
                rendered.set(true);
                div().into_any_element()
            }));
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| HostView);
            gpui::base::Root::new(content, window, cx)
        });
        cx.update(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            let root_id = window
                .root::<gpui::base::Root>()
                .flatten()
                .unwrap()
                .entity_id();
            prepare(HostMode::KitInitialized, cx);
            prepare(HostMode::KitInitialized, cx);
            assert_eq!(cx.theme().mode, ThemeMode::Dark);
            assert_eq!(window.focused(cx), Some(focus));
            assert_eq!(
                window
                    .root::<gpui::base::Root>()
                    .flatten()
                    .unwrap()
                    .entity_id(),
                root_id
            );
            assert_eq!(root.entity_id(), root_id);
            window.toggle_inspector(cx);
        });
        cx.run_until_parked();
        assert!(
            rendered.get(),
            "host inspector callback must survive initialization"
        );
    }
}
