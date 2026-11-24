//! Application startup and window-level focus ownership for native hosts.
use std::{borrow::Cow, path::PathBuf};

use gpui::component::{Theme, ThemeMode};
use gpui::{
    AnyElement, App, AssetSource, Entity, FocusHandle, Global, InteractiveElement, IntoElement,
    ParentElement, SharedString, Styled, Subscription, Window, div,
};

use crate::{AnnotationOverlay, Error, begin_frame, target};

/// Embedded fonts and icons used by Agentation. Configure before calling [`init`].
///
/// Pass this to `Application::with_assets`; use [`Assets::before`] when the
/// application already has an asset source.
pub struct Assets;

impl Assets {
    /// Combine host assets with Agentation and Kit assets. Host paths take precedence.
    pub fn before(other: impl AssetSource) -> impl AssetSource {
        CombinedAssets(other)
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == "agentation/mcp.svg" {
            return Ok(Some(Cow::Borrowed(include_bytes!("assets/mcp.svg"))));
        }
        if let Some(name) = path.strip_prefix("agentation/menubar/") {
            let asset: Option<&'static [u8]> = match name {
                "brand.svg" => Some(include_bytes!("assets/menubar/brand.svg")),
                "crates.png" => Some(include_bytes!("assets/menubar/crates.png")),
                "github.svg" => Some(include_bytes!("assets/menubar/github.svg")),
                "mcp.svg" => Some(include_bytes!("assets/menubar/mcp.svg")),
                "copy.svg" => Some(include_bytes!("assets/menubar/copy.svg")),
                _ => None,
            };
            if let Some(asset) = asset {
                return Ok(Some(Cow::Borrowed(asset)));
            }
        }
        if path == "agentation/mark.svg" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../site/docs/assets/mark.svg"
            ))));
        }
        if gpui::assets::AllAssets
            .list(path)?
            .iter()
            .any(|name| name.as_ref() == path)
        {
            gpui::assets::AllAssets.load(path)
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = gpui::assets::AllAssets.list(path)?;
        if "agentation/mcp.svg".starts_with(path) {
            paths.push("agentation/mcp.svg".into());
        }
        if "agentation/mark.svg".starts_with(path) {
            paths.push("agentation/mark.svg".into());
        }
        for name in [
            "brand.svg",
            "crates.png",
            "github.svg",
            "mcp.svg",
            "copy.svg",
        ] {
            let asset = format!("agentation/menubar/{name}");
            if asset.starts_with(path) {
                paths.push(asset.into());
            }
        }
        Ok(paths)
    }
}

