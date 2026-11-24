use super::*;
use crate::drawing::AnnotationTool;
use crate::{
    AnnotationShape, Rect,
    drawing::{centered_panel, note_panel_position},
};
use gpui::{
    ClipboardItem, CursorStyle, DragMoveEvent, EmptyView, EntityId, MouseButton, Role, anchored,
    canvas, point, px,
};

#[derive(Clone)]
pub(super) struct FeedbackDrag {
    pub owner: EntityId,
}

pub(super) fn tool_shortcut(tool: Option<AnnotationTool>) -> &'static str {
    match tool {
        None => "I",
        Some(AnnotationTool::Box) => "B",
        Some(AnnotationTool::Arrow) => "A",
        Some(AnnotationTool::Pen) => "P",
        Some(AnnotationTool::Pin) => "C",
        Some(AnnotationTool::Text) => "T",
    }
}

pub(super) fn tool_cursor(tool: Option<AnnotationTool>) -> CursorStyle {
    match tool {
        None => CursorStyle::Arrow,
        Some(_) => CursorStyle::Crosshair,
    }
}

pub(super) fn annotation_icon(shape: Option<&AnnotationShape>) -> IconName {
    match shape {
        None => IconName::Crosshair,
        Some(AnnotationShape::Box { .. }) => IconName::Square,
        Some(AnnotationShape::Arrow { .. }) => IconName::ArrowUpRight,
        Some(AnnotationShape::Pen { .. }) => IconName::PenLine,
        Some(AnnotationShape::Pin { .. }) => IconName::MessageSquare,
        Some(AnnotationShape::Text { .. }) => IconName::Type,
    }
}

pub(super) fn comment_input(
    input: &gpui::Entity<TextareaState>,
    background: gpui::Hsla,
    textbox: bool,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let colors = &cx.theme().colors;
    let focused = input.focus_handle(cx).is_focused(window);
    let field = Textarea::new(input)
        .appearance(false)
        .bordered(false)
        .aria_label("Feedback")
        .bg(background)
        .text_color(colors.background)
        .p_0()
        .rounded_none();
    div()
        .relative()
        .w_full()
        .child(field)
        .when(input.read(cx).value().is_empty() && !focused, |root| {
            root.child(
                div()
                    .debug_selector(|| "feedback-editor-placeholder".into())
                    .absolute()
                    .top_0()
                    .left_0()
                    .text_size(cx.theme().typography_tokens().sm.size)
                    .text_color(colors.background.opacity(0.65))
                    .child(if textbox {
                        "Type text…"
                    } else {
                        "Describe what should change…"
                    }),
            )
        })
        .into_any_element()
}

/// The text field shown inside a textbox on the canvas: transparent, drawn in
/// the annotation color, with its own placeholder. Used for inline editing.
pub(super) fn textbox_input(
    input: &gpui::Entity<TextareaState>,
    color: gpui::Hsla,
    _window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let field = Textarea::new(input)
        .appearance(false)
        .bordered(false)
        .aria_label("Textbox")
        .bg(gpui::transparent_black())
        .text_color(color)
        .p_0()
        .rounded_none()
        .text_size(px(16.))
        .line_height(px(22.));
    div()
        .debug_selector(|| "feedback-textbox-input".into())
        .relative()
        .w_full()
        .text_size(px(16.0))
        .line_height(px(22.0))
        .child(field)
        .when(input.read(cx).value().is_empty(), |root| {
            root.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .text_size(px(16.0))
                    .text_color(color.opacity(0.6))
                    .child("Type text…"),
            )
        })
        .into_any_element()
}

pub(super) fn row_action(button: impl IntoElement) -> gpui::Div {
    div()
        .flex_none()
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_key_down(|event, _, cx| {
            if matches!(event.keystroke.key.as_str(), "space" | "enter")
                && !event.keystroke.modifiers.modified()
            {
                cx.stop_propagation();
            }
        })
        .on_key_up(|event, _, cx| {
            if matches!(event.keystroke.key.as_str(), "space" | "enter")
                && !event.keystroke.modifiers.modified()
            {
                cx.stop_propagation();
            }
        })
        .child(button)
}

/// The app's content area in window pixels. While GPUI's inspector is open it
/// owns a sidebar at the window's right, which annotations must not cover and
/// normalized coordinates must not stretch into.
pub(super) fn viewport(window: &Window, cx: &App) -> (f32, f32) {
    let size = window.viewport_size();
    let width = if inspector::sidebar_open(window, cx) {
        (size.width - inspector::sidebar_width(window)).max(px(0.))
    } else {
        size.width
    };
    (width.into(), size.height.into())
}

fn normalized(at: gpui::Point<gpui::Pixels>, window: &Window, cx: &App) -> (f32, f32) {
    let (w, h) = viewport(window, cx);
    (
        (f32::from(at.x) / w.max(1.0)).clamp(0.0, 1.0),
        (f32::from(at.y) / h.max(1.0)).clamp(0.0, 1.0),
    )
}

pub(super) fn consumer_action(
    owner: Entity<AnnotationOverlay>,
    window: &mut Window,
    cx: &mut App,
    action: impl FnOnce(&mut AnnotationOverlay, &mut Window, &mut Context<AnnotationOverlay>) + 'static,
) {
    let host = crate::inspection::inspector_target(window, cx);
    if host == window.window_handle() {
        owner.update(cx, |this, cx| action(this, window, cx));
    } else {
        cx.defer(move |cx| {
            let _ = host.update(cx, |_, window, cx| {
                owner.update(cx, |this, cx| action(this, window, cx))
            });
        });
    }
}

