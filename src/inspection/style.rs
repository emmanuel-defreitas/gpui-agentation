//! Inspector editing and snapshots, extracted from the patched previous presentation inspector.
//! MIT provenance: ORIGIN.md. No UI component types cross this boundary.
use super::{InspectorStyleChange, InspectorStyleConfiguration, State, inspector_target};
use gpui::{
    AbsoluteLength, App, DefiniteLength, DivInspectorState, Entity, Hsla, InspectorElementId,
    Length, Pixels, Rgba, StyleRefinement, Window,
};
use std::fmt;

#[derive(Debug, Default)]
pub(crate) struct StyleEditState {
    selected: Option<InspectorElementId>,
    original_style: Option<StyleRefinement>,
    original_size: Option<gpui::Size<Pixels>>,
}

impl StyleEditState {
    pub(crate) fn select(
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

    pub(crate) fn original_for(
        &self,
        id: &InspectorElementId,
    ) -> Option<(StyleRefinement, gpui::Size<Pixels>)> {
        (self.selected.as_ref() == Some(id))
            .then(|| self.original_style.clone().zip(self.original_size))
            .flatten()
    }
}

#[derive(Clone, Copy)]
pub(crate) enum EditableStyle {
    Width,
    Height,
    Padding,
    Margin,
    Border,
    Opacity,
}

/// A color a box writes: its surface, or the text it sets for its children.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ColorTarget {
    Background,
    Text,
}

impl ColorTarget {
    const ALL: [Self; 2] = [Self::Background, Self::Text];

    pub(crate) fn property(self) -> &'static str {
        match self {
            Self::Background => "Background",
            Self::Text => "Text color",
        }
    }

    fn from_property(property: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|target| target.property() == property)
    }

    /// The solid color written, if any. Gradients and inherited colors read as `None`.
    pub(crate) fn read(self, style: &StyleRefinement) -> Option<Hsla> {
        match self {
            Self::Background => style
                .background
                .as_ref()
                .and_then(|fill| fill.color())
                .and_then(|background| background.as_solid()),
            Self::Text => style.text.color,
        }
    }

    fn write(self, style: &mut StyleRefinement, color: Hsla) {
        match self {
            Self::Background => style.background = Some(color.into()),
            Self::Text => style.text.color = Some(color),
        }
    }

    fn rust(self, color: Hsla) -> String {
        let method = match self {
            Self::Background => "bg",
            Self::Text => "text_color",
        };
        format!("  .{method}({})", rust_color(color))
    }
}

/// `none` for an unwritten color, else its hex as the picker shows it.
pub(crate) fn describe_color(color: Option<Hsla>) -> String {
    color.map_or_else(|| "none".to_string(), |color| hex(color.to_rgb()))
}

/// The gpui constructor for a color: `rgb(0xrrggbb)`, or `rgba(0xrrggbbaa)` when translucent.
pub fn rust_color(color: Hsla) -> String {
    let rgba = color.to_rgb();
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    let (r, g, b, a) = (byte(rgba.r), byte(rgba.g), byte(rgba.b), byte(rgba.a));
    if a == 255 {
        format!("rgb(0x{:06x})", (r << 16) | (g << 8) | b)
    } else {
        format!("rgba(0x{:08x})", (r << 24) | (g << 16) | (b << 8) | a)
    }
}

/// One side of a box as written: pixels, `auto`, or a share of the parent's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Side {
    Px(f32),
    Auto,
    Share(f32),
}

impl Side {
    /// What the overlay draws: `auto` and shares only layout knows, so they draw nothing.
    pub(crate) fn drawn(self) -> Pixels {
        match self {
            Side::Px(px) => Pixels::from(px),
            Side::Auto | Side::Share(_) => Pixels::ZERO,
        }
    }
}

