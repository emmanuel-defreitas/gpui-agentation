//! Stable annotation colors and geometry supplied by the Kit theme.
use gpui::component::{ActiveTheme, Colorize};
use gpui::{App, Hsla, Window, rems};
pub(super) fn palette(cx: &App) -> [Hsla; 4] {
    let t = cx.theme();
    [t.blue, t.green, t.red, t.yellow]
}

pub(super) const COLOR_NAMES: [&str; 4] = ["Blue", "Green", "Red", "Yellow"];

pub(super) fn toolbar_background(cx: &App) -> Hsla {
    if cx.theme().is_dark() {
        cx.theme().secondary
    } else {
        gpui::rgb(0xffffff).into()
    }
}

pub(super) fn note_background(color: usize, cx: &App) -> Hsla {
    let color = palette(cx)[color];
    if cx.theme().is_dark() {
        color
    } else {
        color.mix_oklab(gpui::rgb(0xffffff).into(), 0.18)
    }
}

/// Keep labels readable on every annotation fill in either system appearance.
pub(super) fn foreground(background: Hsla) -> Hsla {
    let rgb: gpui::Rgba = background.into();
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b);
    gpui::rgb(if luminance > 0.179 {
        0x000000
    } else {
        0xffffff
    })
    .into()
}
pub(super) fn stroke_metrics(window: &Window, _: &App) -> (f32, f32) {
    (
        rems(0.125).to_pixels(window.rem_size()).into(),
        rems(0.75).to_pixels(window.rem_size()).into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::component::{Theme, ThemeMode};
    #[gpui::test]
    fn annotation_palette_has_four_distinct_named_colors_in_both_themes(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::integration::init_for_tests);
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update(|cx| {
                Theme::change(mode, None, cx);
                let [blue, green, red, yellow] = palette(cx).map(gpui::Rgba::from);
                assert!(blue.b > blue.r && blue.b > blue.g);
                assert!(green.g > green.r && green.g > green.b);
                assert!(red.r > red.g && red.r > red.b);
                assert!(yellow.r > yellow.b && yellow.g > yellow.b);
                let luminance = |color: gpui::Rgba| {
                    let linear = |v: f32| {
                        if v <= 0.04045 {
                            v / 12.92
                        } else {
                            ((v + 0.055) / 1.055).powf(2.4)
                        }
                    };
                    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
                };
                for (index, color) in palette(cx).into_iter().enumerate() {
                    let note = note_background(index, cx);
                    let note_fill = luminance(note.into());
                    let note_ink = luminance(foreground(note).into());
                    let ratio = (note_fill.max(note_ink) + 0.05) / (note_fill.min(note_ink) + 0.05);
                    if !mode.is_dark() {
                        assert!(note_fill > 0.7, "light notes use pale backgrounds");
                        assert!(ratio >= 7., "light note text has strong contrast");
                    } else {
                        assert!(ratio >= 4.5);
                    }

                    let fill = luminance(color.into());
                    let ink = luminance(foreground(color).into());
                    assert!(
                        (fill.max(ink) + 0.05) / (fill.min(ink) + 0.05) >= 4.5,
                        "annotation labels need readable contrast in {mode:?}"
                    );
                }
            });
        }
    }
}
