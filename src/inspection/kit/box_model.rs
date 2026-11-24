//! Box-model drawing retained from the attributed inspector; colors use Kit.
use super::*;
const MARGIN: usize = 0;
const BORDER: usize = 1;
const PADDING: usize = 2;
const CONTENT: usize = 3;
fn layer_color(layer: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    [theme.red, theme.yellow, theme.green, theme.blue][layer]
}
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
    let colors = theme;
    let hue = |hue| layer_color(hue, cx);
    let band = |bounds, widths, hue| {
        quad(
            bounds,
            Corners::default(),
            transparent_black(),
            widths,
            layer_color(hue, cx).opacity(0.4),
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
    window.paint_quad(fill(shrink(inner, padding), hue(CONTENT).opacity(0.25)));
    let text = px(12.);
    let label = format!(
        "{} × {}",
        number(bounds.size.width.into()),
        number(bounds.size.height.into())
    );
    let run = TextRun {
        len: label.len(),
        font: window.text_style().font(),
        color: colors.background,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(label.into(), text, &[run], None);
    let (height, pad) = (text * 1.5, text / 2.0);
    let top = (outer.top() - height).max(Pixels::ZERO);
    let tag = Bounds::new(
        point(outer.left(), top),
        size(line.width + pad * 2.0, height),
    );
    let radius = theme.radius;
    window.paint_quad(fill(tag, colors.foreground).corner_radii(radius));
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
    let color = layer_color(hue, cx);
    let value = |side: Side| div().font_family("monospace").child(side.to_string());
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
        .rounded(theme.radius)
        .border_1()
        .border_color(color)
        .bg(color.opacity(0.12))
        .child(
            div()
                .absolute()
                .top_1()
                .left_2()
                .text_color(theme.muted_foreground)
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
pub(super) fn box_model(
    state: &DivInspectorState,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
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
            .child(div().text_color(theme.muted_foreground).child(name))
            .child(div().font_family("monospace").child(value))
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
        .rounded(theme.radius)
        .border_1()
        .border_color(layer_color(CONTENT, cx))
        .bg(layer_color(CONTENT, cx).opacity(0.12))
        .child(
            div()
                .font_family("monospace")
                .child(pair(content.width, content.height)),
        )
        .into_any_element();
    let padded = ring("padding", layers.padding, PADDING, core, cx);
    let bordered = ring("border", layers.border, BORDER, padded, cx);
    let model = ring("margin", layers.margin, MARGIN, bordered, cx);
    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_size(px(12.))
        // Captured bounds belong to the consumer, never to its detached panel.
        .when(
            inspector_target(window, cx) == window.window_handle(),
            |body| {
                body.child(
                    div().child(
                        canvas(
                            |_, _, _| {},
                            move |_, _, window, cx| paint_layers(bounds, layers, window, cx),
                        )
                        .absolute()
                        .top_0()
                        .left_0(),
                    ),
                )
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
