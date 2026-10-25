use super::*;
use crate::ely::{
    buttons::{ActionBar, Button, IconButton},
    forms::ColorSwatch,
    lists::{ListItem, SelectableList},
    media::Tool,
    motion,
    overlays::Popover,
    primitives::{FocusRing, FocusScope, Icon, IconName, Measure, raise},
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
    typography::{Caption, Label, Overline},
};
use crate::{
    AnnotationShape, Rect,
    drawing::{centered_panel, note_panel_position},
};
use gpui::{
    ClipboardItem, CursorStyle, DragMoveEvent, EmptyView, EntityId, MouseButton, RenderOnce, Role,
    anchored, canvas, point, px,
};

#[derive(Clone)]
pub(super) struct FeedbackDrag {
    pub owner: EntityId,
}

const TOOLS: [(Option<Tool>, IconName, &str); 6] = [
    (None, IconName::Crosshair, "Inspect element"),
    (Some(Tool::Box), IconName::Square, "Box"),
    (Some(Tool::Arrow), IconName::ArrowUpRight, "Arrow"),
    (Some(Tool::Pen), IconName::PenLine, "Pen"),
    (Some(Tool::Pin), IconName::MessageSquare, "Comment"),
    (Some(Tool::Select), IconName::Type, "Textbox"),
];

pub(super) fn tool_shortcut(tool: Option<Tool>) -> &'static str {
    match tool {
        None => "I",
        Some(Tool::Box) => "B",
        Some(Tool::Arrow) => "A",
        Some(Tool::Pen) => "P",
        Some(Tool::Pin) => "C",
        Some(Tool::Select) => "T",
    }
}

pub(super) struct WidthMotion {
    from: f32,
    to: f32,
    since: Instant,
}

impl WidthMotion {
    fn value(&self, length: Duration) -> (f32, bool) {
        let t = (self.since.elapsed().as_secs_f32() / length.as_secs_f32()).min(1.0);
        (
            motion::lerp(self.from, self.to, motion::ease_out_cubic(t)),
            t < 1.0,
        )
    }
}

fn tool_cursor(tool: Option<Tool>) -> CursorStyle {
    match tool {
        None => CursorStyle::Arrow,
        Some(_) => CursorStyle::Crosshair,
    }
}

// Match Ely's editor references: file identity first, location separate, path
// secondary. Unlike ReferencesPanel, this hover footer must remain click-through.
fn inspector_source(source: &str, cx: &App) -> gpui::Div {
    let theme = cx.theme();
    let parsed = source.rsplit_once(':').and_then(|(rest, column)| {
        let (path, line) = rest.rsplit_once(':')?;
        line.parse::<u32>().ok()?;
        column.parse::<u32>().ok()?;
        Some((path, format!("Ln {line}, Col {column}")))
    });
    let (path, location) = parsed.unwrap_or((source, String::new()));
    let (directory, filename) = path.rsplit_once('/').unwrap_or(("", path));
    div()
        .w_full()
        .flex_none()
        .pt_2()
        .border_t_1()
        .border_color(theme.colors.border)
        .flex()
        .flex_col()
        .gap_1()
        .debug_selector(|| "feedback-inspector-source".into())
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(IconName::FileCode)
                        .size(IconSize::Sm)
                        .color(theme.colors.fg_muted),
                )
                .child(
                    Label::new(filename.to_owned())
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(theme.text_size(TextSize::Sm)),
                )
                .when(!location.is_empty(), |row| {
                    row.child(
                        div()
                            .flex_none()
                            .px_1p5()
                            .py_0p5()
                            .rounded(theme.radius(Radius::Sm))
                            .bg(theme.colors.sunken)
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(theme.colors.fg_muted)
                            .debug_selector(|| "feedback-inspector-location".into())
                            .child(location),
                    )
                }),
        )
        .when(!directory.is_empty(), |footer| {
            footer.child(
                Caption::new(directory.to_owned())
                    .w_full()
                    .truncate()
                    .text_ellipsis_start()
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle),
            )
        })
}

#[derive(IntoElement)]
struct NotesTrigger {
    count: usize,
    toggle: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    owner: gpui::WeakEntity<AnnotationOverlay>,
}

