use super::*;
use crate::ely::{primitives::Measure, theme::Radius};
use crate::{Annotation, AnnotationShape, Rect};
use gpui::{AnyElement, MouseButton, px};

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

    pub(super) fn textbox(
        &self,
        note: &Annotation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(AnnotationShape::Text { at, text, .. }) = &note.feedback.shape else {
            return div().into_any_element();
        };
        let (w, h) = (
            f32::from(window.viewport_size().width),
            f32::from(window.viewport_size().height),
        );
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let selected = self.selected_note.as_deref() == Some(note.id.as_str());
        let color = cx.theme().colors.chart[note.annotation_color()];
        let id = note.id.clone();
        let click_id = id.clone();
        let drag_id = id.clone();
        let menu_id = id.clone();
        let measure_id = id.clone();
        let owner = cx.entity().downgrade();
        let drag_owner = owner.clone();
        let entity = cx.entity_id();
        let body = div()
            .id(format!("feedback-text-{id}"))
            .debug_selector(|| "feedback-textbox".into())
            .max_w(px((w - at.0 * w).max(1.0)))
            .min_h(side)
            .pl(side + gpui::rems(0.5).to_pixels(window.rem_size()))
            .pr_1()
            .py_1()
            .border_1()
            .border_color(if selected {
                color
            } else {
                gpui::transparent_black()
            })
            .rounded(cx.theme().radius(Radius::Sm))
            .text_size(px(16.0))
            .line_height(px(22.0))
            .text_color(color)
            .cursor_move()
            .occlude()
            .flex()
            .flex_col()
            .children(text.split('\n').map(|line| div().child(line.to_owned())))
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
            .on_drag(
                super::surface::FeedbackDrag { owner: entity },
                move |_, _, window, cx| {
                    let _ = drag_owner.update(cx, |this, cx| {
                        this.begin_textbox_drag(&drag_id, window, cx);
                        this.advance_drag(window.mouse_position(), window, cx);
                    });
                    cx.new(|_| gpui::EmptyView)
                },
            )
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    if event.click_count() >= 2 {
                        this.edit(&click_id, window, cx);
                    } else {
                        this.select_textbox(&click_id, window, cx);
                    }
                }),
            );
        div()
            .absolute()
            .size_full()
            .child(
                div()
                    .absolute()
                    .left(px(at.0 * w))
                    .top(px(at.1 * h))
                    .max_w(px((w - at.0 * w).max(1.0)))
                    .child(
                        Measure::new(format!("textbox-bounds-{id}"), move |bounds, _, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                if this.textbox_bounds.get(&measure_id) != Some(&bounds) {
                                    this.textbox_bounds.insert(measure_id.clone(), bounds);
                                    cx.notify();
                                }
                            });
                        })
                        .child(body),
                    ),
            )
            .child(self.marker(
                &id,
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
                layer.child(
                    div()
                        .debug_selector(|| "feedback-textbox-selected".into())
                        .absolute()
                        .left(px(at.0 * w))
                        .top(px(at.1 * h))
                        .size_0(),
                )
            })
            .into_any_element()
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
