pub(crate) mod source;

use gpui::{
    AnyElement, App, AppContext, BorderStyle, Bounds, Context, Corners,
    DivInspectorState, Edges, Entity, FontWeight, Hsla, Inspector,
    InspectorElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, TextAlign, TextRun, Window,
    canvas, div, fill, point, prelude::FluentBuilder, quad, size, transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{ColorPicker, ColorSwatch, Slider},
    primitives::{FocusScope, raise},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, LEADING, literal, tabular},
};

// Chart hues Rust, Ochre, Green and Blue, as browsers color a box's layers.
const MARGIN: usize = 6;
const BORDER: usize = 2;
const PADDING: usize = 5;
const CONTENT: usize = 0;

// Temporary presentation bridge; relative paths bypass the Ely namespace adapter.
pub use super::super::super::inspection::{
    InspectorSelection, InspectorStyleChange, InspectorStyleConfiguration,
    active_inspector_configuration, clear_inspector_selection, inspector_selection,
    inspector_target, set_inspector_target,
};
pub use super::super::super::inspection::style::{restore_inspector_configuration, rust_color};
use super::super::super::inspection::{self as service, style::*};

fn edges(sides: [Side; 4]) -> Edges<Pixels> {
    let [top, right, bottom, left] = sides.map(Side::drawn);
    Edges {
        top,
        right,
        bottom,
        left,
    }
}

fn shrink(bounds: Bounds<Pixels>, by: Edges<Pixels>) -> Bounds<Pixels> {
    let inner = bounds.extend(Edges {
        top: -by.top,
        right: -by.right,
        bottom: -by.bottom,
        left: -by.left,
    });
    Bounds::new(
        inner.origin,
        inner.size.max(&size(Pixels::ZERO, Pixels::ZERO)),
    )
}

/// Paints the layers over the page at the box's bounds, with a tag that gives its size.
fn paint_layers(bounds: Bounds<Pixels>, layers: Layers, window: &mut Window, cx: &mut App) {
    let theme = cx.theme();
    let colors = &theme.colors;
    let hue = |hue| colors.hue(hue, "inspector");
    let band = |bounds, widths, hue| {
        quad(
            bounds,
            Corners::default(),
            transparent_black(),
            widths,
            colors.hue(hue, "inspector").alpha(0.4),
            BorderStyle::Solid,
        )
    };
    let (margin, border, padding) = (
        edges(layers.margin),
        edges(layers.border),
        edges(layers.padding),
    );
    let (outer, inner) = (bounds.extend(margin), shrink(bounds, border));
    window.paint_quad(band(outer, margin, MARGIN));
    window.paint_quad(band(bounds, border, BORDER));
    window.paint_quad(band(inner, padding, PADDING));
    window.paint_quad(fill(shrink(inner, padding), hue(CONTENT).alpha(0.25)));
    let text = theme.text_size(TextSize::Xs).to_pixels(window.rem_size());
    let label = format!(
        "{} × {}",
        number(bounds.size.width.into()),
        number(bounds.size.height.into())
    );
    let run = TextRun {
        len: label.len(),
        font: window.text_style().font(),
        color: colors.bg,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(label.into(), text, &[run], None);
    let (height, pad) = (text * LEADING, text / 2.0);
    let top = (outer.top() - height).max(Pixels::ZERO);
    let tag = Bounds::new(
        point(outer.left(), top),
        size(line.width + pad * 2.0, height),
    );
    let radius = theme.radius(Radius::Sm).to_pixels(window.rem_size());
    window.paint_quad(fill(tag, colors.fg).corner_radii(radius));
    if let Err(error) = line.paint(
        point(tag.left() + pad, tag.top()),
        height,
        TextAlign::Left,
        None,
        window,
        cx,
    ) {
        log::error!("inspector: size tag did not paint: {error:#}");
    }
}

/// One layer of the box model: its name, its four sides around what it holds.
fn ring(
    name: &'static str,
    sides: [Side; 4],
    hue: usize,
    inside: AnyElement,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let color = theme.colors.hue(hue, "inspector");
    let value = |side: Side| tabular(div()).child(side.to_string());
    let shown = sides.map(|side| side.to_string()).join("-");
    div()
        .debug_selector(move || format!("inspector-{name}-{shown}"))
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Sm))
        .border_1()
        .border_color(color)
        .bg(color.alpha(0.12))
        .child(
            div()
                .absolute()
                .top_1()
                .left_2()
                .text_color(theme.colors.fg_muted)
                .child(name),
        )
        .child(value(sides[0]))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(value(sides[3]))
                .child(inside)
                .child(value(sides[1])),
        )
        .child(value(sides[2]))
        .into_any_element()
}

