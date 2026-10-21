use super::*;
use crate::ely::{
    buttons::{ActionBar, Button, IconButton},
    data_display::CountBadge,
    forms::ColorSwatch,
    lists::{ListItem, SelectableList},
    media::Tool,
    motion,
    overlays::Popover,
    primitives::{FocusRing, FocusScope, Icon, IconName, Measure, raise},
    theme::{ActiveTheme, ControlSize, Elevation, Radius},
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
struct FeedbackDrag {
    owner: EntityId,
}

const TOOLS: [(Option<Tool>, IconName, &str); 4] = [
    (None, IconName::Crosshair, "Inspect element"),
    (Some(Tool::Box), IconName::Square, "Box"),
    (Some(Tool::Arrow), IconName::ArrowUpRight, "Arrow"),
    (Some(Tool::Pen), IconName::PenLine, "Pen"),
];

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
        Some(Tool::Arrow) => CursorStyle::Arrow,
        _ => CursorStyle::Crosshair,
    }
}

#[derive(IntoElement)]
struct NotesTrigger {
    count: usize,
    toggle: std::rc::Rc<dyn Fn(&mut Window, &mut App)>,
    owner: gpui::WeakEntity<AnnotationOverlay>,
}

impl RenderOnce for NotesTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let notes = IconButton::new("feedback-review", IconName::MapPin)
            .tooltip(format!("Notes ({})", self.count))
            .variant(ButtonVariant::Ghost)
            .on_click(move |_, window, cx| {
                let _ = self.owner.update(cx, |this, cx| {
                    if this.draft.take().is_some() {
                        this.return_focus(window, cx);
                        cx.notify();
                    }
                });
                (self.toggle)(window, cx);
            });
        CountBadge::new("feedback-notes-count", self.count).over(round_icon(notes, window, cx))
    }
}

pub(super) fn annotation_icon(shape: Option<&AnnotationShape>) -> IconName {
    match shape {
        None => IconName::Crosshair,
        Some(AnnotationShape::Box { .. }) => IconName::Square,
        Some(AnnotationShape::Arrow { .. }) => IconName::ArrowUpRight,
        Some(AnnotationShape::Pen { .. }) => IconName::PenLine,
        Some(AnnotationShape::Pin { .. }) => IconName::MapPin,
    }
}

