//! Temporary Ely presentation adapter. Persisted palette indexes stay stable
//! when the UI backend changes; drawing code receives only colors and pixels.
use crate::ely::theme::ActiveTheme;
use gpui::{App, Hsla, Window};

/// Blue, green, orange, red: retain the original first four chart entries.
pub(super) fn palette(cx: &App) -> [Hsla; 4] {
    let chart = cx.theme().colors.chart;
    [chart[0], chart[1], chart[2], chart[3]]
}

/// Stroke width and arrow-head length in logical pixels at this window's rem size.
pub(super) fn stroke_metrics(window: &Window, cx: &App) -> (f32, f32) {
    let media = cx.theme().media();
    (
        f32::from(media.mark_stroke.to_pixels(window.rem_size())),
        f32::from(media.arrow_head.to_pixels(window.rem_size())),
    )
}