impl RenderOnce for NotesTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let notes = IconButton::new("feedback-review", IconName::MessageSquare)
            .tooltip(format!("Comments ({})", self.count))
            .variant(ButtonVariant::Ghost)
            .on_click(move |_, window, cx| {
                let dismissed = self
                    .owner
                    .update(cx, |this, cx| this.dismiss_note(window, cx))
                    .unwrap_or(false);
                if dismissed {
                    (self.toggle)(window, cx);
                }
            });
        annotation_count(self.count, round_icon(notes, window, cx), cx)
            .debug_selector(|| "feedback-floating-comments".into())
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
    input: &gpui::Entity<TextInput>,
    background: gpui::Hsla,
    textbox: bool,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let mut field = Input::new(input).render(window, cx).into_any_element();
    let colors = &cx.theme().colors;
    if let Some(root) = field.downcast_mut::<gpui::Div>() {
        root.style().refine(
            &gpui::StyleRefinement::default()
                .bg(background)
                .text_color(colors.bg.opacity(0.5))
                .border_0()
                .rounded_none()
                .p_0(),
        );
    }
    div()
        .relative()
        .w_full()
        .child(field)
        .when(input.read(cx).text().is_empty(), |root| {
            root.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .text_size(cx.theme().text_size(TextSize::Sm))
                    .text_color(colors.bg.opacity(0.5))
                    .child(if textbox {
                        "Type text…"
                    } else {
                        "Describe what should change…"
                    }),
            )
        })
        .into_any_element()
}

