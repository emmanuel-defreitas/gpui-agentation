//! Textboxes on the canvas, in the manner of Freeform or Sketch: draw a frame to
//! place one, click to select it, drag its body to move it, and pull the grips
//! on its dashed bounding box to resize it.
use super::surface::{FeedbackDrag, textbox_input, viewport};
use super::*;
use crate::ely::{primitives::Measure, theme::Radius};
use crate::{Annotation, AnnotationShape, Rect};
use gpui::{AnyElement, Bounds, CursorStyle, EmptyView, MouseButton, Pixels, Point, point, px};

/// A handle on a selected textbox's bounding box. Side grips change the wrap
/// width; corner grips also set the minimum height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Grip {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Left,
    Right,
}

impl Grip {
    pub(super) const ALL: [Self; 6] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
        Self::Left,
        Self::Right,
    ];

    fn moves_left(self) -> bool {
        matches!(self, Self::Left | Self::TopLeft | Self::BottomLeft)
    }

    fn moves_right(self) -> bool {
        matches!(self, Self::Right | Self::TopRight | Self::BottomRight)
    }

    fn moves_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight)
    }

    fn moves_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }

    fn name(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    fn cursor(self) -> CursorStyle {
        match self {
            Self::TopLeft | Self::BottomRight => CursorStyle::ResizeUpLeftDownRight,
            Self::TopRight | Self::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
            Self::Left | Self::Right => CursorStyle::ResizeLeftRight,
        }
    }

    /// Where the grip sits on `bounds`.
    fn center(self, bounds: Bounds<Pixels>) -> Point<Pixels> {
        let x = if self.moves_left() {
            bounds.left()
        } else if self.moves_right() {
            bounds.right()
        } else {
            bounds.center().x
        };
        let y = if self.moves_top() {
            bounds.top()
        } else if self.moves_bottom() {
            bounds.bottom()
        } else {
            bounds.center().y
        };
        point(x, y)
    }
}

/// Smallest textbox width, as a share of the content area.
pub(super) const MIN_WIDTH: f32 = 0.04;

