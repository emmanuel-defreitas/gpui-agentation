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

/// Smallest framed textbox width in pixels.
pub(super) const MIN_WIDTH: f32 = 128.0;

/// Clicks this close to a selected box, in pixels, still belong to its grips.
const GRIP_REACH: f32 = 8.0;

/// A textbox's normalized geometry: its top-left corner and, when framed, its
/// wrap width and minimum height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TextFrame {
    pub at: (f32, f32),
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl TextFrame {
    fn of(shape: &AnnotationShape) -> Option<Self> {
        match shape {
            AnnotationShape::Text {
                at, width, height, ..
            } => Some(Self {
                at: *at,
                width: *width,
                height: *height,
            }),
            _ => None,
        }
    }

    fn write(self, shape: &mut AnnotationShape) {
        if let AnnotationShape::Text {
            at, width, height, ..
        } = shape
        {
            *at = self.at;
            *width = self.width;
            *height = self.height;
        }
    }
}

/// A grip being pulled: on a saved note (kept for undo) or on the unsaved draft.
pub(super) struct TextboxResize {
    pub note: Option<Annotation>,
    pub origin: TextFrame,
    pub grip: Grip,
}

pub(super) const DEFAULT_WIDTH: f32 = 280.0;

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

    pub(super) fn textbox_drag_position(
        &self,
        id: &str,
        at: (f32, f32),
        window: &Window,
        cx: &App,
    ) -> (f32, f32) {
        let size = self
            .textbox_bounds
            .get(id)
            .map_or((DEFAULT_WIDTH, 40.0), |bounds| {
                (f32::from(bounds.size.width), f32::from(bounds.size.height))
            });
        constrained_origin(at, size, viewport(window, cx))
    }

    /// Begin resizing from one of a textbox's grips: a saved note by `id`, or
    /// the unsaved draft with `None`. Editing continues through the resize.
    pub(super) fn begin_textbox_resize(
        &mut self,
        id: Option<&str>,
        grip: Grip,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = id else {
            if let Some(origin) = self
                .draft
                .as_ref()
                .filter(|draft| draft.id.is_none())
                .and_then(|draft| draft.shape.as_ref())
                .and_then(TextFrame::of)
            {
                self.textbox_resize = Some(TextboxResize {
                    note: None,
                    origin,
                    grip,
                });
                cx.notify();
            }
            return;
        };
        let editing_this = self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.id.as_deref() == Some(id));
        if !editing_this && !self.dismiss_note(window, cx) {
            return;
        }
        if let Ok(note) = self.store.annotation(id) {
            if let Some(origin) = note.feedback.shape.as_ref().and_then(TextFrame::of) {
                self.selected_note = Some(id.into());
                self.textbox_resize = Some(TextboxResize {
                    note: Some(note),
                    origin,
                    grip,
                });
            }
        }
        cx.notify();
    }

    /// `origin` with `grip` pulled to `pointer` (normalized). `measured` is the
    /// box's current size on screen, for sides the frame leaves to its text.
    pub(super) fn resized_frame(
        origin: TextFrame,
        measured: Option<&Bounds<Pixels>>,
        grip: Grip,
        pointer: (f32, f32),
        window: &Window,
        cx: &App,
    ) -> TextFrame {
        let (w, h) = viewport(window, cx);
        let min_width = (MIN_WIDTH / w.max(1.0)).min(1.0);
        let current_width = origin.width.unwrap_or_else(|| {
            measured.map_or(min_width, |bounds| {
                f32::from(bounds.size.width) / w.max(1.0)
            })
        });
        let current_height = origin.height.unwrap_or_else(|| {
            measured.map_or(0.0, |bounds| f32::from(bounds.size.height) / h.max(1.0))
        });
        let min_height = f32::from(
            cx.theme()
                .control_height(ControlSize::Lg)
                .to_pixels(window.rem_size()),
        ) / h.max(1.0);
        let (mut left, mut top) = origin.at;
        let (mut right, mut bottom) = (left + current_width, top + current_height);
        if grip.moves_left() {
            left = pointer.0.clamp(0.0, (right - min_width).max(0.0));
        } else if grip.moves_right() {
            right = pointer.0.clamp((left + min_width).min(1.0), 1.0);
        }
        if grip.moves_top() {
            top = pointer.1.clamp(0.0, (bottom - min_height).max(0.0));
        } else if grip.moves_bottom() {
            bottom = pointer.1.clamp((top + min_height).min(1.0), 1.0);
        }
        TextFrame {
            at: (left, top),
            width: Some(right - left),
            height: if grip.moves_top() || grip.moves_bottom() {
                Some(bottom - top)
            } else {
                origin.height
            },
        }
    }

    /// Apply the grip's current pointer to whatever is being resized.
    pub(super) fn advance_textbox_resize(
        &mut self,
        pointer: (f32, f32),
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(resize) = &self.textbox_resize else {
            return;
        };
        let (grip, origin) = (resize.grip, resize.origin);
        match &resize.note {
            Some(before) => {
                let measured = self.textbox_bounds.get(&before.id);
                let frame = Self::resized_frame(origin, measured, grip, pointer, window, cx);
                let mut after = moved_textbox(before, frame.at, viewport(window, cx));
                if let Some(shape) = &mut after.feedback.shape {
                    frame.write(shape);
                }
                if let Some(note) = self
                    .session
                    .annotations
                    .iter_mut()
                    .find(|n| n.id == before.id)
                {
                    *note = after;
                }
            }
            None => {
                let measured = self.draft_textbox_bounds.as_ref();
                let frame = Self::resized_frame(origin, measured, grip, pointer, window, cx);
                let (w, h) = viewport(window, cx);
                if let Some(draft) = self.draft.as_mut().filter(|draft| draft.id.is_none()) {
                    if let Some(shape) = &mut draft.shape {
                        frame.write(shape);
                        let bounds = shape.bounds(w, h);
                        draft.target.bounds = bounds;
                        draft.x = (bounds.x + bounds.width / 2.0) / w.max(1.0) * 100.0;
                        draft.y = bounds.y + bounds.height / 2.0;
                    }
                }
            }
        }
        cx.notify();
    }

    /// Finish a grip drag: a saved note records the change; the draft keeps it.
    pub(super) fn finish_textbox_resize(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(resize) = self.textbox_resize.take() else {
            return false;
        };
        if let Some(before) = resize.note {
            let after = self
                .session
                .annotations
                .iter()
                .find(|n| n.id == before.id)
                .cloned();
            if let Some(after) = after {
                if before != after {
                    match self.store.replace_note(Some(&before), Some(&after)) {
                        Ok(()) => {
                            // Editing continues on the resized frame, not the old one.
                            if let Some(draft) = self
                                .draft
                                .as_mut()
                                .filter(|draft| draft.id.as_deref() == Some(after.id.as_str()))
                            {
                                draft.shape = after.feedback.shape.clone();
                            }
                            self.record_change(Some(before), Some(after));
                        }
                        Err(error) => self.notice = error.to_string(),
                    }
                }
            }
            if let Ok(session) = self.store.session(&self.session.id) {
                self.session = session;
            }
        }
        cx.notify();
        true
    }

    /// Whether a press at `at` belongs to the box or its grips, so it must not
    /// end editing.
    fn textbox_owns_press(bounds: Option<&Bounds<Pixels>>, at: Point<Pixels>) -> bool {
        bounds.is_some_and(|bounds| {
            bounds
                .extend(gpui::Edges::all(px(GRIP_REACH)))
                .contains(&at)
        })
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
        let frame = self.textbox_frame(at, *width, *height, color, true, None, window, cx);
        let mut grips = Vec::new();
        if let Some(bounds) = self.draft_textbox_bounds {
            for grip in Grip::ALL {
                grips.push(self.resize_grip(None, grip, bounds, color, cx));
            }
        }
        Some(
            div()
                .absolute()
                .size_full()
                .child(frame)
                .children(grips)
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
        // The pin sits above the box's left edge, like a tag, so the text
        // starts at the box's own padding.
        div()
            .absolute()
            .size_full()
            .child(frame)
            .child(self.marker(
                &note.id,
                note.index,
                Rect {
                    x: at.0 * w + f32::from(side) / 2.0,
                    y: at.1 * h - f32::from(side) / 2.0 - 4.0,
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
                            .map(|grip| self.resize_grip(Some(&note.id), grip, *bounds, color, cx)),
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
        let max_w = (w - left).max(1.0);
        let fixed_w = width.map(|fraction| (fraction * w).clamp(MIN_WIDTH.min(max_w), max_w));
        let min_h = height.map_or(side, |fraction| px(fraction * h).max(side));
        let selected = editing || saved.as_ref().is_some_and(|(_, _, selected)| *selected);
        let owner = cx.entity().downgrade();
        let drag_owner = owner.clone();
        let entity = cx.entity_id();
        let id = saved.as_ref().map(|(id, ..)| id.clone());
        let text = saved.as_ref().map(|(_, text, _)| text.clone());

        let mut body = div()
            .id(format!(
                "feedback-textbox-body-{}",
                id.as_deref().unwrap_or("draft")
            ))
            .debug_selector(|| "feedback-textbox".into())
            .min_h(min_h)
            .px_2()
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
            None => body.min_w(px(MIN_WIDTH.min(max_w))).max_w(px(max_w)),
        };
        if editing {
            body = body.child(textbox_input(&self.input, color, window, cx));
        } else if let Some(text) = &text {
            body = body.children(text.split('\n').map(|line| {
                div()
                    .min_h(px(22.0))
                    .whitespace_normal()
                    .child(line.to_owned())
            }));
        }
        if let Some(id) = id.as_ref().filter(|_| !editing) {
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
        let press_id = id.clone();
        div().absolute().size_full().child(
            div()
                .id(format!(
                    "feedback-textbox-frame-{}",
                    id.as_deref().unwrap_or("draft")
                ))
                .absolute()
                .when(editing, |frame| {
                    frame.on_mouse_down_out(cx.listener(
                        move |this, event: &gpui::MouseDownEvent, window, cx| {
                            let own = match &press_id {
                                Some(id) => this.textbox_bounds.get(id),
                                None => this.draft_textbox_bounds.as_ref(),
                            };
                            if Self::textbox_owns_press(own, event.position)
                                || this
                                    .mentions
                                    .bounds
                                    .is_some_and(|bounds| bounds.contains(&event.position))
                            {
                                return;
                            }
                            if event.button == MouseButton::Left {
                                let dismissed = this.dismiss_note(window, cx);
                                window.prevent_default();
                                if !dismissed {
                                    cx.stop_propagation();
                                }
                            }
                        },
                    ))
                })
                .left(px(left))
                .top(px(at.1 * h))
                .max_w(px(max_w))
                .child(
                    Measure::new(measure_key, move |bounds, _, cx| {
                        let _ = owner.update(cx, |this, cx| match &measure_id {
                            Some(measure_id) => {
                                if this.textbox_bounds.get(measure_id) != Some(&bounds) {
                                    this.textbox_bounds.insert(measure_id.clone(), bounds);
                                    cx.notify();
                                }
                            }
                            None => {
                                if this.draft_textbox_bounds != Some(bounds) {
                                    this.draft_textbox_bounds = Some(bounds);
                                    cx.notify();
                                }
                            }
                        });
                    })
                    .child(body),
                )
                .when(editing, |frame| {
                    frame.child(
                        div()
                            .mt_1()
                            .p_1()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_1()
                            .rounded(cx.theme().radius(Radius::Sm))
                            .bg(cx.theme().colors.surface)
                            .occlude()
                            .child(
                                div()
                                    .debug_selector(|| "feedback-textbox-cancel".into())
                                    .child(
                                        Button::new("textbox-cancel", "Cancel")
                                            .size(ControlSize::Sm)
                                            .variant(ButtonVariant::Ghost)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.draft = None;
                                                this.draft_textbox_bounds = None;
                                                this.notice.clear();
                                                this.return_focus(window, cx);
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "feedback-textbox-save".into())
                                    .child(
                                        Button::new("textbox-save", "Save")
                                            .size(ControlSize::Sm)
                                            .variant(ButtonVariant::Primary)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save(window, cx);
                                            })),
                                    ),
                            )
                            .when(!self.notice.is_empty(), |row| {
                                row.child(self.notice.clone())
                            }),
                    )
                }),
        )
    }

    /// One grip on a textbox's bounding box; `None` is the unsaved draft.
    fn resize_grip(
        &self,
        id: Option<&str>,
        grip: Grip,
        bounds: Bounds<Pixels>,
        color: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        const SIZE: f32 = 10.0;
        let drag_owner = cx.entity().downgrade();
        let entity = cx.entity_id();
        let resize_id = id.map(str::to_owned);
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
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    this.annotation_press = Some(event.position);
                    window.prevent_default();
                    cx.stop_propagation();
                }),
            )
            .on_drag(FeedbackDrag { owner: entity }, move |_, _, window, cx| {
                let _ = drag_owner.update(cx, |this, cx| {
                    this.begin_textbox_resize(resize_id.as_deref(), grip, window, cx);
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

/// Keep the whole textbox visible, including its pin and resize handle.
pub(super) fn constrained_origin(
    at: (f32, f32),
    size: (f32, f32),
    viewport: (f32, f32),
) -> (f32, f32) {
    (
        at.0.clamp(0.0, (1.0 - size.0 / viewport.0.max(1.0)).max(0.0)),
        at.1.clamp(0.0, (1.0 - size.1 / viewport.1.max(1.0)).max(0.0)),
    )
}