/// A picked box's model, sizes and place, and its layers drawn over the page.
fn box_model(state: &DivInspectorState, window: &mut Window, cx: &mut App) -> AnyElement {
    let (bounds, children) = (state.bounds, state.content_size);
    let layers = layers(&state.base_style, window.rem_size());
    let content = shrink(shrink(bounds, edges(layers.border)), edges(layers.padding)).size;
    let pair = |w: Pixels, h: Pixels| format!("{} × {}", number(w.into()), number(h.into()));
    let theme = cx.theme();
    let fact = |name: &'static str, value: String| {
        div()
            .flex()
            .justify_between()
            .gap_2()
            .child(div().text_color(theme.colors.fg_muted).child(name))
            .child(tabular(div()).child(value))
    };
    let sized = format!(
        "{}x{}",
        number(bounds.size.width.into()),
        number(bounds.size.height.into())
    );
    let core = div()
        .debug_selector(move || {
            format!(
                "inspector-content-{}x{}",
                number(content.width.into()),
                number(content.height.into())
            )
        })
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Sm))
        .border_1()
        .border_color(theme.colors.hue(CONTENT, "inspector"))
        .bg(theme.colors.hue(CONTENT, "inspector").alpha(0.12))
        .child(tabular(div()).child(pair(content.width, content.height)))
        .into_any_element();
    let padded = ring("padding", layers.padding, PADDING, core, cx);
    let bordered = ring("border", layers.border, BORDER, padded, cx);
    let model = ring("margin", layers.margin, MARGIN, bordered, cx);
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_size(theme.text_size(TextSize::Xs))
        // Captured bounds belong to the consumer, never to its detached panel.
        .when(
            inspector_target(window, cx) == window.window_handle(),
            |body| {
                body.child(raise(
                    "ely-inspector-overlay",
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, cx| paint_layers(bounds, layers, window, cx),
                    )
                    .absolute()
                    .top_0()
                    .left_0(),
                ))
            },
        )
        .child(model)
        .child(
            div()
                .debug_selector(move || format!("inspector-size-{sized}"))
                .flex()
                .flex_col()
                .gap_1()
                .child(fact(
                    "Place",
                    format!(
                        "{}, {}",
                        number(bounds.origin.x.into()),
                        number(bounds.origin.y.into())
                    ),
                ))
                .child(fact("Size", pair(bounds.size.width, bounds.size.height)))
                .child(fact("Children", pair(children.width, children.height))),
        )
        .into_any_element()
}

fn edit_row(
    label: &'static str,
    id: &'static str,
    value: f32,
    range: (f64, f64),
    step: f64,
    precision: usize,
    edit: EditableStyle,
    selected: InspectorElementId,
    edits: Entity<StyleEditState>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let input_id = format!("{id}-{selected:?}");
    let unit = if matches!(edit, EditableStyle::Opacity) {
        ""
    } else {
        " px"
    };
    let shown = format!("{value:.precision$}{unit}");
    div()
        .debug_selector(move || format!("inspector-config-{id}"))
        .flex()
        .items_center()
        .gap_3()
        .h(theme.control_height(ControlSize::Md))
        .child(
            div()
                .w_16()
                .flex_none()
                .text_color(theme.colors.fg_muted)
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .debug_selector(move || format!("inspector-slider-{id}"))
                .child(
                    Slider::new(input_id, value as f64)
                        .label(label)
                        .range(range.0, range.1.max(value as f64))
                        .step(step)
                        .on_change(move |value, window, cx| {
                            apply_edit(&selected, edit, value as f32, &edits, window, cx)
                        }),
                ),
        )
        .child(
            tabular(div())
                .w(gpui::rems(3.5))
                .flex_none()
                .text_right()
                .text_size(theme.text_size(TextSize::Xs))
                .child(shown),
        )
        .into_any_element()
}