impl AnnotationOverlay {
    pub(super) fn select_textbox(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        self.selected_note = Some(id.into());
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(super) fn begin_textbox_drag(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        if let Ok(note) = self.store.annotation(id) {
            if let Some(AnnotationShape::Text { at, .. }) = note.feedback.shape.as_ref() {
                let mouse = self
                    .annotation_press
                    .take()
                    .unwrap_or(window.mouse_position());
                let (w, h) = viewport(window, cx);
                let offset = (
                    f32::from(mouse.x) / w.max(1.0) - at.0,
                    f32::from(mouse.y) / h.max(1.0) - at.1,
                );
                self.selected_note = Some(id.into());
                self.textbox_drag = Some((note, offset));
            }
        }
        cx.notify();
    }

    /// Begin resizing from one of the selected textbox's grips.
    pub(super) fn begin_textbox_resize(
        &mut self,
        id: &str,
        grip: Grip,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        if let Ok(note) = self.store.annotation(id) {
            if matches!(note.feedback.shape, Some(AnnotationShape::Text { .. })) {
                self.selected_note = Some(id.into());
                self.textbox_resize = Some((note, grip));
            }
        }
        cx.notify();
    }

    /// The textbox as it stands with `grip` pulled to `pointer` (normalized).
    pub(super) fn resized_textbox(
        &self,
        before: &Annotation,
        grip: Grip,
        pointer: (f32, f32),
        window: &Window,
        cx: &App,
    ) -> Option<Annotation> {
        let AnnotationShape::Text {
            at, width, height, ..
        } = before.feedback.shape.as_ref()?
        else {
            return None;
        };
        let (w, h) = viewport(window, cx);
        let measured = self.textbox_bounds.get(&before.id);
        let current_width = width.unwrap_or_else(|| {
            measured.map_or(MIN_WIDTH, |bounds| {
                f32::from(bounds.size.width) / w.max(1.0)
            })
        });
        let current_height = height.unwrap_or_else(|| {
            measured.map_or(0.0, |bounds| f32::from(bounds.size.height) / h.max(1.0))
        });
        let min_height = f32::from(
            cx.theme()
                .control_height(ControlSize::Lg)
                .to_pixels(window.rem_size()),
        ) / h.max(1.0);
        let (mut left, mut top) = *at;
        let (mut right, mut bottom) = (left + current_width, top + current_height);
        if grip.moves_left() {
            left = pointer.0.clamp(0.0, right - MIN_WIDTH);
        } else if grip.moves_right() {
            right = pointer.0.clamp(left + MIN_WIDTH, 1.0);
        }
        if grip.moves_top() {
            top = pointer.1.clamp(0.0, bottom - min_height);
        } else if grip.moves_bottom() {
            bottom = pointer.1.clamp(top + min_height, 1.0);
        }
        let mut after = moved_textbox(before, (left, top), (w, h));
        if let Some(AnnotationShape::Text { width, height, .. }) = &mut after.feedback.shape {
            *width = Some(right - left);
            if grip.moves_top() || grip.moves_bottom() {
                *height = Some(bottom - top);
            }
        }
        Some(after)
    }

    /// The inline editor for a brand-new, still-unsaved textbox draft. Keeps the
    /// comment editor from floating: text is typed directly on the canvas.
    pub(super) fn draft_textbox(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let draft = self.draft.as_ref()?;
        let AnnotationShape::Text {
            at, width, height, ..
        } = draft.shape.as_ref()?
        else {
            return None;
        };
        if draft.id.is_some() {
            return None; // Saved textboxes render through `textbox` instead.
        }
        let color = cx.theme().colors.chart[draft.color];
        Some(
            self.textbox_frame(at, *width, *height, color, true, None, window, cx)
                .into_any_element(),
        )
    }

    pub(super) fn textbox(
        &self,
        note: &Annotation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(AnnotationShape::Text {
            at,
            text,
            width,
            height,
            ..
        }) = &note.feedback.shape
        else {
            return div().into_any_element();
        };
        let editing = self
            .draft
            .as_ref()
            .is_some_and(|d| d.editing && d.id.as_deref() == Some(note.id.as_str()));
        let selected = editing || self.selected_note.as_deref() == Some(note.id.as_str());
        let color = cx.theme().colors.chart[note.annotation_color()];
        let (w, h) = viewport(window, cx);
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let frame = self.textbox_frame(
            at,
            *width,
            *height,
            color,
            editing,
            Some((note.id.clone(), text.clone(), selected)),
            window,
            cx,
        );
        div()
            .absolute()
            .size_full()
            .child(frame)
            .child(self.marker(
                &note.id,
                note.index,
                Rect {
                    x: at.0 * w + f32::from(side) / 2.0,
                    y: at.1 * h + f32::from(side) / 2.0,
                    width: 0.0,
                    height: 0.0,
                },
                window,
                cx,
            ))
            .when(selected, |layer| {
                let marker = div()
                    .debug_selector(|| "feedback-textbox-selected".into())
                    .absolute()
                    .left(px(at.0 * w))
                    .top(px(at.1 * h))
                    .size_0();
                let layer = layer.child(marker);
                if let Some(bounds) = self.textbox_bounds.get(&note.id) {
                    layer.children(
                        Grip::ALL
                            .into_iter()
                            .map(|grip| self.resize_grip(&note.id, grip, *bounds, color, cx)),
                    )
                } else {
                    layer
                }
            })
            .into_any_element()
    }

    /// Shared body of a textbox: a movable box that shows its text or hosts the
    /// inline editor. Its dashed bounding border appears while selected or
    /// editing, as in Freeform; at rest only the text shows. `saved` carries an
    /// existing note's id, text, and selection; `None` is the unsaved draft.
    #[allow(clippy::too_many_arguments)]
    fn textbox_frame(
        &self,
        at: &(f32, f32),
        width: Option<f32>,
        height: Option<f32>,
        color: gpui::Hsla,
        editing: bool,
        saved: Option<(String, String, bool)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let (w, h) = viewport(window, cx);
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let left = at.0 * w;
        let max_w = (w - left).max(f32::from(side));
        let fixed_w = width.map(|fraction| (fraction * w).clamp(f32::from(side), max_w));
        let min_h = height.map_or(side, |fraction| px(fraction * h).max(side));
        let selected = editing || saved.as_ref().is_some_and(|(_, _, selected)| *selected);
        let owner = cx.entity().downgrade();
        let drag_owner = owner.clone();
        let entity = cx.entity_id();
        let id = saved.as_ref().map(|(id, ..)| id.clone());
        let text = saved.as_ref().map(|(_, text, _)| text.clone());

        let mut body = div()
            .id("feedback-textbox-body")
            .debug_selector(|| "feedback-textbox".into())
            .min_h(min_h)
            .pl(side + gpui::rems(0.5).to_pixels(window.rem_size()))
            .pr_1()
            .py_1()
            .border_1()
            .border_dashed()
            .border_color(if selected {
                color
            } else {
                gpui::transparent_black()
            })
            .rounded(cx.theme().radius(Radius::Sm))
            .text_size(px(16.0))
            .line_height(px(22.0))
            .text_color(color)
            .cursor(if selected && !editing {
                CursorStyle::OpenHand
            } else if editing {
                CursorStyle::IBeam
            } else {
                CursorStyle::PointingHand
            })
            .occlude()
            .flex()
            .flex_col();
        body = match fixed_w {
            Some(fixed) => body.w(px(fixed)),
            None => body.max_w(px(max_w)),
        };
        if editing {
            body = body.child(textbox_input(&self.input, color, window, cx));
        } else if let Some(text) = &text {
            body = body.children(text.split('\n').map(|line| div().child(line.to_owned())));
        }
        if let Some(id) = &id {
            let click_id = id.clone();
            let drag_id = id.clone();
            let menu_id = id.clone();
            body = body
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, window, _| {
                        this.annotation_press = Some(event.position);
                        window.prevent_default();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                        this.show_annotation_menu(&menu_id, event.position, window, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_drag(FeedbackDrag { owner: entity }, move |_, _, window, cx| {
                    let _ = drag_owner.update(cx, |this, cx| {
                        this.begin_textbox_drag(&drag_id, window, cx);
                        this.advance_drag(window.mouse_position(), window, cx);
                    });
                    cx.new(|_| EmptyView)
                })
                .on_click(
                    cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                        if event.click_count() >= 2 {
                            this.edit(&click_id, window, cx);
                        } else {
                            this.select_textbox(&click_id, window, cx);
                        }
                    }),
                );
        }

        let measure_id = id.clone();
        // Each box measures under its own key: the draft and the note it becomes
        // can share bounds, and a shared key would then report nothing.
        let measure_key = id.as_deref().map_or_else(
            || "textbox-bounds-draft".to_string(),
            |id| format!("textbox-bounds-{id}"),
        );
        div().absolute().size_full().child(
            div()
                .absolute()
                .left(px(left))
                .top(px(at.1 * h))
                .max_w(px(max_w))
                .child(
                    Measure::new(measure_key, move |bounds, _, cx| {
                        if let Some(measure_id) = &measure_id {
                            let measure_id = measure_id.clone();
                            let _ = owner.update(cx, |this, cx| {
                                if this.textbox_bounds.get(&measure_id) != Some(&bounds) {
                                    this.textbox_bounds.insert(measure_id.clone(), bounds);
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .child(body),
                ),
        )
    }

    /// One grip on the selected textbox's bounding box.
    fn resize_grip(
        &self,
        id: &str,
        grip: Grip,
        bounds: Bounds<Pixels>,
        color: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        const SIZE: f32 = 10.0;
        let drag_owner = cx.entity().downgrade();
        let entity = cx.entity_id();
        let resize_id = id.to_owned();
        let at = grip.center(bounds);
        let name = grip.name();
        div()
            .absolute()
            .left(at.x - px(SIZE / 2.0))
            .top(at.y - px(SIZE / 2.0))
            .size(px(SIZE))
            .rounded_sm()
            .bg(gpui::white())
            .border_1()
            .border_color(color)
            .cursor(grip.cursor())
            .occlude()
            .id(format!("feedback-textbox-grip-{name}"))
            .debug_selector(move || format!("feedback-textbox-resize-{name}"))
            .on_drag(FeedbackDrag { owner: entity }, move |_, _, window, cx| {
                let _ = drag_owner.update(cx, |this, cx| {
                    this.begin_textbox_resize(&resize_id, grip, window, cx);
                    this.advance_drag(window.mouse_position(), window, cx);
                });
                cx.new(|_| EmptyView)
            })
    }
}

pub(super) fn moved_textbox(
    before: &Annotation,
    at: (f32, f32),
    viewport: (f32, f32),
) -> Annotation {
    let mut after = before.clone();
    if let Some(AnnotationShape::Text { at: origin, .. }) = &mut after.feedback.shape {
        let delta = (
            (at.0 - origin.0) * viewport.0,
            (at.1 - origin.1) * viewport.1,
        );
        *origin = at;
        after.feedback.x += delta.0 / viewport.0.max(1.0) * 100.0;
        after.feedback.y += delta.1;
        after.feedback.bounding_box.x += delta.0;
        after.feedback.bounding_box.y += delta.1;
    }
    after
}
