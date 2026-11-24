//! GPUI Kit presentation of Agentation-owned inspection services.
//! Compiled by the cutover consumer until the atomic native dependency switch.
use crate::inspection::{self as service, inspector_target, style::*};
use gpui::base::FocusTrapElement;
use gpui::component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, ButtonVariants},
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    slider::{Slider, SliderEvent, SliderState, SliderValue},
};
use gpui::{
    AnyElement, App, AppContext, BorderStyle, Bounds, Context, Corners, DivInspectorState, Edges,
    Entity, FontWeight, Hsla, Inspector, InspectorElementId, InteractiveElement, IntoElement,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Subscription, TextAlign, TextRun,
    Window, canvas, div, fill, point, prelude::FluentBuilder, px, quad, size, transparent_black,
};

mod box_model;
pub mod companion;
mod source;

/// A host renderer cannot be inspected/restored through GPUI's public API.
/// Replacement is therefore an explicit host choice, never inferred from Theme.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RendererPolicy {
    /// Leave both the host's renderer and its inspector element factories alone.
    Preserve,
    /// Install Agentation's Kit inspector after the host has initialized Kit.
    Replace,
}

/// Install the presentation once, only when the host explicitly grants ownership.
pub fn install(policy: RendererPolicy, cx: &mut App) {
    if policy == RendererPolicy::Preserve {
        return;
    }
    if !service::claim_renderer(service::RendererOwner::Agentation, cx) {
        return;
    }
    companion::initialize(cx);
    register_element(cx);
    cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
        if let Some(hidden) = companion::render(inspector, window, cx) {
            return hidden;
        }
        panel(inspector, window, cx, None, |_, _, _| None)
    }));
}

const FIELDS: [(&str, EditableStyle, f32, f32, f32); 6] = [
    ("Width", EditableStyle::Width, 1., 4096., 1.),
    ("Height", EditableStyle::Height, 1., 4096., 1.),
    ("Padding", EditableStyle::Padding, 0., 512., 1.),
    ("Margin", EditableStyle::Margin, -512., 512., 1.),
    ("Border", EditableStyle::Border, 0., 64., 1.),
    ("Opacity", EditableStyle::Opacity, 0., 1., 0.1),
];

/// Widget state and subscriptions live as long as the owning inspector.
struct Controls {
    selected: Option<InspectorElementId>,
    edits: Entity<StyleEditState>,
    sliders: Vec<Entity<SliderState>>,
    colors: Vec<Entity<ColorPickerState>>,
    _subscriptions: Vec<Subscription>,
}
impl Controls {
    fn new(edits: Entity<StyleEditState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        let sliders = FIELDS
            .iter()
            .map(|&(_, field, min, max, step)| {
                let slider = cx.new(|_| SliderState::new().min(min).max(max).step(step));
                subscriptions.push(cx.subscribe_in(
                    &slider,
                    window,
                    move |this: &mut Self, _, event, window, cx| {
                        if let (Some(id), SliderEvent::Change(SliderValue::Single(value))) =
                            (&this.selected, event)
                        {
                            apply_edit(id, field, *value, &this.edits, window, cx);
                        }
                    },
                ));
                slider
            })
            .collect();
        let colors = [ColorTarget::Background, ColorTarget::Text]
            .into_iter()
            .map(|target| {
                let color = cx.new(|cx| ColorPickerState::new(window, cx));
                subscriptions.push(cx.subscribe_in(
                    &color,
                    window,
                    move |this: &mut Self, _, event, window, cx| {
                        if let (Some(id), ColorPickerEvent::Change(Some(color))) =
                            (&this.selected, event)
                        {
                            apply_color(id, target, *color, &this.edits, window, cx);
                        }
                    },
                ));
                color
            })
            .collect();
        Self {
            selected: None,
            edits,
            sliders,
            colors,
            _subscriptions: subscriptions,
        }
    }

    fn synchronize(
        &mut self,
        id: InspectorElementId,
        state: &DivInspectorState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected = Some(id);
        let rem = window.rem_size();
        let style = &state.base_style;
        let values = [
            length_pixels(style.size.width, state.bounds.size.width, rem),
            length_pixels(style.size.height, state.bounds.size.height, rem),
            uniform_definite(style, rem),
            uniform_length(style, rem),
            uniform_border(style, rem),
            style.opacity.unwrap_or(1.),
        ];
        for (slider, value) in self.sliders.iter().zip(values) {
            if slider.read(cx).value() != SliderValue::Single(value) {
                slider.update(cx, |slider, cx| slider.set_value(value, window, cx));
            }
        }
        for (color, target) in self
            .colors
            .iter()
            .zip([ColorTarget::Background, ColorTarget::Text])
        {
            let value = target.read(style);
            if color.read(cx).value() != value {
                color.update(cx, |color, cx| match value {
                    Some(value) => color.set_value(value, window, cx),
                    None => color.clear_value(window, cx),
                });
            }
        }
    }
}