/// A color row: its name, a swatch that opens the picker, and the hex it reads.
/// `fallback` seeds the picker when the box writes no color of its own.
fn color_row(
    label: &'static str,
    target: ColorTarget,
    current: Option<Hsla>,
    fallback: Hsla,
    selected: InspectorElementId,
    edits: Entity<StyleEditState>,
    picker: &Entity<Option<ColorTarget>>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let control = match target {
        ColorTarget::Background => "ely-inspector-background",
        ColorTarget::Text => "ely-inspector-text-color",
    };
    let expanded = picker.read(cx).as_ref() == Some(&target);
    let shown = current.unwrap_or(fallback);
    let toggle = picker.clone();
    let picker_id = selected.clone();
    let picker_edits = edits.clone();
    div()
        .debug_selector(move || format!("inspector-config-{control}"))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .h(theme.control_height(ControlSize::Md))
                .child(
                    div()
                        .w_16()
                        .flex_none()
                        .text_color(theme.colors.fg_muted)
                        .child(label),
                )
                .child(
                    div()
                        .debug_selector(move || format!("inspector-swatch-{control}"))
                        .child(
                            ColorSwatch::new(format!("{control}-{selected:?}"), shown)
                                .size(ControlSize::Sm)
                                .selected(expanded)
                                .tooltip(if expanded {
                                    "Close the picker"
                                } else {
                                    "Pick a color"
                                })
                                .on_click(move |window, cx| {
                                    toggle.update(cx, |expanded, _| {
                                        *expanded = (*expanded != Some(target)).then_some(target);
                                    });
                                    window.refresh();
                                }),
                        ),
                )
                .child(
                    tabular(div())
                        .flex_1()
                        .min_w_0()
                        .text_right()
                        .text_size(theme.text_size(TextSize::Xs))
                        .child(describe_color(current)),
                ),
        )
        .when(expanded, |row| {
            row.child(
                div()
                    .debug_selector(move || format!("inspector-picker-{control}"))
                    .child(
                        ColorPicker::new(format!("{control}-picker-{picker_id:?}"), shown)
                            .on_change(move |color, window, cx| {
                                apply_color(&picker_id, target, color, &picker_edits, window, cx)
                            }),
                    ),
            )
        })
        .into_any_element()
}

