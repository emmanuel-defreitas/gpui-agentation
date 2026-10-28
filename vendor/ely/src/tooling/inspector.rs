pub(crate) mod source;

use std::{collections::HashMap, fmt};

use gpui::{
    AbsoluteLength, AnyElement, App, AppContext, BorderStyle, Bounds, Context, Corners,
    DefiniteLength, DivInspectorState, Edges, Entity, FontWeight, Global, Inspector,
    InspectorElementId, InteractiveElement, IntoElement, Length, ParentElement, Pixels,
    StatefulInteractiveElement, StyleRefinement, Styled, TextAlign, TextRun, Window, WindowId,
    canvas, div, fill, point, prelude::FluentBuilder, quad, size, transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Slider,
    primitives::{FocusScope, raise},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, LEADING, literal, tabular},
};

// Chart hues Rust, Ochre, Green and Blue, as browsers color a box's layers.
const MARGIN: usize = 6;
const BORDER: usize = 2;
const PADDING: usize = 5;
const CONTENT: usize = 0;

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorStyleChange {
    pub property: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorStyleConfiguration {
    pub source: String,
    pub element_path: String,
    pub changes: Vec<InspectorStyleChange>,
    pub rust: String,
}

/// Snapshot of the element displayed in this window's inspector sidebar.
#[derive(Clone)]
pub struct InspectorSelection {
    pub id: InspectorElementId,
    pub bounds: Bounds<Pixels>,
}

#[derive(Default)]
struct InspectorConfiguration {
    selections: HashMap<WindowId, InspectorSelection>,
    configurations: HashMap<WindowId, InspectorStyleConfiguration>,
}
impl Global for InspectorConfiguration {}

#[cfg(agentation_external_inspector)]
#[derive(Default)]
struct InspectorTargets {
    windows: HashMap<WindowId, gpui::AnyWindowHandle>,
    controllers: HashMap<WindowId, gpui::EntityId>,
}
#[cfg(agentation_external_inspector)]
impl Global for InspectorTargets {}

/// Route a detached inspector's controls to the consumer's native picker.
#[cfg(agentation_external_inspector)]
pub fn set_inspector_target(view: WindowId, target: Option<gpui::AnyWindowHandle>, cx: &mut App) {
    let targets = cx.default_global::<InspectorTargets>();
    if let Some(target) = target {
        targets.windows.insert(view, target);
    } else {
        targets.windows.remove(&view);
        targets.controllers.remove(&view);
    }
}

pub fn inspector_target(window: &Window, _cx: &App) -> gpui::AnyWindowHandle {
    #[cfg(agentation_external_inspector)]
    if let Some(target) = _cx
        .try_global::<InspectorTargets>()
        .and_then(|targets| targets.windows.get(&window.window_handle().window_id()))
    {
        return *target;
    }
    window.window_handle()
}

pub fn active_inspector_configuration(
    window: &Window,
    cx: &App,
) -> Option<InspectorStyleConfiguration> {
    cx.try_global::<InspectorConfiguration>()?
        .configurations
        .get(&window.window_handle().window_id())
        .cloned()
}

pub fn inspector_selection(window: &Window, cx: &App) -> Option<InspectorSelection> {
    cx.try_global::<InspectorConfiguration>()?
        .selections
        .get(&window.window_handle().window_id())
        .cloned()
}

pub fn clear_inspector_selection(window: &Window, cx: &mut App) {
    if cx.has_global::<InspectorConfiguration>() {
        let state = cx.global_mut::<InspectorConfiguration>();
        let id = window.window_handle().window_id();
        state.selections.remove(&id);
        state.configurations.remove(&id);
    }
}

#[derive(Debug, Default)]
struct StyleEditState {
    selected: Option<InspectorElementId>,
    original_style: Option<StyleRefinement>,
    original_size: Option<gpui::Size<Pixels>>,
}

impl StyleEditState {
    fn select(
        &mut self,
        id: InspectorElementId,
        style: &StyleRefinement,
        size: gpui::Size<Pixels>,
    ) -> bool {
        if self.selected.as_ref() == Some(&id) {
            return false;
        }
        self.selected = Some(id);
        self.original_style = Some(style.clone());
        self.original_size = Some(size);
        true
    }

    fn original_for(
        &self,
        id: &InspectorElementId,
    ) -> Option<(StyleRefinement, gpui::Size<Pixels>)> {
        (self.selected.as_ref() == Some(id))
            .then(|| self.original_style.clone().zip(self.original_size))
            .flatten()
    }
}

#[derive(Clone, Copy)]
enum EditableStyle {
    Width,
    Height,
    Padding,
    Margin,
    Border,
    Opacity,
}

/// One side of a box as written: pixels, `auto`, or a share of the parent's width.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Px(f32),
    Auto,
    Share(f32),
}

