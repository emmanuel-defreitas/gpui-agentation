//! Overlay unit tests, grouped by feature. They live in one module so they can
//! share helpers and reach `pub(super)` internals of the overlay.
use super::*;
use gpui::VisualTestContext;

mod annotations;
mod settings;
mod toolbar;

pub(super) fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

fn toolbar(overlay: &Entity<AnnotationOverlay>, cx: &mut VisualTestContext) -> VisualTestContext {
    let handle = cx.update(|_, cx| {
        overlay
            .read(cx)
            .companion_toolbar
            .expect("companion toolbar")
    });
    let mut tools = VisualTestContext::from_window(handle.into(), &cx.cx);
    draw(&mut tools);
    tools
}