fn configuration_panel(
    id: InspectorElementId,
    state: &DivInspectorState,
    edits: &Entity<StyleEditState>,
    picker: &Entity<Option<ColorTarget>>,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let rem = window.rem_size();
    let theme = cx.theme();
    let original = edits.read(cx).original_for(&id);
    let changes = original
        .as_ref()
        .and_then(|(original, size)| {
            style_configuration(&id, original, &state.base_style, *size, rem)
        })
        .map(|configuration| configuration.changes)
        .unwrap_or_default();
    let row = |label, control_id, value, range, step, precision, edit| {
        edit_row(
            label,
            control_id,
            value,
            range,
            step,
            precision,
            edit,
            id.clone(),
            edits.clone(),
            cx,
        )
    };
    let reset_id = id.clone();
    let reset_state = edits.clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.bg)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Properties"))
                .child(
                    div().debug_selector(|| "inspector-reset".into()).child(
                        Button::new("ely-inspector-reset", "Reset")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .disabled(changes.is_empty())
                            .on_click(move |_, window, cx| {
                                reset_edits(&reset_id, &reset_state, window, cx)
                            }),
                    ),
                ),
        )
        .child(row(
            "Width",
            "ely-inspector-width",
            length_pixels(state.base_style.size.width, state.bounds.size.width, rem),
            (1.0, 4096.0),
            1.0,
            0,
            EditableStyle::Width,
        ))
        .child(row(
            "Height",
            "ely-inspector-height",
            length_pixels(state.base_style.size.height, state.bounds.size.height, rem),
            (1.0, 4096.0),
            1.0,
            0,
            EditableStyle::Height,
        ))
        .child(row(
            "Padding",
            "ely-inspector-padding",
            uniform_definite(&state.base_style, rem),
            (0.0, 512.0),
            1.0,
            0,
            EditableStyle::Padding,
        ))
        .child(row(
            "Margin",
            "ely-inspector-margin",
            uniform_length(&state.base_style, rem),
            (-512.0, 512.0),
            1.0,
            0,
            EditableStyle::Margin,
        ))
        .child(row(
            "Border",
            "ely-inspector-border",
            uniform_border(&state.base_style, rem),
            (0.0, 64.0),
            1.0,
            0,
            EditableStyle::Border,
        ))
        .child(row(
            "Opacity",
            "ely-inspector-opacity",
            state.base_style.opacity.unwrap_or(1.0),
            (0.0, 1.0),
            0.1,
            1,
            EditableStyle::Opacity,
        ))
        .child(color_row(
            "Surface",
            ColorTarget::Background,
            ColorTarget::Background.read(&state.base_style),
            theme.colors.surface,
            id.clone(),
            edits.clone(),
            picker,
            cx,
        ))
        .child(color_row(
            "Text",
            ColorTarget::Text,
            ColorTarget::Text.read(&state.base_style),
            theme.colors.fg,
            id.clone(),
            edits.clone(),
            picker,
            cx,
        ))
        .children((!changes.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .pt_1()
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.colors.accent)
                        .child("Active overrides"),
                )
                .children(changes.into_iter().map(|change| {
                    div()
                        .px_2()
                        .py_1()
                        .rounded(theme.radius(Radius::Sm))
                        .bg(theme.colors.accent.alpha(0.1))
                        .text_size(theme.text_size(TextSize::Xs))
                        .child(format!(
                            "{}: {} → {}",
                            change.property, change.before, change.after
                        ))
                }))
        }))
        .into_any_element()
}

/// The inspector's panel: how to pick, the picked box's source and ids, and its model.
pub fn inspector_panel(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
    footer: impl FnOnce(&Inspector, &mut Window, &mut Context<Inspector>) -> Option<AnyElement>,
) -> AnyElement {
    inspector_panel_with_header(inspector, window, cx, None, footer)
}