pub(super) fn comment_input(
    input: &gpui::Entity<TextInput>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let focused = input.focus_handle(cx).is_focused(window);
    let mut field = Input::new(input).render(window, cx).into_any_element();
    let colors = &cx.theme().colors;
    if let Some(root) = field.downcast_mut::<gpui::Div>() {
        root.style().refine(
            &gpui::StyleRefinement::default()
                .bg(colors.sunken)
                .border_color(if focused {
                    colors.border_strong
                } else {
                    colors.border
                }),
        );
    }
    field
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

impl AnnotationOverlay {
    fn annotation_bounds(&self, feedback: &AnnotationInput, window: &Window, cx: &App) -> Rect {
        match &feedback.shape {
            Some(shape) => {
                let (w, h) = viewport(window);
                shape.bounds(w, h)
            }
            None => target::targets(window, cx)
                .into_iter()
                .find(|t| t.id == feedback.element_path)
                .map(|t| t.bounds)
                .unwrap_or(feedback.bounding_box),
        }
    }

    fn finish_stroke(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toolbar_drag = None;
        let points = std::mem::take(&mut self.stroke);
        let Some(tool) = self.tool else {
            return;
        };
        let Some(shape) =
            crate::drawing::finish_shape(tool, points, normalized(at, window), self.color)
        else {
            return;
        };
        let (w, h) = viewport(window);
        let bounds = shape.bounds(w, h);
        self.draft = Some(Draft {
            index: None,
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

    fn advance_drag(
        &mut self,
        at: gpui::Point<gpui::Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(offset) = self.toolbar_drag {
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

    fn toolbar_position(&self, viewport: (f32, f32)) -> (f32, f32) {
        let size = self.toolbar_size;
        let at = self
            .toolbar_at
            .unwrap_or(((viewport.0 - size.0) / 2.0, viewport.1 - size.1 - 48.0));
        (
            at.0.clamp(8.0, (viewport.0 - size.0 - 8.0).max(8.0)),
            at.1.clamp(8.0, (viewport.1 - size.1 - 8.0).max(8.0)),
        )
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
                cx.stop_propagation();
                cx.notify();
            }));
        let mut content = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_none()
            .child(handle);
        if self.active {
            for (index, (tool, icon, name)) in TOOLS.into_iter().enumerate() {
                content = content.child(
                    IconButton::new(("feedback-tool", index), icon)
                        .tooltip(name)
                        .variant(if self.tool == tool {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.tool = tool;
                            this.draft = None;
                            this.stroke.clear();
                            window.focus(&this.focus, cx);
                            cx.notify();
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
            let done = IconButton::new("feedback-done", IconName::X)
                .tooltip("Done · Esc")
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
            content = content.child(round_icon(done, window, cx));
        } else {
            let add = IconButton::new("feedback-toggle", IconName::Plus)
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
                .child(self.notes(window, cx));
        }
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
            .child(content),
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
            move |close, window, cx| {
                let Some(owner) = owner.upgrade() else {
                    return div();
                };
                let this = owner.read(cx);
                let notes = this.store.app_annotations(&this.session.app_name);
                let mut rows = SelectableList::new("feedback-notes-list")
                    .label("All app annotations")
                    .selected(this.selected_note.iter().cloned())
                    .max_h(window.viewport_size().height * 0.5);
                let mut message = None;
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
                            let edit =
                                IconButton::new(format!("feedback-edit-{id}"), IconName::Pencil)
                                    .size(ControlSize::Sm)
                                    .variant(ButtonVariant::Ghost)
                                    .tooltip(format!("Edit note {}", note.index))
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        edit_close(window, cx);
                                        edit_owner
                                            .update(cx, |this, cx| this.edit(&edit_id, window, cx));
                                    });
                            let delete_owner = owner.clone();
                            let delete_id = id.clone();
                            let delete =
                                IconButton::new(format!("feedback-delete-{id}"), IconName::Trash2)
                                    .size(ControlSize::Sm)
                                    .variant(ButtonVariant::Ghost)
                                    .tooltip(format!("Delete note {}", note.index))
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        delete_owner.update(cx, |this, cx| {
                                            this.delete(&delete_id, window, cx);
                                        });
                                    });
                            rows = rows.row(
                                id.clone(),
                                ListItem::new(
                                    format!("feedback-note-{id}"),
                                    format!("{}. {}", note.index, note.feedback.comment),
                                )
                                .description(note.url.clone())
                                .leading(Icon::new(annotation_icon(note.feedback.shape.as_ref())))
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
                rows = rows
                    .on_change(move |keys, _, cx| {
                        select_owner.update(cx, |this, cx| {
                            this.selected_note = keys.first().map(|key| key.to_string());
                            cx.notify();
                        });
                    })
                    .on_activate(move |id, window, cx| {
                        close(window, cx);
                        activate_owner.update(cx, |this, cx| this.edit(id, window, cx));
                    });
                let owner = owner.clone();
                div()
                    .w(gpui::rems(22.0))
                    .max_w(window.viewport_size().width - px(32.0))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child("All annotations")
                    .when_some(message, |panel, message| panel.child(message))
                    .child(rows)
                    .child(
                        Button::new("feedback-copy-markdown", "Copy Markdown for agent")
                            .icon(IconName::Copy)
                            .on_click(move |_, _, cx| {
                                owner.update(cx, |this, cx| {
                                    match this.store.app_markdown(&this.session.app_name) {
                                        Ok(markdown) => {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                markdown,
                                            ));
                                            this.notice = "Copied Markdown for agent".into();
                                        }
                                        Err(error) => this.notice = error.to_string(),
                                    }
                                    cx.notify();
                                });
                            }),
                    )
                    .when(!this.notice.is_empty(), |panel| {
                        panel.child(this.notice.clone())
                    })
            },
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
                ) && note.feedback.shape.as_ref().is_some_and(|shape| {
                    shape.hit((at.x.into(), at.y.into()), viewport(window), 8.0)
                })
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
            f32::from(gpui::rems(24.0).to_pixels(window.rem_size())).min((w - 16.0).max(1.0));
        let panel_size = (width, self.editor_size.1.min((h - 16.0).max(1.0)));
        let at = match annotation {
            Some(bounds) => note_panel_position(bounds, panel_size, (w, h), 8.0),
            None => centered_panel((w / 2.0, h / 2.0), panel_size, (w, h), 8.0),
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
            .absolute()
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
                .bg(theme.colors.surface)
                .text_color(theme.colors.fg)
                .border_1()
                .border_color(theme.colors.border)
                .rounded(theme.radius(Radius::Lg))
                .shadow(theme.elevation(Elevation::Floating))
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
                .size_full()
                .occlude()
                .cursor(if self.draft.is_some() {
                    CursorStyle::Arrow
                } else {
                    tool_cursor(self.tool)
                })
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
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
                        if this.draft.is_some() {
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
            let bounds = self.annotation_bounds(&a.feedback, window, cx);
            layer = layer.child(self.marker(&a.id, a.index, bounds, window, cx));
        }
        if self.draft.is_some() {
            layer = layer.child(self.panel(window, cx));
        }
        layer = layer.child(self.toolbar(window, cx));
        let scope = FocusScope::new(&self.focus)
            .size_full()
            .child(layer)
            .when(active, |scope| scope.trap());
        raise(
            "gpui-agentation-overlay",
            anchored().position(point(px(0.0), px(0.0))).child(
                div()
                    .id("feedback-surface")
                    .relative()
                    .w(px(w))
                    .h(px(h))
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
                            if event.keystroke.key == "escape"
                                && (this.active || this.draft.is_some())
                            {
                                this.stroke.clear();
                                this.toolbar_drag = None;
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

    fn marker(
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
        let side = cx
            .theme()
            .control_height(ControlSize::Lg)
            .to_pixels(window.rem_size());
        let mut button = Button::new(format!("feedback-marker-{id}"), index.to_string())
            .size(ControlSize::Lg)
            .primary()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.edit(&id, window, cx);
            }))
            .render(window, cx)
            .into_any_element();
        if let Some(root) = button.downcast_mut::<gpui::Stateful<gpui::Div>>() {
            root.style().refine(
                &gpui::StyleRefinement::default()
                    .w(side)
                    .h(side)
                    .px_0()
                    .rounded_full(),
            );
        }
        let at = centered_panel(center, (side.into(), side.into()), viewport(window), 0.0);
        div()
            .absolute()
            .left(px(at.0))
            .top(px(at.1))
            .occlude()
            .child(button)
    }
}

#[cfg(all(test, feature = "test-support"))]
#[path = "surface.test.rs"]
mod tests;
