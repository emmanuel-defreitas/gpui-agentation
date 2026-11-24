//! Stable annotation colors and geometry supplied by the Kit theme.
use gpui::component::ActiveTheme;
use gpui::{App, Hsla, Window, rems};
pub(super) fn palette(cx: &App) -> [Hsla; 4] {
    let t = cx.theme();
    [t.chart_1, t.chart_2, t.chart_3, t.chart_4]
}
pub(super) fn stroke_metrics(window: &Window, _: &App) -> (f32, f32) {
    (
        rems(0.125).to_pixels(window.rem_size()).into(),
        rems(0.75).to_pixels(window.rem_size()).into(),
    )
}