pub fn inspector_panel_with_header(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
    header: Option<AnyElement>,
    footer: impl FnOnce(&Inspector, &mut Window, &mut Context<Inspector>) -> Option<AnyElement>,
) -> AnyElement {
    let picking = inspector.is_picking();
    if picking {
        // gpui holds a pick on a press without a redraw, so the panel draws each frame while picking.
        window.request_animation_frame();
    }
    let place = inspector.active_element_id().map(|id| {
        let ids: Vec<String> = id.path.global_id.iter().map(ToString::to_string).collect();
        (
            id.path.source_location,
            ids.iter()
                .rev()
                .find(|id| {
                    !id.contains("::") && !id.starts_with("view-") && id.as_str() != "FocusHandle"
                })
                .map(|id| {
                    id.strip_prefix("agentation-target-")
                        .unwrap_or(id)
                        .to_string()
                })
                .unwrap_or_else(|| "GPUI element".into()),
        )
    });
    let source_path = place
        .as_ref()
        .map(|(location, _)| source::breadcrumb(location, window, cx));
    let states = inspector.render_inspector_states(window, cx);
    let footer = footer(inspector, window, cx);
    let focus = window
        .use_keyed_state("ely-inspector-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let owner = cx.entity();
    let theme = cx.theme();
    let colors = &theme.colors;
    // A hosted inspector inherits its window's surface and rounded outer edge.
    let panel_background = if inspector_target(window, cx) == window.window_handle() {
        colors.surface
    } else {
        transparent_black()
    };
    let hint = match picking {
        true => {
            "Click an element to edit it. Scroll to select a parent. Press Esc to exit picking."
        }
        false if header.is_some() => "Selected · Properties save to this inspection annotation.",
        false => "Selected · Changes preview live. Pick selects another element.",
    };
    let body = div()
        .id("ely-inspector")
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(panel_background)
        .border_l_1()
        .border_color(colors.border)
        .text_color(colors.fg)
        .text_size(theme.text_size(TextSize::Sm))
        .child(
            div()
                .debug_selector(|| "inspector-header".into())
                .flex()
                .flex_col()
                .gap_1()
                .flex_none()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(colors.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("Inspector"))
                        .children(header.or_else(|| {
                            (!picking).then(|| {
                                div().debug_selector(|| "inspector-pick".into()).child(
                                    Button::new("ely-inspector-pick", "Pick")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ControlSize::Sm)
                                        .on_click(move |_, window, cx| {
                                            owner.update(cx, |inspector, cx| {
                                                inspector.start_picking();
                                                cx.notify();
                                            });
                                            let target = inspector_target(window, cx);
                                            if target != window.window_handle() {
                                                let _ =
                                                    target.update(cx, |_, host, _| host.refresh());
                                            }
                                            log::info!("inspector: picking");
                                            window.refresh();
                                        }),
                                )
                            }).map(IntoElement::into_any_element)
                        })),
                )
                .child(
                    div()
                        .debug_selector(move || format!("inspector-picking-{picking}"))
                        .text_color(colors.fg_muted)
                        .text_size(theme.text_size(TextSize::Xs))
                        .child(hint),
                ),
        )
        .child(
            div()
                .id("ely-inspector-details")
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .children(source_path)
                .children(place.map(|(_, ids)| {
                    div().flex().flex_col().gap_1().child(
                        literal(div())
                            .text_color(colors.fg_muted)
                            .child(Ellipsis::new(ids)),
                    )
                }))
                .children(states),
        )
        .children(footer);
    // A second root, out of the app's scope: it owns Tab itself.
    FocusScope::new(&focus)
        .size_full()
        .child(body)
        .into_any_element()
}

/// Draws gpui's inspector, which debug builds have, in Ely's look. `window.toggle_inspector` opens it at the window's right, and the page gives up its width. Hovering picks the box under the pointer, the wheel steps out to the boxes around it, and a press holds it; the picked box shows its margin, border, padding and size over the page, and its model in the panel.
pub fn install_inspector(cx: &mut App) {
    register_inspector_element(cx);
    if service::claim_renderer(service::RendererOwner::Default, cx) {
        cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
            inspector_panel(inspector, window, cx, |_, _, _| None)
        }));
    }
}

/// Register the Ely controls without taking ownership of the window renderer.
pub(crate) fn register_inspector_element(cx: &mut App) {
    if !service::register_element_renderer(cx) { return; }
    cx.register_inspector_element(move |_window, cx| {
        // Each inspector owns its baseline; windows must never share edits.
        let edits = cx.new(|_| StyleEditState::default());
        let picker = cx.new(|_| None::<ColorTarget>);
        move |id, state: &DivInspectorState, window: &mut Window, cx: &mut App| {
            if service::capture(id.clone(), state, &edits, window, cx) {
                picker.update(cx, |expanded, _| *expanded = None);
            }
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(configuration_panel(id, state, &edits, &picker, window, cx))
                .child(box_model(state, window, cx))
                .into_any_element()
        }
    });
    log::info!("inspector: installed");
}