fn row_action(button: impl IntoElement) -> gpui::Div {
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

fn viewport(window: &Window) -> (f32, f32) {
    let size = window.viewport_size();
    (size.width.into(), size.height.into())
}

pub(super) fn round_icon(
    button: IconButton,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let mut button = button.render(window, cx).into_any_element();
    if let Some(root) = button.downcast_mut::<gpui::Stateful<gpui::Div>>() {
        root.style()
            .refine(&gpui::StyleRefinement::default().rounded_full());
    }
    button
}

fn normalized(at: gpui::Point<gpui::Pixels>, window: &Window) -> (f32, f32) {
    let (w, h) = viewport(window);
    (
        (f32::from(at.x) / w.max(1.0)).clamp(0.0, 1.0),
        (f32::from(at.y) / h.max(1.0)).clamp(0.0, 1.0),
    )
}

fn toolbar_anchor(
    viewport: (f32, f32),
    size: (f32, f32),
    floating_at: Option<(f32, f32)>,
) -> (f32, f32) {
    let at = floating_at.unwrap_or(((viewport.0 - size.0) / 2.0, viewport.1 - size.1 - 48.0));
    (
        at.0.clamp(8.0, (viewport.0 - size.0 - 8.0).max(8.0)),
        at.1.clamp(8.0, (viewport.1 - size.1 - 8.0).max(8.0)),
    )
}

pub(super) fn notes_content(
    owner: gpui::WeakEntity<AnnotationOverlay>,
    close: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Div {
    let Some(owner) = owner.upgrade() else {
        return div();
    };
    notes_body(owner.read(cx), owner.clone(), close, false, window, cx)
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
    let mut rows = SelectableList::new("feedback-notes-list")
        .label("All app annotations")
        .selected(this.selected_note.iter().cloned())
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
                let edit = IconButton::new(format!("feedback-edit-{id}"), IconName::Pencil)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip(format!("Edit note {}", note.index))
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        edit_close(window, cx);
                        edit_owner.update(cx, |this, cx| this.edit(&edit_id, window, cx));
                    });
                let delete_owner = owner.clone();
                let delete_id = id.clone();
                let delete = IconButton::new(format!("feedback-delete-{id}"), IconName::Trash2)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .tooltip(format!("Delete note {}", note.index))
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        delete_owner.update(cx, |this, cx| {
                            this.delete(&delete_id, window, cx);
                        });
                    });
                // Ely ListItem uses a single-line ellipsis renderer. Keep the
                // complete comment in storage/editor, flatten only its preview.
                let preview = note
                    .feedback
                    .comment
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                rows = rows.row(
                    id.clone(),
                    ListItem::new(
                        format!("feedback-note-{id}"),
                        if sheet {
                            preview.clone()
                        } else {
                            format!("{}. {}", note.index, preview)
                        },
                    )
                    .description(note.url.clone())
                    .leading(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(sheet, |leading| {
                                leading.child(number_badge(
                                    note.index.to_string(),
                                    ControlSize::Sm,
                                    cx,
                                ))
                            })
                            .child(Icon::new(annotation_icon(note.feedback.shape.as_ref()))),
                    )
                    .trailing(
                        div()
                            .flex()
                            .gap_1()
                            .child(row_action(edit))
                            .child(row_action(delete)),
                    ),
                );
            }
        }
        Err(error) => message = Some(error.to_string()),
    }
    let select_owner = owner.clone();
    let activate_owner = owner.clone();
    let activate_close = close.clone();
    rows = rows
        .on_change(move |keys, _, cx| {
            select_owner.update(cx, |this, cx| {
                this.selected_note = keys.first().map(|key| key.to_string());
                cx.notify();
            });
        })
        .on_activate(move |id, window, cx| {
            activate_close(window, cx);
            activate_owner.update(cx, |this, cx| this.edit(id, window, cx));
        });
    let add_owner = owner.clone();
    let add_close = close.clone();
    let owner = owner.clone();
    let add = Button::new("feedback-add", "Add")
        .icon(IconName::Plus)
        .on_click(move |_, window, cx| {
            add_close(window, cx);
            add_owner.update(cx, |this, cx| {
                this.active = false;
                this.draft = None;
                this.toggle(window, cx);
            });
        });
    let copy = Button::new("feedback-copy-markdown", "Copy")
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
                    .text_color(cx.theme().colors.fg_muted)
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
                                    .child(add.full_width()),
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
    pub(super) fn finish_stroke(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.toolbar_drag.take().is_some() {
            self.persist_toolbar(cx);
        }
        if let Some((before, offset)) = self.textbox_drag.take() {
            let p = normalized(at, window);
            let after = super::textbox::moved_textbox(
                &before,
                (
                    (p.0 - offset.0).clamp(0.0, 1.0),
                    (p.1 - offset.1).clamp(0.0, 1.0),
                ),
                viewport(window),
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
            let mut after = before.clone();
            let p = normalized(at, window);
            let p = (
                (p.0 - offset.0).clamp(0.0, 1.0),
                (p.1 - offset.1).clamp(0.0, 1.0),
            );
            let (_, h) = viewport(window);
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
        let shape = if tool == Tool::Select && !points.is_empty() {
            Some(AnnotationShape::Text {
                at: points[0],
                text: String::new(),
                color: self.color,
            })
        } else {
            crate::drawing::finish_shape(tool, points, normalized(at, window), self.color)
        };
        let Some(shape) = shape else {
            return;
        };
        let (w, h) = viewport(window);
        let bounds = shape.bounds(w, h);
        self.draft = Some(Draft {
            index: None,
            editing: true,
            original_comment: String::new(),
            color: self.color,
            url: self.session.url.clone(),
            target: Target {
                id: format!("drawing:{}", uuid::Uuid::new_v4()),
                label: format!("{tool:?}"),
                source: String::new(),
                bounds,
            },
            shape: Some(shape),
            id: None,
            x: (bounds.x + bounds.width / 2.0) / w.max(1.0) * 100.0,
            y: bounds.y + bounds.height / 2.0,
        });
        self.notice.clear();
        self.input.update(cx, |input, cx| input.set_text("", cx));
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn advance_drag(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((before, offset)) = &self.textbox_drag {
            let p = normalized(at, window);
            let after = super::textbox::moved_textbox(
                before,
                (
                    (p.0 - offset.0).clamp(0.0, 1.0),
                    (p.1 - offset.1).clamp(0.0, 1.0),
                ),
                viewport(window),
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
            let p = normalized(at, window);
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
        } else if let Some(offset) = self.toolbar_drag {
            let (w, h) = viewport(window);
            self.toolbar_at = Some((f32::from(at.x) - offset.0, f32::from(at.y) - offset.1));
            self.toolbar_at = Some(self.toolbar_position((w, h)));
            cx.notify();
        } else if !self.stroke.is_empty() && self.stroke.len() < 4096 {
            self.pointer = Some((at.x.into(), at.y.into()));
            let next = normalized(at, window);
            if self.stroke.last() != Some(&next) {
                self.stroke.push(next);
                cx.notify();
            }
        }
    }

    pub(super) fn history_controls(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut controls = div().flex().items_center().gap_1();
        for (name, label, icon, disabled) in [
            ("undo", "Undo", IconName::Undo2, self.history.is_empty()),
            ("redo", "Redo", IconName::Redo2, self.future.is_empty()),
            ("save", "Save", IconName::Save, self.draft.is_none()),
            (
                "delete",
                "Delete",
                IconName::Trash2,
                self.draft.as_ref().and_then(|d| d.id.as_ref()).is_none()
                    && self.selected_note.is_none(),
            ),
        ] {
            controls = controls.child(
                div()
                    .debug_selector(move || format!("feedback-history-{name}"))
                    .child(
                        super::surface_button::SurfaceButton::new(
                            format!("feedback-history-{name}"),
                            label,
                            icon,
                            false,
                        )
                        .icon_only()
                        .when(self.active, |button| {
                            button.when_some(super::commands::shortcut(name), |button, keys| {
                                button.shortcut(keys)
                            })
                        })
                        .disabled(disabled)
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.dispatch_command(
                                    &super::commands::AnnotationCommand { name: name.into() },
                                    window,
                                    cx,
                                )
                            },
                        )),
                    ),
            );
        }
        controls
    }

    fn toolbar_position(&self, viewport: (f32, f32)) -> (f32, f32) {
        toolbar_anchor(viewport, self.toolbar_size, self.toolbar_at)
    }

    fn toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let owner = cx.entity_id();
        let start_drag = cx.entity().downgrade();
        let handle = div()
            .id("feedback-drag-handle")
            .role(Role::Button)
            .aria_label("Move annotation toolbar; drag or use arrow keys")
            .tab_index(0)
            .focus_ring(cx)
            .cursor_move()
            .p_1()
            .child(Icon::new(IconName::GripVertical))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    let at = this.toolbar_position(viewport(window));
                    this.toolbar_drag = Some((
                        f32::from(event.position.x) - at.0,
                        f32::from(event.position.y) - at.1,
                    ));
                    cx.stop_propagation();
                }),
            )
            .on_drag(FeedbackDrag { owner }, move |_, _, window, cx| {
                let _ = start_drag.update(cx, |this, cx| {
                    this.advance_drag(window.mouse_position(), window, cx)
                });
                cx.new(|_| EmptyView)
            })
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "left" => (-8.0, 0.0),
                    "right" => (8.0, 0.0),
                    "up" => (0.0, -8.0),
                    "down" => (0.0, 8.0),
                    _ => return,
                };
                let at = this.toolbar_position(viewport(window));
                this.toolbar_at = Some((at.0 + delta.0, at.1 + delta.1));
                this.persist_toolbar(cx);
                cx.stop_propagation();
                cx.notify();
            }));
        let mut content = div()
            .flex()
            .flex_wrap()
            .max_w(window.viewport_size().width - px(32.0))
            .items_center()
            .gap_2()
            .flex_none()
            .child(handle);
        if self.active {
            for (index, (tool, icon, name)) in TOOLS.into_iter().enumerate() {
                content = content.child(
                    super::surface_button::SurfaceButton::new(
                        ("feedback-tool", index),
                        name,
                        icon,
                        self.tool == tool,
                    )
                    .shortcut(tool_shortcut(tool).to_ascii_lowercase())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose_tool(tool, window, cx);
                    })),
                );
            }
            if self.tool.is_some() {
                for color in 0..4 {
                    let owner = cx.entity().downgrade();
                    content = content.child(
                        ColorSwatch::new(("feedback-color", color), cx.theme().colors.chart[color])
                            .selected(color == self.color)
                            .on_click(move |_, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.color = color;
                                    cx.notify();
                                });
                            }),
                    );
                }
            }
            content = content
                .child(self.notes(window, cx))
                .child(self.settings(cx));
            let done = IconButton::new("feedback-done", IconName::Power)
                .tooltip("Done · Esc")
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
            content = content.child(round_icon(done, window, cx));
        } else {
            let add = IconButton::new("feedback-toggle", IconName::Power)
                .tooltip("Add annotation")
                .variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.draft.is_none() {
                        this.previous_focus = window.focused(cx);
                    }
                    this.active = true;
                    this.tool = None;
                    this.draft = None;
                    this.stroke.clear();
                    this.over_shape = false;
                    window.focus(&this.focus, cx);
                    cx.notify();
                }));
            content = content
                .child(round_icon(add, window, cx))
                .child(self.notes(window, cx))
                .child(self.settings(cx));
        }
        content = content.child(self.history_controls(cx));
        let length = motion::duration(motion::BASE, cx);
        // Floating ActionBar uses p_1p5 and a one-pixel border on each side.
        let padding = f32::from(gpui::rems(0.75).to_pixels(window.rem_size())) + 2.0;
        let owner = cx.entity().downgrade();
        let mut bar = ActionBar::new().floating().overflow_x_hidden().child(
            Measure::new("feedback-toolbar-content", move |bounds, _, cx| {
                let width = f32::from(bounds.size.width) + padding;
                let _ = owner.update(cx, |this, cx| {
                    if this
                        .toolbar_width
                        .as_ref()
                        .is_none_or(|motion| motion.to != width)
                    {
                        let from = this
                            .toolbar_width
                            .as_ref()
                            .map(|motion| motion.value(length).0)
                            .unwrap_or(width);
                        this.toolbar_width = Some(WidthMotion {
                            from,
                            to: width,
                            since: Instant::now(),
                        });
                        cx.notify();
                    }
                });
            })
            .flex_none()
            .child(div().id("feedback-toolbar-scroll").child(content)),
        );
        if let Some(motion) = &self.toolbar_width {
            let (width, moving) = motion.value(length);
            bar = bar.w(px(width));
            if moving {
                window.request_animation_frame();
            }
        }
        let at = self.toolbar_position(viewport(window));
        let this = cx.entity().downgrade();
        div()
            .absolute()
            .left(px(at.0))
            .top(px(at.1))
            .occlude()
            .cursor_default()
            .on_mouse_move(cx.listener(|this, _: &gpui::MouseMoveEvent, _, cx| {
                if this.pointer.take().is_some() {
                    cx.notify();
                }
            }))
            .child(
                Measure::new("feedback-toolbar-measure", move |bounds, _, cx| {
                    let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                    let _ = this.update(cx, |this, cx| {
                        if this.toolbar_size != size {
                            this.toolbar_size = size;
                            cx.notify();
                        }
                    });
                })
                .max_w(window.viewport_size().width - px(16.0))
                .child(bar),
            )
    }

    fn notes(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let count = self
            .store
            .app_annotations(&self.session.app_name)
            .map(|notes| notes.len())
            .unwrap_or(0);
        let owner = cx.entity().downgrade();
        let trigger_owner = owner.clone();
        Popover::with_opener(
            "feedback-notes",
            move |toggle| NotesTrigger {
                count,
                toggle,
                owner: trigger_owner,
            },
            move |close, window, cx| notes_content(owner, close, window, cx),
        )
    }

    fn shape_at(&self, at: gpui::Point<gpui::Pixels>, window: &Window) -> Option<String> {
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
                        shape.hit((at.x.into(), at.y.into()), viewport(window), 8.0)
                    }))
            })
            .map(|note| note.id.clone())
    }

    fn panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (w, h) = viewport(window);
        let annotation = if let Some(draft) = &self.draft {
            let mut bounds = if let Some(shape) = &draft.shape {
                shape.bounds(w, h)
            } else {
                target::targets(window, cx)
                    .into_iter()
                    .find(|t| draft.url == self.session.url && t.id == draft.target.id)
                    .map(|t| t.bounds)
                    .unwrap_or(draft.target.bounds)
            };
            let marker_height = f32::from(
                cx.theme()
                    .control_height(ControlSize::Lg)
                    .to_pixels(window.rem_size()),
            );
            if bounds.height < marker_height {
                bounds.y -= (marker_height - bounds.height) / 2.0;
                bounds.height = marker_height;
            }
            Some(bounds)
        } else {
            None
        };
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
                Measure::new("feedback-editor-measure", move |bounds, _, cx| {
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
                .bg(theme.colors.chart[self.draft.as_ref().map_or(self.color, |d| d.color)])
                .text_color(theme.colors.bg)
                .border_1()
                .border_color(theme.colors.bg.opacity(0.25))
                .rounded(theme.radius(Radius::Lg))
                .shadow(theme.elevation(Elevation::Raised))
                .child(
                    div()
                        .id("feedback-editor-scroll")
                        .max_h(px((h - 48.0).max(1.0)))
                        .overflow_y_scroll()
                        .child(content),
                ),
            )
    }

    pub(super) fn surface(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let active = self.active || self.draft.is_some();
        let (w, h) = viewport(window);
        let owner = cx.entity_id();
        let start_drag = cx.entity().downgrade();
        let mut layer = div().absolute().size_full();
        if self.active {
            let drawing = self.active && self.tool.is_some() && self.draft.is_none();
            let picker = div()
                .id("feedback-picker")
                .absolute()
                .w_full()
                .top(px(if self.banner && !self.toolbar_hidden {
                    self.banner_bottom
                } else {
                    0.
                }))
                .bottom_0()
                .occlude()
                .cursor(if self.draft.is_some() {
                    CursorStyle::Arrow
                } else {
                    tool_cursor(self.tool)
                })
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
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
                        if let Some(id) = this.shape_at(event.position, window) {
                            this.edit(&id, window, cx);
                            return;
                        }
                        if this.tool.is_some() {
                            this.stroke = vec![normalized(event.position, window)];
                            cx.notify();
                            return;
                        }
                        let size = viewport(window);
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
                .when(drawing, |pad| {
                    pad.on_drag(FeedbackDrag { owner }, move |_, _, window, cx| {
                        let _ = start_drag.update(cx, |this, cx| {
                            this.advance_drag(window.mouse_position(), window, cx)
                        });
                        cx.new(|_| EmptyView)
                    })
                });
            layer = layer.child(picker);
            if self.tool.is_none() {
                if let Some(selected) = self
                    .draft
                    .as_ref()
                    .filter(|d| d.shape.is_none())
                    .map(|d| &d.target)
                    .or(self.hovered.as_ref())
                {
                    if let Some(target) = target::targets(window, cx)
                        .iter()
                        .find(|t| t.id == selected.id)
                    {
                        let b = target.bounds;
                        layer = layer.child(
                            div()
                                .absolute()
                                .left(px(b.x))
                                .top(px(b.y))
                                .w(px(b.width))
                                .h(px(b.height))
                                .border_2()
                                .border_color(cx.theme().colors.chart[0]),
                        );
                        if self.draft.is_none() && !self.settings_open && !self.review_open {
                            let theme = cx.theme();
                            let width = (w - 16.0).clamp(1.0, 360.0);
                            let at =
                                note_panel_position(b, (width, self.inspector_height), (w, h), 8.0);
                            let owner = cx.entity().downgrade();
                            // This card deliberately has no occlusion or mouse
                            // handlers: picking continues through its surface.
                            layer = layer.child(
                                div()
                                    .absolute()
                                    .left(px(at.0))
                                    .top(px(at.1))
                                    .w(px(width))
                                    .debug_selector(|| "feedback-inspector-overlay".into())
                                    .child(
                                        Measure::new(
                                            "feedback-inspector-measure",
                                            move |bounds, _, cx| {
                                                let height = f32::from(bounds.size.height);
                                                let _ = owner.update(cx, |this, cx| {
                                                    if this.inspector_height != height {
                                                        this.inspector_height = height;
                                                        cx.notify();
                                                    }
                                                });
                                            },
                                        )
                                        .w_full()
                                        .p_3()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .overflow_hidden()
                                        .bg(theme.colors.overlay)
                                        .text_color(theme.colors.fg)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .rounded(theme.radius(Radius::Md))
                                        .shadow(theme.elevation(Elevation::Floating))
                                        .child(Overline::new("Inspect").flex_none())
                                        .child(
                                            Label::new(
                                                target
                                                    .label
                                                    .split_whitespace()
                                                    .collect::<Vec<_>>()
                                                    .join(" "),
                                            )
                                            .w_full()
                                            .truncate()
                                            .flex_none(),
                                        )
                                        .child(
                                            Caption::new(format!(
                                                "{:.0} × {:.0} px · x {:.0}, y {:.0}",
                                                b.width, b.height, b.x, b.y
                                            ))
                                            .flex_none(),
                                        )
                                        .child(inspector_source(&target.source, cx)),
                                    ),
                            );
                        }
                    }
                }
            }
        }
        if self.active && self.draft.is_none() && self.tool.is_some() {
            if let Some(at) = self.pointer {
                if let Some((_, icon, _)) = TOOLS.iter().find(|(tool, _, _)| *tool == self.tool) {
                    layer = layer.child(
                        div()
                            .absolute()
                            .left(px((at.0 + 14.0).min(w - 24.0)))
                            .top(px((at.1 + 14.0).min(h - 24.0)))
                            .text_color(cx.theme().colors.chart[self.color])
                            .child(Icon::new(*icon)),
                    );
                }
            }
        }
        let selected_id = self.draft.as_ref().and_then(|d| d.id.as_deref());
        let mut shapes: Vec<_> = self
            .session
            .annotations
            .iter()
            .filter(|a| {
                matches!(
                    a.status,
                    AnnotationStatus::Pending | AnnotationStatus::Acknowledged
                ) || Some(a.id.as_str()) == selected_id
            })
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
        let colors = cx.theme().colors.chart;
        let media = cx.theme().media();
        let stroke = f32::from(media.mark_stroke.to_pixels(window.rem_size()));
        let head = f32::from(media.arrow_head.to_pixels(window.rem_size()));
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
        for a in self.session.annotations.iter().filter(|a| {
            matches!(
                a.status,
                AnnotationStatus::Pending | AnnotationStatus::Acknowledged
            ) || Some(a.id.as_str()) == selected_id
        }) {
            if a.feedback.shape.is_none()
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
            if let Some(AnnotationShape::Text { at, text, color }) = &a.feedback.shape {
                let _ = (at, text, color);
                layer = layer.child(self.textbox(a, window, cx));
            } else {
                layer = layer.child(self.marker(&a.id, a.index, bounds, window, cx));
            }
        }
        if self.draft.is_some() {
            layer = layer.child(self.panel(window, cx));
        }
        if let Some(popup) = self.mention_popup(window, cx) {
            layer = layer.child(popup);
        }
        if !self.banner && !self.toolbar_hidden {
            layer = layer.child(self.toolbar(window, cx));
        }
        if self.settings_open || self.review_open {
            layer = layer.child(self.sheet(window, cx));
        }
        layer = layer.child(self.annotation_context_layer(cx));
        let scope = FocusScope::new(&self.focus)
            .size_full()
            .child(layer)
            .when(active && !self.banner, |scope| scope.trap());
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
                            if let Some(id) = this.shape_at(event.position, window) {
                                this.show_annotation_menu(&id, event.position, window, cx);
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
                    .when(!self.active, |root| {
                        root.when(self.over_shape, |root| root.cursor_pointer())
                            .on_mouse_move(cx.listener(
                                |this, event: &gpui::MouseMoveEvent, window, cx| {
                                    let over = this.shape_at(event.position, window).is_some();
                                    if this.over_shape != over {
                                        this.over_shape = over;
                                        cx.notify();
                                    }
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                                    if let Some(id) = this.shape_at(event.position, window) {
                                        this.edit(&id, window, cx);
                                        cx.stop_propagation();
                                    }
                                }),
                            )
                    })
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
                                this.toolbar_drag = None;
                                if this.pin_drag.take().is_some()
                                    | this.textbox_drag.take().is_some()
                                {
                                    if let Ok(session) = this.store.session(&this.session.id) {
                                        this.session = session;
                                    }
                                }
                                if this.draft.take().is_some() {
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
            .map_or(cx.theme().colors.chart[self.color], |note| {
                cx.theme().colors.chart[note.annotation_color()]
            });
        let drag_id = id.clone();
        let menu_id = id.clone();
        let drag_owner = cx.entity().downgrade();
        let owner = cx.entity_id();
        let button = number_badge(index.to_string(), ControlSize::Lg, cx)
            .size(side)
            .text_size(cx.theme().text_size(TextSize::Sm) * scale)
            .bg(amber)
            .debug_selector(move || format!("feedback-marker-number-{index}"))
            .id(format!("feedback-marker-{id}"))
            .role(Role::Button)
            .aria_label(format!("Edit comment {index}"))
            .track_focus(&focus)
            .focus_ring(cx)
            .hover(move |style| style.bg(amber.opacity(0.85)))
            .active(move |style| style.bg(amber.opacity(0.7)))
            .on_drag(FeedbackDrag { owner }, move |_, _, window, cx| {
                let _ = drag_owner.update(cx, |this, cx| {
                    if !this.dismiss_note(window, cx) {
                        return;
                    }
                    if let Ok(note) = this.store.annotation(&drag_id) {
                        if matches!(note.feedback.shape, Some(AnnotationShape::Text { .. })) {
                            this.begin_textbox_drag(&drag_id, window, cx);
                            this.advance_drag(window.mouse_position(), window, cx);
                            return;
                        }
                        let (_, h) = viewport(window);
                        let p = normalized(
                            this.annotation_press
                                .take()
                                .unwrap_or(window.mouse_position()),
                            window,
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
        let at = centered_panel(center, (side.into(), side.into()), viewport(window), 0.0);
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
        .text_color(cx.theme().colors.bg)
        .text_size(cx.theme().text_size(crate::ely::theme::TextSize::Sm))
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

#[cfg(test)]
#[path = "surface.test.rs"]
mod tests;
