use super::surface::{FeedbackDrag, textbox_input, viewport};
use super::*;
use crate::ely::{primitives::Measure, theme::Radius};
use crate::{Annotation, AnnotationShape, Rect};
use gpui::{AnyElement, CursorStyle, EmptyView, MouseButton, px};

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
                let offset = (
                    f32::from(mouse.x) / f32::from(window.viewport_size().width).max(1.0) - at.0,
                    f32::from(mouse.y) / f32::from(window.viewport_size().height).max(1.0) - at.1,
                );
                self.selected_note = Some(id.into());
                self.textbox_drag = Some((note, offset));
            }
        }
        cx.notify();
    }

    /// Begin a width resize from the textbox's bottom-right handle.
    pub(super) fn begin_textbox_resize(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        if let Ok(note) = self.store.annotation(id) {
            if matches!(note.feedback.shape, Some(AnnotationShape::Text { .. })) {
                self.selected_note = Some(id.into());
                self.textbox_resize = Some(note);
            }
        }
        cx.notify();
    }

    /// The inline editor for a brand-new, still-unsaved textbox draft. Keeps the
    /// comment editor from floating: text is typed directly on the canvas.
    pub(super) fn draft_textbox(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let draft = self.draft.as_ref()?;
        let AnnotationShape::Text { at, width, .. } = draft.shape.as_ref()? else {
            return None;
        };
        if draft.id.is_some() {
            return None; // Saved textboxes render through `textbox` instead.
        }
        let color = cx.theme().colors.chart[draft.color];
        Some(
            self.textbox_frame(at, *width, color, true, None, window, cx)
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
            at, text, width, ..
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
        let (w, h) = viewport(window);
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let frame = self.textbox_frame(
            at,
            *width,
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
                    layer.child(self.resize_handle(&note.id, *bounds, color, cx))
                } else {
                    layer
                }
            })
            .into_any_element()
    }

    /// Shared body of a textbox: a dashed, movable box that either shows its
    /// text or hosts the inline editor. `saved` carries an existing note's id,
    /// text, and selection; `None` is the unsaved draft.
    #[allow(clippy::too_many_arguments)]
    fn textbox_frame(
        &self,
        at: &(f32, f32),
        width: Option<f32>,
        color: gpui::Hsla,
        editing: bool,
        saved: Option<(String, String, bool)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let (w, h) = viewport(window);
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let left = at.0 * w;
        let max_w = (w - left).max(f32::from(side));
        let fixed_w = width.map(|fraction| (fraction * w).clamp(f32::from(side), max_w));
        let selected = editing || saved.as_ref().is_some_and(|(_, _, selected)| *selected);
        let owner = cx.entity().downgrade();
        let drag_owner = owner.clone();
        let entity = cx.entity_id();
        let id = saved.as_ref().map(|(id, ..)| id.clone());
        let text = saved.as_ref().map(|(_, text, _)| text.clone());

        let mut body = div()
            .id("feedback-textbox-body")
            .debug_selector(|| "feedback-textbox".into())
            .min_h(side)
            .pl(side + gpui::rems(0.5).to_pixels(window.rem_size()))
            .pr_1()
            .py_1()
            .border_1()
            .border_dashed()
            .border_color(if selected { color } else { color.opacity(0.6) })
            .rounded(cx.theme().radius(Radius::Sm))
            .text_size(px(16.0))
            .line_height(px(22.0))
            .text_color(color)
            .cursor_move()
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
        div().absolute().size_full().child(
            div()
                .absolute()
                .left(px(left))
                .top(px(at.1 * h))
                .max_w(px(max_w))
                .child(
                    Measure::new("textbox-bounds", move |bounds, _, cx| {
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

    fn resize_handle(
        &self,
        id: &str,
        bounds: gpui::Bounds<gpui::Pixels>,
        color: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let drag_owner = cx.entity().downgrade();
        let entity = cx.entity_id();
        let resize_id = id.to_owned();
        let x = f32::from(bounds.origin.x + bounds.size.width) - 6.0;
        let y = f32::from(bounds.origin.y + bounds.size.height) - 6.0;
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .size(px(12.0))
            .rounded_sm()
            .bg(color)
            .border_1()
            .border_color(gpui::white())
            .cursor(CursorStyle::ResizeUpLeftDownRight)
            .occlude()
            .id("feedback-textbox-resize")
            .debug_selector(|| "feedback-textbox-resize".into())
            .on_drag(FeedbackDrag { owner: entity }, move |_, _, window, cx| {
                let _ = drag_owner.update(cx, |this, cx| {
                    this.begin_textbox_resize(&resize_id, window, cx);
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