pub(crate) fn number(value: f32) -> String {
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
pub(crate) struct Layers {
    pub(crate) margin: [Side; 4],
    pub(crate) border: [Side; 4],
    pub(crate) padding: [Side; 4],
}

/// The layers as the style writes them, rems at `rem`.
pub(crate) fn layers(style: &StyleRefinement, rem: Pixels) -> Layers {
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

fn definite_pixels(value: Option<DefiniteLength>, fallback: Pixels, rem: Pixels) -> f32 {
    match value {
        Some(DefiniteLength::Absolute(value)) => value.to_pixels(rem).into(),
        Some(DefiniteLength::Fraction(_)) | None => fallback.into(),
    }
}

pub(crate) fn length_pixels(value: Option<Length>, fallback: Pixels, rem: Pixels) -> f32 {
    match value {
        Some(Length::Definite(value)) => definite_pixels(Some(value), fallback, rem),
        Some(Length::Auto) | None => fallback.into(),
    }
}

fn absolute_pixels(value: Option<AbsoluteLength>, rem: Pixels) -> f32 {
    value.map_or(0.0, |value| value.to_pixels(rem).into())
}

pub(crate) fn uniform_definite(style: &StyleRefinement, rem: Pixels) -> f32 {
    definite_pixels(style.padding.top, Pixels::ZERO, rem)
}

pub(crate) fn uniform_length(style: &StyleRefinement, rem: Pixels) -> f32 {
    length_pixels(style.margin.top, Pixels::ZERO, rem)
}

pub(crate) fn uniform_border(style: &StyleRefinement, rem: Pixels) -> f32 {
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

pub(crate) fn style_configuration(
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
    for target in ColorTarget::ALL {
        let (before, after) = (target.read(original), target.read(current));
        if let Some(after_color) = after.filter(|_| before != after) {
            changed(
                target.property(),
                describe_color(before),
                describe_color(after),
                target.rust(after_color),
            );
        }
    }
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
    let configurations = &mut cx.global_mut::<State>().configurations;
    let window_id = window.window_handle().window_id();
    if let Some(configuration) = configuration {
        configurations.insert(window_id, configuration);
    } else {
        configurations.remove(&window_id);
    }
}

pub(crate) fn apply_edit(
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

pub(crate) fn apply_color(
    id: &InspectorElementId,
    target: ColorTarget,
    color: Hsla,
    edits: &Entity<StyleEditState>,
    window: &mut Window,
    cx: &mut App,
) {
    let host = inspector_target(window, cx);
    if host != window.window_handle() {
        let _ = host.update(cx, |_, window, cx| {
            apply_color(id, target, color, edits, window, cx)
        });
        window.refresh();
        return;
    }
    let rem = window.rem_size();
    let mut current = None;
    let _ = window.with_inspector_state::<DivInspectorState, _>(Some(id), cx, |state, _| {
        if let Some(state) = state {
            target.write(&mut state.base_style, color);
            current = Some((*state.base_style).clone());
        }
    });
    if let Some(current) = current {
        publish_configuration(id, &current, edits, rem, window, cx);
        window.refresh();
    }
}

/// Restore a saved note's overrides after its live element has captured a baseline.
pub fn restore_inspector_configuration(
    id: &InspectorElementId,
    configuration: InspectorStyleConfiguration,
    window: &mut Window,
    cx: &mut App,
) {
    let target = inspector_target(window, cx);
    if target != window.window_handle() {
        let _ = target.update(cx, |_, host, cx| {
            restore_inspector_configuration(id, configuration, host, cx)
        });
        window.refresh();
        return;
    }
    let mut applied = false;
    let _ = window.with_inspector_state::<DivInspectorState, _>(Some(id), cx, |state, _| {
        let Some(state) = state else {
            return;
        };
        for change in &configuration.changes {
            if let Some(target) = ColorTarget::from_property(&change.property) {
                if let Ok(color) = parse_hex(&change.after) {
                    target.write(&mut state.base_style, color.into());
                    applied = true;
                }
                continue;
            }
            let edit = match change.property.as_str() {
                "Width" => EditableStyle::Width,
                "Height" => EditableStyle::Height,
                "Padding" => EditableStyle::Padding,
                "Margin" => EditableStyle::Margin,
                "Border" => EditableStyle::Border,
                "Opacity" => EditableStyle::Opacity,
                _ => continue,
            };
            let Some(value) = change
                .after
                .split('/')
                .next()
                .and_then(|part| part.trim().trim_end_matches("px").parse::<f32>().ok())
                .filter(|value| value.is_finite())
            else {
                continue;
            };
            set_edit(&mut state.base_style, edit, value);
            applied = true;
        }
    });
    if applied {
        cx.global_mut::<State>()
            .configurations
            .insert(window.window_handle().window_id(), configuration);
        window.refresh();
    }
}

pub(crate) fn reset_edits(
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
    cx.global_mut::<State>()
        .configurations
        .remove(&window.window_handle().window_id());
    window.refresh();
}

/// Reads `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`, with or without the hash.
pub(crate) fn parse_hex(text: &str) -> Result<Rgba, String> {
    let digits = text.trim().trim_start_matches('#');
    let doubled = || digits.chars().flat_map(|ch| [ch, ch]);
    let long: String = match digits.len() {
        3 => doubled().chain("ff".chars()).collect(),
        4 => doubled().collect(),
        6 => format!("{digits}ff"),
        8 => digits.to_string(),
        _ => return Err(format!("{text:?} is not a hex color")),
    };
    let value =
        u32::from_str_radix(&long, 16).map_err(|_| format!("{text:?} is not a hex color"))?;
    let channel = |shift: u32| ((value >> shift) & 0xff) as f32 / 255.0;
    Ok(Rgba {
        r: channel(24),
        g: channel(16),
        b: channel(8),
        a: channel(0),
    })
}

/// `#rrggbb`, with alpha appended when it is not opaque.
pub(crate) fn hex(rgba: Rgba) -> String {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let base = format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b)
    );
    if byte(rgba.a) == 255 {
        base
    } else {
        format!("{base}{:02x}", byte(rgba.a))
    }
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