fn register_element(cx: &mut App) {
    if !service::register_element_renderer(cx) {
        return;
    }
    cx.register_inspector_element(move |window, cx| {
        let edits = cx.new(|_| StyleEditState::default());
        let controls = cx.new(|cx| Controls::new(edits.clone(), window, cx));
        move |id: InspectorElementId,
              state: &DivInspectorState,
              window: &mut Window,
              cx: &mut App| {
            service::capture(id.clone(), state, &edits, window, cx);
            controls.update(cx, |controls, cx| {
                controls.synchronize(id.clone(), state, window, cx)
            });
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(properties(id, state, &controls, window, cx))
                .child(box_model::box_model(state, window, cx))
                .into_any_element()
        }
    });
}

fn properties(
    id: InspectorElementId,
    state: &DivInspectorState,
    controls: &Entity<Controls>,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let controls = controls.read(cx);
    let changes = controls
        .edits
        .read(cx)
        .original_for(&id)
        .and_then(|(original, size)| {
            style_configuration(&id, &original, &state.base_style, size, window.rem_size())
        })
        .map(|configuration| configuration.changes)
        .unwrap_or_default();
    let edits = controls.edits.clone();
    let rows = FIELDS
        .iter()
        .zip(&controls.sliders)
        .map(|(&(label, _, _, _, _), slider)| {
            let value = slider.read(cx).value().to_string();
            div()
                .flex()
                .items_center()
                .gap_3()
                .h_8()
                .child(
                    div()
                        .w_16()
                        .flex_none()
                        .text_color(cx.theme().muted_foreground)
                        .child(label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .debug_selector(move || format!("inspector-slider-{label}"))
                        .child(Slider::new(slider).horizontal().accessibility_label(label)),
                )
                .child(div().w_12().text_right().text_xs().child(value))
        });
    let colors = [
        ("Surface", ColorTarget::Background),
        ("Text", ColorTarget::Text),
    ]
    .into_iter()
    .zip(&controls.colors)
    .map(|((label, target), color)| {
        div()
            .flex()
            .items_center()
            .gap_3()
            .min_h_8()
            .child(
                div()
                    .w_16()
                    .flex_none()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .debug_selector(move || format!("inspector-color-{label}"))
                    .child(ColorPicker::new(color).small().accessibility_label(label)),
            )
            .child(
                div()
                    .flex_1()
                    .text_right()
                    .text_xs()
                    .child(describe_color(target.read(&state.base_style))),
            )
    });
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Properties"))
                .child(
                    div().debug_selector(|| "inspector-reset".into()).child(
                        Button::new("inspector-reset")
                            .label("Reset")
                            .ghost()
                            .small()
                            .disabled(changes.is_empty())
                            .on_click(move |_, window, cx| reset_edits(&id, &edits, window, cx)),
                    ),
                ),
        )
        .children(rows)
        .children(colors)
        .when(!changes.is_empty(), |body| {
            body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pt_2()
                    .text_xs()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Active overrides"),
                    )
                    .children(changes.into_iter().map(|change| {
                        div().child(format!(
                            "{}: {} → {}",
                            change.property, change.before, change.after
                        ))
                    })),
            )
        })
        .into_any_element()
}

/// Shared panel for in-host inspection and an explicitly routed companion Root.
/// The caller owns annotation synchronization and any header/footer actions.
pub fn panel(
    inspector: &mut Inspector,
    window: &mut Window,
    cx: &mut Context<Inspector>,
    header: Option<AnyElement>,
    footer: impl FnOnce(&Inspector, &mut Window, &mut Context<Inspector>) -> Option<AnyElement>,
) -> AnyElement {
    let picking = inspector.is_picking();
    if picking {
        window.request_animation_frame();
    }
    let target = inspector_target(window, cx);
    let header = header.or_else(|| {
        (target != window.window_handle()).then(|| {
            Button::new("inspector-stop")
                .label("Stop")
                .small()
                .on_click(move |_, _, cx| companion::stop(target.window_id(), cx))
                .into_any_element()
        })
    });
    let source = inspector
        .active_element_id()
        .map(|id| source::breadcrumb(id.path.source_location, window, cx));
    let states = inspector.render_inspector_states(window, cx);
    let footer = footer(inspector, window, cx);
    let focus = window
        .use_keyed_state("agentation-inspector-focus", cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let owner = cx.entity();
    let hint = if picking {
        "Click an element to edit it. Scroll to select a parent. Esc exits picking."
    } else {
        "Selected · Changes preview live. Pick selects another element."
    };
    div()
        .id("agentation-inspector")
        .size_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(cx.theme().background)
        .text_color(cx.theme().foreground)
        .text_sm()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_none()
                .p_3()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("Inspector"))
                        .children(header.or_else(|| {
                            (!picking).then(|| {
                                Button::new("inspector-pick")
                                    .label("Pick")
                                    .ghost()
                                    .small()
                                    .on_click(move |_, window, cx| {
                                        owner.update(cx, |inspector, cx| {
                                            inspector.start_picking();
                                            cx.notify();
                                        });
                                        let target = inspector_target(window, cx);
                                        if target != window.window_handle() {
                                            let _ = target.update(cx, |_, host, _| host.refresh());
                                        }
                                        window.refresh();
                                    })
                                    .into_any_element()
                            })
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(hint),
                ),
        )
        .child(
            div()
                .id("inspector-details")
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .p_3()
                .gap_3()
                .overflow_y_scroll()
                .children(source)
                .children(states),
        )
        .children(footer)
        .focus_trap("inspector-focus", &focus)
        .into_any_element()
}

#[cfg(test)]
mod tests;