pub(super) fn notes_body(
    this: &AnnotationOverlay,
    owner: Entity<AnnotationOverlay>,
    close: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    sheet: bool,
    window: &mut Window,
    cx: &App,
) -> gpui::Div {
    let notes = this.store.app_annotations(&this.session.app_name);
    let mut rows = div()
        .id("feedback-notes-list")
        .role(gpui::Role::ListBox)
        .aria_label("All app annotations")
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .track_focus(&this.notes_focus)
        .max_h(if sheet {
            (window.viewport_size().height - gpui::rems(14.).to_pixels(window.rem_size()))
                .max(px(80.))
        } else {
            window.viewport_size().height * 0.5
        });
    let mut message = None;
    let has_notes = notes.as_ref().is_ok_and(|notes| !notes.is_empty());
    match notes {
        Ok(notes) => {
            if notes.is_empty() {
                message = Some("No annotations yet.".to_string());
            }
            for note in notes {
                let id = note.id.clone();
                let edit_owner = owner.clone();
                let edit_close = close.clone();
                let edit_id = id.clone();
                let edit = Button::new(format!("feedback-edit-{id}"))
                    .icon(IconName::Pencil)
                    .small()
                    .ghost()
                    .tooltip(format!("Edit note {}", note.index))
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        edit_close(window, cx);
                        let edit_id = edit_id.clone();
                        consumer_action(edit_owner.clone(), window, cx, move |this, window, cx| {
                            this.edit(&edit_id, window, cx)
                        });
                    });
                let delete_owner = owner.clone();
                let delete_id = id.clone();
                let delete = Button::new(format!("feedback-delete-{id}"))
                    .icon(IconName::Trash)
                    .small()
                    .ghost()
                    .tooltip(format!("Delete note {}", note.index))
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        let delete_id = delete_id.clone();
                        consumer_action(
                            delete_owner.clone(),
                            window,
                            cx,
                            move |this, window, cx| {
                                this.delete(&delete_id, window, cx);
                            },
                        );
                    });
                // previous presentation ListItem uses a single-line ellipsis renderer. Keep the
                // complete comment in storage/editor, flatten only its preview.
                let preview = note
                    .feedback
                    .comment
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let preview = if preview.is_empty() && note.feedback.inspection.is_some() {
                    format!("Inspection: {}", note.feedback.element)
                } else {
                    preview
                };
                let selected = this.selected_note.as_deref() == Some(id.as_str());
                let row_owner = owner.clone();
                let row_close = close.clone();
                let focus = this.notes_focus.clone();
                let row_id = id.clone();
                rows =
                    rows.child(
                        gpui::component::list::ListItem::new(format!("feedback-note-{id}"))
                            .selected(selected)
                            .accessibility_label(preview.clone())
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .w_full()
                                    .when(sheet, |body| {
                                        body.child(number_badge(
                                            note.index.to_string(),
                                            ControlSize::Sm,
                                            cx,
                                        ))
                                    })
                                    .child(Icon::new(annotation_icon(note.feedback.shape.as_ref())))
                                    .child(
                                        div().flex_1().min_w_0().child(preview).child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(note.url.clone()),
                                        ),
                                    )
                                    .child(row_action(edit))
                                    .child(row_action(delete)),
                            )
                            .on_click(move |event, window, cx| {
                                window.focus(&focus, cx);
                                row_owner.update(cx, |this, cx| {
                                    this.selected_note = Some(row_id.clone());
                                    cx.notify();
                                });
                                if event.click_count() > 1 {
                                    row_close(window, cx);
                                    let id = row_id.clone();
                                    consumer_action(
                                        row_owner.clone(),
                                        window,
                                        cx,
                                        move |this, window, cx| {
                                            if this.store.annotation(&id).is_ok_and(|note| {
                                                note.feedback.inspection.is_some()
                                            }) {
                                                this.open_inspection(&id, window, cx);
                                            } else {
                                                this.edit(&id, window, cx);
                                            }
                                        },
                                    );
                                }
                            }),
                    );
            }
        }
        Err(error) => message = Some(error.to_string()),
    }
    let keyboard_owner = owner.clone();
    let keyboard_close = close.clone();
    rows = rows.on_key_down(move |event, window, cx| {
        let key = event.keystroke.key.as_str();
        if !matches!(key, "up" | "down" | "enter" | "space") {
            return;
        }
        let selected = keyboard_owner.update(cx, |this, cx| {
            let notes = this
                .store
                .app_annotations(&this.session.app_name)
                .unwrap_or_default();
            if notes.is_empty() {
                return None;
            }
            let current = notes
                .iter()
                .position(|n| Some(&n.id) == this.selected_note.as_ref());
            let index = match key {
                "up" => current.unwrap_or(0).saturating_sub(1),
                "down" => current.map(|i| (i + 1).min(notes.len() - 1)).unwrap_or(0),
                _ => current.unwrap_or(0),
            };
            this.selected_note = Some(notes[index].id.clone());
            cx.notify();
            Some((
                notes[index].id.clone(),
                notes[index].feedback.inspection.is_some(),
            ))
        });
        if matches!(key, "enter" | "space") {
            if let Some((id, inspection)) = selected {
                keyboard_close(window, cx);
                consumer_action(
                    keyboard_owner.clone(),
                    window,
                    cx,
                    move |this, window, cx| {
                        if inspection {
                            this.open_inspection(&id, window, cx);
                        } else {
                            this.edit(&id, window, cx);
                        }
                    },
                );
            }
        }
        cx.stop_propagation();
    });
    let add_owner = owner.clone();
    let add_close = close.clone();
    let owner = owner.clone();
    let add = Button::new("feedback-add")
        .label("Add")
        .icon(IconName::Plus)
        .on_click(move |_, window, cx| {
            add_close(window, cx);
            consumer_action(add_owner.clone(), window, cx, |this, window, cx| {
                this.choose_tool(None, window, cx);
            });
        });
    let copy = Button::new("feedback-copy-markdown")
        .label("Copy")
        .icon(IconName::Copy)
        .on_click(move |_, _, cx| {
            owner.update(cx, |this, cx| {
                match this.store.app_markdown(&this.session.app_name) {
                    Ok(markdown) => {
                        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
                        this.notice = "Copied Markdown for agent".into();
                    }
                    Err(error) => this.notice = error.to_string(),
                }
                cx.notify();
            });
        });
    let content = div()
        .w_full()
        .flex()
        .flex_col()
        .gap_3()
        .when_some(message, |panel, message| {
            panel.child(
                div()
                    .py_2()
                    .text_color(cx.theme().colors.muted_foreground)
                    .child(message),
            )
        })
        .when(has_notes, |panel| {
            panel.child(
                div()
                    .w_full()
                    .debug_selector(|| "feedback-comments-list".into())
                    .child(rows),
            )
        });
    if sheet {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("feedback-comments-scroll")
                    .debug_selector(|| "feedback-comments-scroll".into())
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .p_6()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(content)
                            .child(
                                div()
                                    .w_full()
                                    .debug_selector(|| "feedback-comments-add".into())
                                    .child(add.w_full()),
                            ),
                    ),
            )
            .when(!this.notice.is_empty(), |panel| {
                panel.child(div().px_6().pb_3().child(this.notice.clone()))
            })
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .border_t_1()
                    .border_color(cx.theme().colors.border)
                    .p_6()
                    .flex()
                    .justify_end()
                    .debug_selector(|| "feedback-comments-footer".into())
                    .child(
                        div()
                            .debug_selector(|| "feedback-comments-copy".into())
                            .child(copy),
                    ),
            )
    } else {
        div()
            .w(gpui::rems(17.6))
            .max_w(window.viewport_size().width - px(32.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Annotations"),
            )
            .child(content)
            .child(div().flex().justify_end().gap_2().child(add).child(copy))
            .when(!this.notice.is_empty(), |panel| {
                panel.child(this.notice.clone())
            })
    }
}