struct CombinedAssets<T>(T);
impl<T: AssetSource> AssetSource for CombinedAssets<T> {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if self.0.list(path)?.iter().any(|name| name.as_ref() == path) {
            if let Some(bytes) = self.0.load(path)? {
                return Ok(Some(bytes));
            }
        }
        Assets.load(path)
    }
    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = Assets.list(path)?;
        paths.extend(self.0.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

struct Initialized;
impl Global for Initialized {}

/// Initialize Kit and Agentation once per GPUI app.
///
/// Configure [`Assets`] on the application before launch and call this inside
/// `Application::run`, before mounting tools. Hosts do not need to initialize
/// Kit separately. An initialized Kit host must use [`init_with_options`] with
/// [`HostMode::KitInitialized`] to preserve its theme and bindings. Repeated
/// successful initialization calls are harmless.
pub fn init(cx: &mut App) -> gpui::Result<()> {
    init_with_options(InitOptions::default(), cx)
}

/// Ownership of Kit application initialization.
#[derive(Clone, Copy, Default)]
pub enum HostMode {
    /// Agentation initializes Kit once before any windows are opened.
    #[default]
    Standalone,
    /// The host has already called Kit's full initialization function.
    KitInitialized,
}

/// Explicit host choices for initialization and inspector ownership.
#[derive(Clone, Copy)]
pub struct InitOptions {
    /// Who initializes Kit's globals, bindings and theme.
    pub host: HostMode,
    /// Whether Agentation may replace the application's inspector renderer.
    pub inspector: crate::inspection::kit::RendererPolicy,
}
impl Default for InitOptions {
    fn default() -> Self {
        Self {
            host: HostMode::Standalone,
            inspector: crate::inspection::kit::RendererPolicy::Replace,
        }
    }
}

/// Initialize without replacing a Kit host's theme, bindings, assets or Root.
/// A Kit-initialized host must choose its mode and inspector policy explicitly.
pub fn init_with_options(options: InitOptions, cx: &mut App) -> gpui::Result<()> {
    if cx.has_global::<Initialized>() {
        return Ok(());
    }
    if cx.asset_source().load("agentation/mcp.svg")?.is_none() {
        anyhow::bail!(
            "Configure Application::with_assets(gpui_agentation::Assets), or Assets::before(host_assets)"
        );
    }
    match options.host {
        HostMode::Standalone => {
            if cx.has_global::<Theme>() {
                anyhow::bail!(
                    "Kit already has a theme; use HostMode::KitInitialized after full Kit initialization"
                );
            }
            gpui::init(cx);
            cx.set_global(FollowSystemAppearance);
        }
        HostMode::KitInitialized => {
            if !cx.has_global::<Theme>() {
                anyhow::bail!("Initialize GPUI Kit before Agentation");
            }
        }
    }
    crate::inspection::kit::install(options.inspector, cx);
    cx.set_global(Initialized);
    Ok(())
}

/// Initialize Kit and Agentation for headless GPUI tests.
#[cfg(any(test, feature = "test-support"))]
pub fn init_for_tests(cx: &mut App) {
    if !cx.has_global::<Theme>() {
        gpui::init(cx);
        cx.set_global(FollowSystemAppearance);
    }
    crate::overlay::inspector::install(cx);
    cx.set_global(Initialized);
}

/// One window's annotation tools and root focus boundary.
///
/// Retain this on the window's root GPUI entity and render content through
/// [`Self::root`], inside the host's Kit Root. Like the rest of this
/// crate, this type is absent in release builds.
pub struct Agentation {
    overlay: Entity<AnnotationOverlay>,
    focus: FocusHandle,
    _appearance: Option<Subscription>,
}

/// Set when Agentation created the theme itself, so its tools follow the OS
/// light/dark setting. A host that already initialized Kit keeps control.
struct FollowSystemAppearance;
impl Global for FollowSystemAppearance {}

impl Agentation {
    /// Mount tools for a window after [`init`], using a local store and port.
    ///
    /// The initial host focus is preserved. A storage error is returned; a busy
    /// listener leaves local annotations usable, as with [`AnnotationOverlay::mount`].
    pub fn mount(
        app_name: &str,
        route: &str,
        path: impl Into<PathBuf>,
        port: u16,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self, Error> {
        if !cx.has_global::<Initialized>() {
            return Err(Error::Invalid(
                "Call gpui_agentation::init before mounting Agentation".into(),
            ));
        }
        let overlay = AnnotationOverlay::mount(app_name, route, path, port, window, cx)?;
        let focus = cx.focus_handle();
        if window.focused(cx).is_none() {
            window.focus(&focus, cx);
        }
        let appearance = cx.has_global::<FollowSystemAppearance>().then(|| {
            Theme::change(ThemeMode::from(window.appearance()), Some(window), cx);
            window.observe_window_appearance(|window, cx| {
                Theme::change(ThemeMode::from(window.appearance()), Some(window), cx)
            })
        });
        Ok(Self {
            overlay,
            focus,
            _appearance: appearance,
        })
    }

    /// Render one screen and own its root focus handling; the toolbar is a companion window.
    ///
    /// Call once per window render, below custom chrome and outside content
    /// padding/scrolling. The content should fill the remaining column space.
    /// This registers the screen only, not every descendant. `track_caller`
    /// preserves the host call site for the inspector's source reference.
    #[track_caller]
    pub fn root(
        &self,
        id: &str,
        label: &str,
        content: impl IntoElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        begin_frame(window, cx);
        let overlay = self.overlay.clone();
        div()
            .track_focus(&self.focus)
            .size_full()
            .child(
                div()
                    .id("agentation-host-root")
                    .capture_action(move |action: &crate::AnnotationCommand, window, cx| {
                        overlay.update(cx, |overlay, cx| {
                            overlay.dispatch_command(action, window, cx)
                        });
                        cx.stop_propagation();
                    })
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(self.overlay.clone())
                    .child(target(id, label, content)),
            )
            .into_any_element()
    }

    /// Switch annotation buckets after the host commits navigation.
    ///
    /// Saved comments remain on their route; an unfinished editor is cancelled.
    pub fn set_route(&self, route: &str, window: &mut Window, cx: &mut App) {
        self.overlay
            .update(cx, |overlay, cx| overlay.set_route(route, window, cx));
    }
}

#[cfg(test)]
#[path = "tests/integration.rs"]
mod tests;