impl Side {
    /// What the overlay draws: `auto` and shares only layout knows, so they draw nothing.
    fn drawn(self) -> Pixels {
        match self {
            Side::Px(px) => Pixels::from(px),
            Side::Auto | Side::Share(_) => Pixels::ZERO,
        }
    }
}

fn number(value: f32) -> String {
    ((value * 10.0).round() / 10.0).to_string()
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Px(px) => write!(f, "{}", number(*px)),
            Side::Auto => write!(f, "auto"),
            Side::Share(share) => write!(f, "{}%", number(share * 100.0)),
        }
    }
}

/// A box's margin, border and padding, each top, right, bottom, left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layers {
    margin: [Side; 4],
    border: [Side; 4],
    padding: [Side; 4],
}

/// The layers as the style writes them, rems at `rem`.
fn layers(style: &StyleRefinement, rem: Pixels) -> Layers {
    let definite = |length: DefiniteLength| match length {
        DefiniteLength::Absolute(length) => Side::Px(length.to_pixels(rem).into()),
        DefiniteLength::Fraction(share) => Side::Share(share),
    };
    let margin = |length: Option<Length>| match length {
        None => Side::Px(0.0),
        Some(Length::Auto) => Side::Auto,
        Some(Length::Definite(length)) => definite(length),
    };
    let padding = |length: Option<DefiniteLength>| length.map_or(Side::Px(0.0), definite);
    let border = |length: Option<AbsoluteLength>| {
        Side::Px(length.map_or(0.0, |length| length.to_pixels(rem).into()))
    };
    let (m, b, p) = (&style.margin, &style.border_widths, &style.padding);
    Layers {
        margin: [m.top, m.right, m.bottom, m.left].map(margin),
        border: [b.top, b.right, b.bottom, b.left].map(border),
        padding: [p.top, p.right, p.bottom, p.left].map(padding),
    }
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
        .when(inspector_target(window, cx) == window.window_handle(), |body| {
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
        })
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

fn definite_pixels(value: Option<DefiniteLength>, fallback: Pixels, rem: Pixels) -> f32 {
    match value {
        Some(DefiniteLength::Absolute(value)) => value.to_pixels(rem).into(),
        Some(DefiniteLength::Fraction(_)) | None => fallback.into(),
    }
}

fn length_pixels(value: Option<Length>, fallback: Pixels, rem: Pixels) -> f32 {
    match value {
        Some(Length::Definite(value)) => definite_pixels(Some(value), fallback, rem),
        Some(Length::Auto) | None => fallback.into(),
    }
}

fn absolute_pixels(value: Option<AbsoluteLength>, rem: Pixels) -> f32 {
    value.map_or(0.0, |value| value.to_pixels(rem).into())
}

fn uniform_definite(style: &StyleRefinement, rem: Pixels) -> f32 {
    definite_pixels(style.padding.top, Pixels::ZERO, rem)
}

fn uniform_length(style: &StyleRefinement, rem: Pixels) -> f32 {
    length_pixels(style.margin.top, Pixels::ZERO, rem)
}

fn uniform_border(style: &StyleRefinement, rem: Pixels) -> f32 {
    absolute_pixels(style.border_widths.top, rem)
}

fn set_edit(style: &mut StyleRefinement, edit: EditableStyle, value: f32) {
    match edit {
        EditableStyle::Width => style.size.width = Some(gpui::px(value).into()),
        EditableStyle::Height => style.size.height = Some(gpui::px(value).into()),
        EditableStyle::Padding => {
            let value: DefiniteLength = gpui::px(value).into();
            style.padding.top = Some(value);
            style.padding.right = Some(value);
            style.padding.bottom = Some(value);
            style.padding.left = Some(value);
        }
        EditableStyle::Margin => {
            let value: Length = gpui::px(value).into();
            style.margin.top = Some(value);
            style.margin.right = Some(value);
            style.margin.bottom = Some(value);
            style.margin.left = Some(value);
        }
        EditableStyle::Border => {
            let value: AbsoluteLength = gpui::px(value).into();
            style.border_widths.top = Some(value);
            style.border_widths.right = Some(value);
            style.border_widths.bottom = Some(value);
            style.border_widths.left = Some(value);
        }
        EditableStyle::Opacity => style.opacity = Some(value.clamp(0.0, 1.0)),
    }
}

fn style_configuration(
    id: &InspectorElementId,
    original: &StyleRefinement,
    current: &StyleRefinement,
    original_size: gpui::Size<Pixels>,
    rem: Pixels,
) -> Option<InspectorStyleConfiguration> {
    let mut changes = Vec::new();
    let mut rust = vec!["div()".to_string()];
    let mut changed = |property: &str, before: String, after: String, line: String| {
        if before != after {
            changes.push(InspectorStyleChange {
                property: property.into(),
                before,
                after,
            });
            rust.push(line);
        }
    };
    let original_width = length_pixels(original.size.width, original_size.width, rem);
    let current_width = length_pixels(current.size.width, original_size.width, rem);
    changed(
        "Width",
        format!("{}px", number(original_width)),
        format!("{}px", number(current_width)),
        format!("  .w(px({current_width:.1}))"),
    );
    let original_height = length_pixels(original.size.height, original_size.height, rem);
    let current_height = length_pixels(current.size.height, original_size.height, rem);
    changed(
        "Height",
        format!("{}px", number(original_height)),
        format!("{}px", number(current_height)),
        format!("  .h(px({current_height:.1}))"),
    );
    let before_layers = layers(original, rem);
    let after_layers = layers(current, rem);
    for (property, before, after, method, value) in [
        (
            "Padding",
            before_layers.padding,
            after_layers.padding,
            "p",
            uniform_definite(current, rem),
        ),
        (
            "Margin",
            before_layers.margin,
            after_layers.margin,
            "m",
            uniform_length(current, rem),
        ),
        (
            "Border",
            before_layers.border,
            after_layers.border,
            "border",
            uniform_border(current, rem),
        ),
    ] {
        let describe = |sides: [Side; 4]| sides.map(|side| side.to_string()).join(" / ");
        changed(
            property,
            describe(before),
            describe(after),
            format!("  .{method}(px({value:.1}))"),
        );
    }
    let before = original.opacity.unwrap_or(1.0);
    let after = current.opacity.unwrap_or(1.0);
    changed(
        "Opacity",
        number(before),
        number(after),
        format!("  .opacity({after:.1})"),
    );
    if changes.is_empty() {
        return None;
    }
    let location = id.path.source_location;
    Some(InspectorStyleConfiguration {
        source: format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        ),
        element_path: id.path.global_id.to_string(),
        changes,
        rust: rust.join("\n"),
    })
}

