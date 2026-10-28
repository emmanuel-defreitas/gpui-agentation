//! Application startup and window-level focus ownership for native hosts.
use std::{borrow::Cow, path::PathBuf};

use crate::ely::{primitives::FocusScope, theme::Theme};
use gpui::{
    AnyElement, App, AssetSource, Entity, FocusHandle, Global, InteractiveElement, IntoElement,
    ParentElement, SharedString, Styled, Window, div,
};

use crate::{AnnotationOverlay, Error, begin_frame, target};

/// Embedded fonts and icons used by Agentation. Configure before calling [`init`].
///
/// Pass this to `Application::with_assets`; use [`Assets::before`] when the
/// application already has an asset source.
pub struct Assets;

impl Assets {
    /// Serve Agentation's assets first, falling back to the host's other assets.
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
                "reload.svg" => Some(include_bytes!("assets/menubar/reload.svg")),
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
                "../docs/assets/mark.svg"
            ))));
        }
        crate::ely::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = crate::ely::Assets.list(path)?;
        if "agentation/mcp.svg".starts_with(path) {
            paths.push("agentation/mcp.svg".into());
        }
        if "agentation/mark.svg".starts_with(path) {
            paths.push("agentation/mark.svg".into());
        }
        for name in [
            "brand.svg",
            "reload.svg",
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
        match Assets.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => self.0.load(path),
        }
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

/// Initialize Agentation's theme, fonts and keyboard bindings once per GPUI app.
///
/// Configure [`Assets`] on the application before launch and call this inside
/// `Application::run`, before mounting tools. Hosts do not need to initialize
/// Ely or import its components. If Ely is already initialized, its existing
/// theme and bindings are retained. Repeated calls are harmless.
pub fn init(cx: &mut App) -> gpui::Result<()> {
    if !cx.has_global::<Initialized>() {
        if !cx.has_global::<Theme>() {
            crate::ely::init(cx).map_err(|error| {
                std::io::Error::other(format!(
                    "Agentation initialization failed; configure Application::with_assets(gpui_agentation::Assets), or Assets::before(host_assets): {error}"
                ))
            })?;
        }
        crate::overlay::inspector::install(cx);
        cx.set_global(Initialized);
    }
    Ok(())
}

/// Initialize Agentation in headless GPUI tests, without fonts or native assets.
///
/// Requires the `test-support` feature. Production applications should use [`init`].
#[cfg(feature = "test-support")]
pub fn init_for_tests(cx: &mut App) {
    if !cx.has_global::<Initialized>() {
        if !cx.has_global::<Theme>() {
            crate::ely::init_for_tests(cx);
        }
        crate::overlay::inspector::install(cx);
        cx.set_global(Initialized);
    }
}

/// One window's annotation tools and root focus boundary.
///
/// Retain this on the window's root GPUI entity and render content through
/// [`Self::root`]. All Ely focus setup is owned here. Like the rest of this
/// crate, this type is absent in release builds.
pub struct Agentation {
    overlay: Entity<AnnotationOverlay>,
    focus: FocusHandle,
}

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
        Ok(Self { overlay, focus })
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
        FocusScope::new(&self.focus)
            .root()
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
#[path = "integration.test.rs"]
mod tests;