impl AnnotationOverlay {
    fn annotation_interaction_started(&self) -> bool {
        self.draft.is_some()
            || !self.stroke.is_empty()
            || self.pin_drag.is_some()
            || self.textbox_drag.is_some()
            || self.textbox_resize.is_some()
    }

    pub(super) fn finish_stroke(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.finish_textbox_resize(cx) {
            return;
        }
        if let Some((before, offset)) = self.textbox_drag.take() {
            let p = normalized(at, window, cx);
            let after = super::textbox::moved_textbox(
                &before,
                self.textbox_drag_position(
                    &before.id,
                    (p.0 - offset.0, p.1 - offset.1),
                    window,
                    cx,
                ),
                viewport(window, cx),
            );
            if before != after {
                match self.store.replace_note(Some(&before), Some(&after)) {
                    Ok(()) => self.record_change(Some(before), Some(after)),
                    Err(error) => self.notice = error.to_string(),
                }
            }
            if let Ok(session) = self.store.session(&self.session.id) {
                self.session = session;
            }
            cx.notify();
            return;
        }
        if let Some((before, offset)) = self.pin_drag.take() {
            // The dropped spot is the new anchor; re-seed it from the target's current position.
            self.follow_anchors.remove(&before.id);
            let mut after = before.clone();
            let p = normalized(at, window, cx);
            let p = (
                (p.0 - offset.0).clamp(0.0, 1.0),
                (p.1 - offset.1).clamp(0.0, 1.0),
            );
            let (_, h) = viewport(window, cx);
            after.feedback.x = p.0 * 100.0;
            after.feedback.y = p.1 * h;
            if let Some(AnnotationShape::Pin { at, .. }) = &mut after.feedback.shape {
                *at = p;
                after.feedback.bounding_box.x = p.0 * f32::from(window.viewport_size().width);
                after.feedback.bounding_box.y = p.1 * h;
            }
            if before != after {
                match self.store.replace_note(Some(&before), Some(&after)) {
                    Ok(()) => {
                        self.record_change(Some(before), Some(after));
                    }
                    Err(error) => self.notice = error.to_string(),
                }
            }
            if let Ok(session) = self.store.session(&self.session.id) {
                self.session = session;
            }
            cx.notify();
            return;
        }
        let points = std::mem::take(&mut self.stroke);
        let Some(tool) = self.tool else {
            return;
        };
        let shape = if tool == AnnotationTool::Text && !points.is_empty() {
            // A drag frames the box; a click places one sized to its text.
            Some(crate::drawing::text_box(
                points[0],
                normalized(at, window, cx),
                viewport(window, cx),
                self.color,
            ))
        } else {
            crate::drawing::finish_shape(tool, points, normalized(at, window, cx), self.color)
        };
        let Some(shape) = shape else {
            return;
        };
        let (w, h) = viewport(window, cx);
        let bounds = shape.bounds(w, h);
        self.draft_textbox_bounds = None;
        self.draft = Some(Draft {
            index: None,
            editing: true,
            original_comment: String::new(),
            color: self.color,
            url: self.session.url.clone(),
            target: Target {
                id: format!("drawing:{}", uuid::Uuid::new_v4()),
                label: tool.persisted_label().into(),
                source: String::new(),
                bounds,
            },
            shape: Some(shape),
            id: None,
            x: (bounds.x + bounds.width / 2.0) / w.max(1.0) * 100.0,
            y: bounds.y + bounds.height / 2.0,
        });
        self.notice.clear();
        self.input
            .update(cx, |input, cx| input.set_comment("", window, cx));
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn advance_drag(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.textbox_resize.is_some() {
            let pointer = normalized(at, window, cx);
            self.advance_textbox_resize(pointer, window, cx);
        } else if let Some((before, offset)) = &self.textbox_drag {
            let p = normalized(at, window, cx);
            let after = super::textbox::moved_textbox(
                before,
                self.textbox_drag_position(
                    &before.id,
                    (p.0 - offset.0, p.1 - offset.1),
                    window,
                    cx,
                ),
                viewport(window, cx),
            );
            if let Some(note) = self
                .session
                .annotations
                .iter_mut()
                .find(|n| n.id == before.id)
            {
                *note = after;
            }
            cx.notify();
        } else if let Some((before, offset)) = &self.pin_drag {
            let p = normalized(at, window, cx);
            let p = (
                (p.0 - offset.0).clamp(0.0, 1.0),
                (p.1 - offset.1).clamp(0.0, 1.0),
            );
            if let Some(note) = self
                .session
                .annotations
                .iter_mut()
                .find(|n| n.id == before.id)
            {
                note.feedback.x = p.0 * 100.0;
                note.feedback.y = p.1 * f32::from(window.viewport_size().height);
                if let Some(AnnotationShape::Pin { at, .. }) = &mut note.feedback.shape {
                    *at = p;
                }
            }
            cx.notify();
        } else if !self.stroke.is_empty() && self.stroke.len() < 4096 {
            self.pointer = Some((at.x.into(), at.y.into()));
            let next = normalized(at, window, cx);
            if self.stroke.last() != Some(&next) {
                self.stroke.push(next);
                cx.notify();
            }
        }
    }

    fn activate_annotation(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .store
            .annotation(id)
            .is_ok_and(|note| note.feedback.inspection.is_some())
        {
            self.open_inspection(id, window, cx);
        } else {
            self.edit(id, window, cx);
        }
    }

    fn shape_at(&self, at: gpui::Point<gpui::Pixels>, window: &Window, cx: &App) -> Option<String> {
        self.session
            .annotations
            .iter()
            .rev()
            .find(|note| {
                matches!(
                    note.status,
                    AnnotationStatus::Pending | AnnotationStatus::Acknowledged
                ) && (self
                    .textbox_bounds
                    .get(&note.id)
                    .is_some_and(|bounds| bounds.contains(&at))
                    || note.feedback.shape.as_ref().is_some_and(|shape| {
                        // Hit the shape where it is drawn: undo its scroll shift.
                        self.scroll_shift(note, window, cx).is_some_and(|shift| {
                            shape.hit(
                                (f32::from(at.x) - shift.0, f32::from(at.y) - shift.1),
                                viewport(window, cx),
                                8.0,
                            )
                        })
                    }))
            })
            .map(|note| note.id.clone())
    }

    fn draft_bounds(&self, window: &Window, cx: &App) -> Option<Rect> {
        let draft = self.draft.as_ref()?;
        let (w, h) = viewport(window, cx);
        let mut bounds = draft
            .shape
            .as_ref()
            .map_or(draft.target.bounds, |shape| shape.bounds(w, h));
        if let Some(id) = &draft.id {
            let shift = if draft.shape.is_some() {
                self.session
                    .annotations
                    .iter()
                    .find(|note| &note.id == id)
                    .map(|note| self.scroll_shift(note, window, cx))
                    .unwrap_or(Some((0., 0.)))?
            } else if let Some(anchor) = self.follow_anchors.get(id) {
                let at = target::origin(window, cx, &anchor.target)?;
                let base = anchor.baseline.get().unwrap_or(at);
                (at.0 - base.0, at.1 - base.1)
            } else {
                (0., 0.)
            };
            bounds.x += shift.0;
            bounds.y += shift.1;
        } else if draft.shape.is_none() {
            bounds = target::targets(window, cx)
                .into_iter()
                .find(|t| draft.url == self.session.url && t.id == draft.target.id)
                .map_or(bounds, |target| target.bounds);
        }
        Some(bounds)
    }

    fn panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (w, h) = viewport(window, cx);
        let annotation = self.draft_bounds(window, cx).map(|mut bounds| {
            let marker_height = f32::from(
                cx.theme()
                    .control_height(ControlSize::Lg)
                    .to_pixels(window.rem_size()),
            );
            if bounds.height < marker_height {
                bounds.y -= (marker_height - bounds.height) / 2.0;
                bounds.height = marker_height;
            }
            bounds
        });
        let width =
            f32::from(gpui::rems(16.0).to_pixels(window.rem_size())).min((w - 16.0).max(1.0));
        let panel_size = (width, self.editor_size.1.min((h - 16.0).max(1.0)));
        let is_text = self
            .draft
            .as_ref()
            .is_some_and(|d| matches!(d.shape, Some(AnnotationShape::Text { .. })));
        let at = if is_text {
            let bounds = annotation.unwrap_or_default();
            (
                bounds.x.clamp(8.0, (w - width - 8.0).max(8.0)),
                bounds.y.clamp(8.0, (h - panel_size.1 - 8.0).max(8.0)),
            )
        } else {
            match annotation {
                Some(bounds) => note_panel_position(bounds, panel_size, (w, h), 8.0),
                None => centered_panel((w / 2.0, h / 2.0), panel_size, (w, h), 8.0),
            }
        };
        let mut content = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.editor(window, cx));
        if !self.notice.is_empty() {
            content = content.child(self.notice.clone());
        }
        let this = cx.entity().downgrade();
        let theme = cx.theme();
        div()
            .id("feedback-editor-panel")
            .debug_selector(|| "feedback-editor-panel".into())
            .absolute()
            .on_mouse_down_out(
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    if event.button == MouseButton::Left {
                        if this
                            .mentions
                            .bounds
                            .is_some_and(|bounds| bounds.contains(&event.position))
                        {
                            return;
                        }
                        let dismissed = this.dismiss_note(window, cx);
                        window.prevent_default();
                        if !dismissed {
                            cx.stop_propagation();
                        }
                    }
                }),
            )
            .left(px(at.0))
            .top(px(at.1))
            .occlude()
            .child(
                BoundsObserver::new("feedback-editor-measure", move |bounds, _, cx| {
                    let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                    let _ = this.update(cx, |this, cx| {
                        if this.editor_size != size {
                            this.editor_size = size;
                            cx.notify();
                        }
                    });
                })
                .w(px(width))
                .max_h(px((h - 16.0).max(1.0)))
                .p_3()
                .bg(annotation_style::palette(cx)
                    [self.draft.as_ref().map_or(self.color, |d| d.color)])
                .text_color(theme.colors.background)
                .border_1()
                .border_color(theme.colors.background.opacity(0.25))
                .rounded(theme.radius_tokens().lg)
                .shadow(theme.shadow_tokens().sm)
                .child(
                    div()
                        .id("feedback-editor-scroll")
                        .max_h(px((h - 48.0).max(1.0)))
                        .overflow_y_scroll()
                        .child(content),
                ),
            )
    }

    /// A border around the element a draft comment is attached to, echoing the
    /// outline Inspect draws. Only for element-attached drafts (no drawn shape).
    fn selection_outline(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let draft = self.draft.as_ref()?;
        if draft.shape.is_some() || self.inspected_id.is_some() {
            return None;
        }
        let bounds = self.draft_bounds(window, cx)?;
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return None;
        }
        let color = annotation_style::palette(cx)[draft.color];
        Some(
            div()
                .debug_selector(|| "feedback-selection-outline".into())
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.width))
                .h(px(bounds.height))
                .border_2()
                .border_color(color)
                .rounded(cx.theme().radius_tokens().sm)
                .into_any_element(),
        )
    }

    pub(super) fn surface(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // Full-window annotation divs are inspectable hitboxes in GPUI. Omit
        // them while inspecting so the picker reaches the host's real elements.
        if self.inspecting
            && window.is_inspector_picking(cx)
            && !self.settings_open
            && !self.review_open
        {
            return div().into_any_element();
        }
        let active = self.active || self.draft.is_some();
        let (w, h) = viewport(window, cx);
        let owner = cx.entity_id();
        let start_drag = cx.entity().downgrade();
        let mut layer = div().absolute().size_full();
        // A reopened editor can be focused while drawing mode is off. Keep
        // its editable geometry stable, but leave scrolling available for
        // unfocused previews and preserve the editor's own scrolling.
        if self.draft.as_ref().is_some_and(|draft| draft.editing)
            && self.input.focus_handle(cx).contains_focused(window, cx)
        {
            layer = layer.child(
                div()
                    .id("feedback-editor-scroll-guard")
                    .absolute()
                    .size_full()
                    .on_scroll(|_, _, cx| cx.stop_propagation()),
            );
        }
        if self.active && !self.inspecting && !self.review_open && self.tool.is_some() {
            let drawing = self.active && self.tool.is_some() && self.draft.is_none();
            let picker = div()
                .id("feedback-picker")
                .absolute()
                .w_full()
                .top_0()
                .bottom_0()
                .when(self.annotation_interaction_started(), |picker| {
                    picker.occlude()
                })
                .cursor(if self.draft.is_some() {
                    CursorStyle::Arrow
                } else {
                    tool_cursor(self.tool)
                })
                .on_scroll(cx.listener(|this, _, _, cx| {
                    if this.annotation_interaction_started() {
                        cx.stop_propagation();
                    }
                }))
                .on_hover(cx.listener(|this, hovered, _, cx| {
                    if !hovered && this.draft.is_none() && this.hovered.take().is_some() {
                        cx.notify();
                    }
                }))
                .on_mouse_move(
                    cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                        if this.draft.is_some() {
                            return;
                        }
                        this.pointer = Some((event.position.x.into(), event.position.y.into()));
                        if this.tool.is_some() {
                            cx.notify();
                            return;
                        }
                        let hovered = target::pick(
                            window,
                            cx,
                            event.position.x.into(),
                            event.position.y.into(),
                        );
                        if hovered.as_ref().map(|t| (&t.id, t.bounds))
                            != this.hovered.as_ref().map(|t| (&t.id, t.bounds))
                        {
                            this.hovered = hovered;
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        // A note's outside-click handler already handled this
                        // press; commands and pins may proceed, but the picker
                        // must not begin another draft on the same press.
                        if window.default_prevented() || this.draft.is_some() {
                            return;
                        }
                        if let Some(id) = this.shape_at(event.position, window, cx) {
                            this.activate_annotation(&id, window, cx);
                            return;
                        }
                        if this.tool.is_some() {
                            this.stroke = vec![normalized(event.position, window, cx)];
                            cx.notify();
                            return;
                        }
                        let size = viewport(window, cx);
                        let at = (f32::from(event.position.x), f32::from(event.position.y));
                        if this.tool.is_none() {
                            if let Some(target) = target::pick(window, cx, at.0, at.1) {
                                let center = (
                                    target.bounds.x + target.bounds.width / 2.0,
                                    target.bounds.y + target.bounds.height / 2.0,
                                );
                                this.select(
                                    target,
                                    center.0 / size.0.max(1.0) * 100.0,
                                    center.1,
                                    window,
                                    cx,
                                );
                            }
                        }
                    }),
                )
                // The picker occludes the surface, so right click reaches the
                // element or tools menu (or a note's menu) here, not the root.
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        if let Some(id) = this.shape_at(event.position, window, cx) {
                            this.show_annotation_menu(&id, event.position, window, cx);
                        } else {
                            this.open_canvas_menu(event.position, window, cx);
                        }
                    }),
                )
                .when(drawing, |pad| {
                    pad.on_drag(FeedbackDrag { owner }, move |_, _, window, cx| {
                        let _ = start_drag.update(cx, |this, cx| {
                            this.advance_drag(window.mouse_position(), window, cx)
                        });
                        cx.new(|_| EmptyView)
                    })
                });
            layer = layer.child(picker);
        }
        if self.active && self.draft.is_none() && self.tool.is_some() {
            if let Some(at) = self.pointer {
                if let Some((_, icon, _, _)) = super::toolbar_window::TOOLS
                    .iter()
                    .find(|(tool, _, _, _)| *tool == self.tool)
                {
                    layer = layer.child(
                        div()
                            .absolute()
                            .left(px((at.0 + 14.0).min(w - 24.0)))
                            .top(px((at.1 + 14.0).min(h - 24.0)))
                            .text_color(annotation_style::palette(cx)[self.color])
                            .child(Icon::new(*icon)),
                    );
                }
            }
        }
        self.sync_anchors(window, cx);
        let selected_id = self.draft.as_ref().and_then(|d| d.id.as_deref());
        // Notes as they appear now: drawn ones follow the content they sit on.
        let visible: Vec<crate::Annotation> = self
            .session
            .annotations
            .iter()
            .filter(|a| {
                matches!(
                    a.status,
                    AnnotationStatus::Pending | AnnotationStatus::Acknowledged
                ) || Some(a.id.as_str()) == selected_id
            })
            .filter_map(|a| self.displayed(a, window, cx))
            .collect();
        let mut shapes: Vec<_> = visible
            .iter()
            .filter_map(|a| a.feedback.shape.clone())
            .collect();
        if let Some(shape) = self
            .draft
            .as_ref()
            .filter(|d| d.id.is_none())
            .and_then(|d| d.shape.clone())
        {
            shapes.push(shape);
        }
        if let Some(tool) = self.tool {
            if let Some(sketch) = AnnotationShape::from_stroke(tool, &self.stroke, self.color) {
                shapes.push(sketch);
            }
        }
        // The Textbox tool frames its box while dragging, as Freeform does.
        if self.tool == Some(AnnotationTool::Text) && self.draft.is_none() {
            if let (Some(start), Some(end)) = (self.stroke.first(), self.stroke.last()) {
                let frame = crate::drawing::text_box(*start, *end, (w, h), self.color);
                if let AnnotationShape::Text {
                    at,
                    width: Some(width),
                    height: Some(height),
                    ..
                } = frame
                {
                    layer = layer.child(
                        div()
                            .debug_selector(|| "feedback-textbox-frame".into())
                            .absolute()
                            .left(px(at.0 * w))
                            .top(px(at.1 * h))
                            .w(px(width * w))
                            .h(px(height * h))
                            .border_1()
                            .border_dashed()
                            .border_color(annotation_style::palette(cx)[self.color])
                            .rounded(cx.theme().radius_tokens().sm),
                    );
                }
            }
        }
        let colors = annotation_style::palette(cx);
        let (stroke, head) = annotation_style::stroke_metrics(window, cx);
        layer = layer.child(
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    for shape in shapes {
                        shape.paint((w, h), stroke, head, colors[shape.color()], window);
                    }
                },
            )
            .absolute()
            .size_full(),
        );
        for a in &visible {
            if a.feedback.shape.is_none()
                && !a.feedback.element_path.starts_with("gpui:")
                && !target::targets(window, cx)
                    .iter()
                    .any(|t| t.id == a.feedback.element_path)
            {
                continue;
            }
            let bounds = Rect {
                x: a.feedback.x / 100.0 * w,
                y: a.feedback.y,
                width: 0.0,
                height: 0.0,
            };
            if matches!(a.feedback.shape, Some(AnnotationShape::Text { .. })) {
                layer = layer.child(self.textbox(a, window, cx));
            } else if a.feedback.shape.is_none() {
                if !self.follow_anchors.contains_key(&a.id) {
                    let target = if a.feedback.element_path.starts_with("gpui:") {
                        super::followed::inspection_target(&a.feedback.element_path, window, cx)
                    } else {
                        Some(a.feedback.element_path.clone())
                    };
                    if let Some(target) = target {
                        // Selection geometry and this origin come from the same
                        // completed frame, before the host's next prepaint.
                        let baseline = std::rc::Rc::new(std::cell::Cell::new(target::origin(
                            window, cx, &target,
                        )));
                        self.follow_anchors
                            .insert(a.id.clone(), super::followed::Anchor { target, baseline });
                    }
                }
                if let Some(anchor) = self.follow_anchors.get(&a.id) {
                    layer = layer.child(super::followed::FollowMark {
                        target: anchor.target.clone(),
                        baseline: anchor.baseline.clone(),
                        center: (bounds.x, bounds.y),
                        marker: self
                            .marker(&a.id, a.index, Rect::default(), window, cx)
                            .into_any_element(),
                    });
                } else {
                    layer = layer.child(self.marker(&a.id, a.index, bounds, window, cx));
                }
            } else {
                layer = layer.child(self.marker(&a.id, a.index, bounds, window, cx));
            }
        }
        if let Some(outline) = self.selection_outline(window, cx) {
            layer = layer.child(outline);
        }
        let text_draft = matches!(
            self.draft.as_ref().and_then(|d| d.shape.as_ref()),
            Some(AnnotationShape::Text { .. })
        );
        if let Some(editor) = self.draft_textbox(window, cx) {
            layer = layer.child(editor);
        }
        // Textboxes edit on the canvas; only other drafts get the floating editor.
        if self.draft_bounds(window, cx).is_some() && self.inspected_id.is_none() && !text_draft {
            layer = layer.child(self.panel(window, cx));
        }
        if let Some(popup) = self.mention_popup(window, cx) {
            layer = layer.child(popup);
        }
        if self.settings_open {
            layer = layer.child(self.sheet(window, cx));
        }
        layer = layer.child(self.context_layer(window, cx));
        let scope = div().size_full().child(layer);
        let scope = if active && !self.banner {
            scope
                .focus_trap("annotation-focus", &self.focus)
                .into_any_element()
        } else {
            scope.into_any_element()
        };
        raise(
            "gpui-agentation-overlay",
            anchored().position(point(px(0.0), px(0.0))).child(
                div()
                    .id("feedback-surface")
                    .when(
                        self.active && !self.settings_open && !self.review_open,
                        |root| root.key_context(self.shortcut_context(window, cx)),
                    )
                    .capture_action(cx.listener(
                        |this, action: &super::commands::AnnotationCommand, window, cx| {
                            this.dispatch_command(action, window, cx);
                            cx.stop_propagation();
                        },
                    ))
                    .relative()
                    .w(px(w))
                    .h(px(h))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                            if this.settings_open || this.review_open {
                                return;
                            }
                            if let Some(id) = this.shape_at(event.position, window, cx) {
                                this.show_annotation_menu(&id, event.position, window, cx);
                                cx.stop_propagation();
                            } else if this.active {
                                // Empty canvas: the clicked element's menu, else tools.
                                this.open_canvas_menu(event.position, window, cx);
                                cx.stop_propagation();
                            }
                        }),
                    )
                    .on_drag_move(cx.listener(
                        move |this, event: &DragMoveEvent<FeedbackDrag>, window, cx| {
                            if event.drag(cx).owner == owner {
                                this.advance_drag(event.event.position, window, cx);
                            }
                        },
                    ))
                    .when(
                        !self.active || (self.inspecting && self.inspected_id.is_some()),
                        |root| {
                            root.when(self.over_shape, |root| root.cursor_pointer())
                                .on_mouse_move(cx.listener(
                                    |this, event: &gpui::MouseMoveEvent, window, cx| {
                                        let over =
                                            this.shape_at(event.position, window, cx).is_some();
                                        if this.over_shape != over {
                                            this.over_shape = over;
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, window, cx| {
                                            if let Some(id) =
                                                this.shape_at(event.position, window, cx)
                                            {
                                                this.activate_annotation(&id, window, cx);
                                                cx.stop_propagation();
                                                return;
                                            }
                                            if this.inspecting
                                                && this.inspected_id.is_some()
                                                && !super::inspector::is_picking(window, cx)
                                                && crate::inspection::inspector_selection(
                                                    window, cx,
                                                )
                                                .filter(|selection| {
                                                    Some(&selection.id)
                                                        == this.inspected_id.as_ref()
                                                })
                                                .is_some_and(|selection| {
                                                    !selection.bounds.contains(&event.position)
                                                })
                                            {
                                                super::inspector::resume_picking(window, cx);
                                                cx.stop_propagation();
                                            }
                                        },
                                    ),
                                )
                        },
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseUpEvent, window, cx| {
                            this.finish_stroke(event.position, window, cx)
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseUpEvent, window, cx| {
                            this.finish_stroke(event.position, window, cx)
                        }),
                    )
                    .capture_key_down(cx.listener(
                        |this, event: &gpui::KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" && this.take_mention_escape() {
                                cx.stop_propagation();
                                return;
                            }
                            if event.keystroke.key == "escape"
                                && this.inspecting
                                && !this.settings_open
                                && !this.review_open
                            {
                                super::inspector::resume_picking(window, cx);
                                cx.stop_propagation();
                                return;
                            }
                            if event.keystroke.key == "escape"
                                && (this.settings_open || this.review_open)
                            {
                                this.dismiss_sheet(window, cx);
                                cx.stop_propagation();
                                return;
                            }
                            if event.keystroke.key == "escape"
                                && (this.active || this.draft.is_some())
                            {
                                this.stroke.clear();
                                let dragging = this.pin_drag.take().is_some()
                                    | this.textbox_drag.take().is_some()
                                    | this.textbox_resize.take().is_some();
                                if dragging {
                                    if let Ok(session) = this.store.session(&this.session.id) {
                                        this.session = session;
                                    }
                                }
                                if dragging {
                                    window.focus(&this.focus, cx);
                                    cx.notify();
                                } else if this.draft.take().is_some() {
                                    this.return_focus(window, cx);
                                    cx.notify();
                                } else {
                                    this.close(window, cx);
                                }
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .child(scope),
            ),
        )
        .into_any_element()
    }

    pub(super) fn marker(
        &self,
        id: &str,
        index: u64,
        bounds: Rect,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let id = id.to_string();
        let center = (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        let scale = self.pin_scale(&id, window, cx);
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size())
            * scale;
        let focus = window
            .use_keyed_state(format!("feedback-marker-focus-{id}"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone()
            .tab_stop(true);
        let amber = self
            .session
            .annotations
            .iter()
            .find(|n| n.id == id)
            .map_or(annotation_style::palette(cx)[self.color], |note| {
                annotation_style::palette(cx)[note.annotation_color()]
            });
        let drag_id = id.clone();
        let menu_id = id.clone();
        let drag_owner = cx.entity().downgrade();
        let owner = cx.entity_id();
        let button = number_badge(index.to_string(), ControlSize::Lg, cx)
            .size(side)
            .text_size(cx.theme().typography_tokens().sm.size * scale)
            .bg(amber)
            .debug_selector(move || format!("feedback-marker-number-{index}"))
            .id(format!("feedback-marker-{id}"))
            .role(Role::Button)
            .aria_label(format!("Edit comment {index}"))
            .track_focus(&focus)
            .focus(|style| style.border_color(cx.theme().ring))
            .hover(move |style| style.bg(amber.opacity(0.85)))
            .active(move |style| style.bg(amber.opacity(0.7)))
            .on_drag(FeedbackDrag { owner }, move |_, _, window, cx| {
                let _ = drag_owner.update(cx, |this, cx| {
                    if !this.dismiss_note(window, cx) {
                        return;
                    }
                    if this.displaced(&drag_id, window, cx) {
                        this.notice =
                            "Scroll the annotation fully into view to move or resize it.".into();
                        cx.notify();
                        return;
                    }
                    if let Ok(note) = this.store.annotation(&drag_id) {
                        if note.feedback.inspection.is_some() {
                            return;
                        }
                        if matches!(note.feedback.shape, Some(AnnotationShape::Text { .. })) {
                            this.begin_textbox_drag(&drag_id, window, cx);
                            this.advance_drag(window.mouse_position(), window, cx);
                            return;
                        }
                        let (_, h) = viewport(window, cx);
                        let p = normalized(
                            this.annotation_press
                                .take()
                                .unwrap_or(window.mouse_position()),
                            window,
                            cx,
                        );
                        let offset = (
                            p.0 - note.feedback.x / 100.0,
                            p.1 - note.feedback.y / h.max(1.0),
                        );
                        this.pin_drag = Some((note, offset));
                        this.advance_drag(window.mouse_position(), window, cx);
                    }
                });
                cx.new(|_| EmptyView)
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    this.commit_scroll(window, cx);
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
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    if this.session.annotations.iter().any(|n| {
                        n.id == id && matches!(n.feedback.shape, Some(AnnotationShape::Text { .. }))
                    }) && event.click_count() < 2
                    {
                        this.select_textbox(&id, window, cx);
                    } else {
                        this.edit(&id, window, cx);
                    }
                }),
            );
        let at = centered_panel(
            center,
            (side.into(), side.into()),
            viewport(window, cx),
            0.0,
        );
        div()
            .absolute()
            .left(px(at.0))
            .top(px(at.1))
            .occlude()
            .child(button)
    }
}

/// Shared amber treatment for comment counts and stable annotation numbers.
pub(super) fn number_badge(
    label: impl Into<gpui::SharedString>,
    size: ControlSize,
    cx: &App,
) -> gpui::Div {
    div()
        .flex_none()
        .size(cx.theme().control_height(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(cx.theme().colors.warning)
        .text_color(cx.theme().colors.background)
        .text_size(cx.theme().typography_tokens().sm.size)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(label.into())
}

pub(super) fn annotation_count(count: usize, over: impl IntoElement, cx: &App) -> gpui::Div {
    let height = cx.theme().control_height(ControlSize::Sm) * 0.67;
    let badge = number_badge(
        if count > 99 {
            "99+".into()
        } else {
            count.to_string()
        },
        ControlSize::Sm,
        cx,
    )
    .w_auto()
    .min_w(height)
    .h(height)
    .px_1();
    div()
        .relative()
        .flex_none()
        .child(over)
        .when(count > 0, |root| {
            root.child(
                div()
                    .debug_selector(|| "feedback-annotation-count".into())
                    .absolute()
                    .top_neg_1()
                    .right_neg_1()
                    .child(badge),
            )
        })
}