fn publish_configuration(
    id: &InspectorElementId,
    current: &StyleRefinement,
    edits: &Entity<StyleEditState>,
    rem: Pixels,
    window: &Window,
    cx: &mut App,
) {
    let configuration = edits
        .read(cx)
        .original_for(id)
        .and_then(|(original, size)| style_configuration(id, &original, current, size, rem));
    let configurations = &mut cx.global_mut::<InspectorConfiguration>().configurations;
    let window_id = window.window_handle().window_id();
    if let Some(configuration) = configuration {
        configurations.insert(window_id, configuration);
    } else {
        configurations.remove(&window_id);
    }
}

fn apply_edit(
    id: &InspectorElementId,
    edit: EditableStyle,
    value: f32,
    edits: &Entity<StyleEditState>,
    window: &mut Window,
    cx: &mut App,
) {
    let target = inspector_target(window, cx);
    if target != window.window_handle() {
        let _ = target.update(cx, |_, window, cx| {
            apply_edit(id, edit, value, edits, window, cx)
        });
        window.refresh();
        return;
    }
    let rem = window.rem_size();
    let mut current = None;
    let _ = window.with_inspector_state::<DivInspectorState, _>(Some(id), cx, |state, _| {
        if let Some(state) = state {
            set_edit(&mut state.base_style, edit, value);
            current = Some((*state.base_style).clone());
        }
    });
    if let Some(current) = current {
        publish_configuration(id, &current, edits, rem, window, cx);
        window.refresh();
    }
}

fn reset_edits(
    id: &InspectorElementId,
    edits: &Entity<StyleEditState>,
    window: &mut Window,
    cx: &mut App,
) {
    let target = inspector_target(window, cx);
    if target != window.window_handle() {
        let _ = target.update(cx, |_, window, cx| reset_edits(id, edits, window, cx));
        window.refresh();
        return;
    }
    let original = edits.read(cx).original_for(id).map(|(style, _)| style);
    let _ = window.with_inspector_state::<DivInspectorState, _>(Some(id), cx, |state, _| {
        if let (Some(state), Some(original)) = (state, original) {
            *state.base_style = original;
        }
    });
    cx.global_mut::<InspectorConfiguration>()
        .configurations
        .remove(&window.window_handle().window_id());
    window.refresh();
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

fn configuration_panel(
    id: InspectorElementId,
    state: &DivInspectorState,
    edits: &Entity<StyleEditState>,
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
    #[cfg(agentation_external_inspector)]
    {
        let controller_id = cx.entity_id();
        cx.default_global::<InspectorTargets>()
            .controllers
            .insert(window.window_handle().window_id(), controller_id);
    }
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
                        .children((!picking).then(|| {
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
                                            let _ = target.update(cx, |_, host, _| host.refresh());
                                        }
                                        log::info!("inspector: picking");
                                        window.refresh();
                                    }),
                            )
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
    if !cx.has_global::<InspectorConfiguration>() {
        cx.set_global(InspectorConfiguration::default());
    }
    cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
        inspector_panel(inspector, window, cx, |_, _, _| None)
    }));
    cx.register_inspector_element(move |_window, cx| {
        #[cfg(agentation_external_inspector)]
        let controller_id = cx
            .try_global::<InspectorTargets>()
            .and_then(|targets| {
                targets
                    .controllers
                    .get(&_window.window_handle().window_id())
            })
            .copied();
        // Each inspector owns its baseline; windows must never share edits.
        let edits = cx.new(|_| StyleEditState::default());
        move |id, state: &DivInspectorState, window: &mut Window, cx: &mut App| {
            let target = inspector_target(window, cx);
            #[cfg(agentation_external_inspector)]
            let active = target == window.window_handle()
                || target
                    .update(cx, |_, host, _| {
                        host.inspector_entity()
                            .is_some_and(|active| Some(active.entity_id()) == controller_id)
                    })
                    .unwrap_or(false);
            #[cfg(not(agentation_external_inspector))]
            let active = true;
            if active {
                let target_id = target.window_id();
                if edits.update(cx, |edits, _| {
                    edits.select(id.clone(), &state.base_style, state.bounds.size)
                }) {
                    let configuration = cx.global_mut::<InspectorConfiguration>();
                    configuration.selections.remove(&target_id);
                    configuration.configurations.remove(&target_id);
                }
                cx.global_mut::<InspectorConfiguration>().selections.insert(
                    target_id,
                    InspectorSelection {
                        id: id.clone(),
                        bounds: state.bounds,
                    },
                );
            }
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(configuration_panel(id, state, &edits, window, cx))
                .child(box_model(state, window, cx))
                .into_any_element()
        }
    });
    log::info!("inspector: installed");
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{
        GlobalElementId, InspectorElementId, InspectorElementPath, StyleRefinement, Styled, div,
        px, relative, rems, size,
    };

    use super::{EditableStyle, Side, layers, set_edit, style_configuration};

    #[test]
    fn layers_read_as_written() {
        let mut style = div().mt(px(4.0)).mx_auto().p(rems(0.5)).border_2();
        let written = layers(style.style(), px(16.0));
        assert_eq!(
            written.margin,
            [Side::Px(4.0), Side::Auto, Side::Px(0.0), Side::Auto]
        );
        assert_eq!(written.padding, [Side::Px(8.0); 4]);
        assert_eq!(written.border, [Side::Px(2.0); 4]);
        let bare = layers(&StyleRefinement::default(), px(16.0));
        assert_eq!(
            bare.border,
            [Side::Px(0.0); 4],
            "no border written, none read"
        );
        let mut share = StyleRefinement::default();
        share.padding.left = Some(relative(0.25));
        let shown = layers(&share, px(16.0))
            .padding
            .map(|side| side.to_string());
        assert_eq!(shown, ["0", "0", "0", "25%"]);
        assert_eq!(Side::Share(0.25).drawn(), px(0.0), "a share draws nothing");
    }

    #[test]
    fn visual_edits_export_only_changed_gpui_configuration() {
        let original = StyleRefinement::default();
        let mut current = original.clone();
        set_edit(&mut current, EditableStyle::Width, 140.0);
        set_edit(&mut current, EditableStyle::Padding, 12.0);
        set_edit(&mut current, EditableStyle::Opacity, 0.5);
        let id = InspectorElementId {
            path: Rc::new(InspectorElementPath {
                global_id: GlobalElementId::default(),
                source_location: std::panic::Location::caller(),
            }),
            instance_id: 0,
        };

        let exported = style_configuration(
            &id,
            &original,
            &current,
            size(px(100.0), px(40.0)),
            px(16.0),
        )
        .expect("changed styles export");

        assert_eq!(
            exported
                .changes
                .iter()
                .map(|change| change.property.as_str())
                .collect::<Vec<_>>(),
            ["Width", "Padding", "Opacity"]
        );
        assert!(exported.rust.contains(".w(px(140.0))"));
        assert!(exported.rust.contains(".p(px(12.0))"));
        assert!(exported.rust.contains(".opacity(0.5)"));
    }
}
