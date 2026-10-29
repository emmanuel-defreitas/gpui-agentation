//! Right-click context menus. One model (`Row`) drives both the native macOS
//! `NSMenu` popup and the in-app Ely fallback used off macOS and in tests, so
//! every menu — pin, shape, element, and tools — looks like a macOS menu.
use super::commands::AnnotationCommand;
use super::surface::viewport;
use super::toolbar_window::TOOLS;
use super::*;
use crate::ely::menus::{ContextMenu, Menu, MenuItem};
use gpui::{AnyElement, Pixels, Point, SharedString};

/// A per-note menu command. Pins offer edit/copy; shapes offer color/duplicate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NoteAction {
    Edit,
    Delete,
    Copy,
    Duplicate,
    Comment,
    Color(usize),
}

/// What a right click landed on, so the fallback layer can rebuild its menu.
#[derive(Clone)]
pub(super) enum ContextKind {
    /// An existing note, classified as a pin or a shape by its stored shape.
    Note(String),
    /// A registered host element picked under the cursor.
    Element(Target),
    /// Empty canvas: the full tool list.
    Tools,
}

pub(super) struct SubItem {
    pub label: SharedString,
    pub tag: i64,
    pub checked: bool,
    pub enabled: bool,
}

/// One menu row in the shared model.
pub(super) enum Row {
    Item {
        label: SharedString,
        tag: i64,
        checked: bool,
        enabled: bool,
    },
    Submenu {
        label: SharedString,
        items: Vec<SubItem>,
    },
    Separator,
}

fn item(label: impl Into<SharedString>, tag: i64) -> Row {
    Row::Item {
        label: label.into(),
        tag,
        checked: false,
        enabled: true,
    }
}

fn disabled(label: impl Into<SharedString>, tag: i64, enabled: bool) -> Row {
    Row::Item {
        label: label.into(),
        tag,
        checked: false,
        enabled,
    }
}

fn color_submenu(selected: usize, tag_of: impl Fn(usize) -> i64) -> Row {
    Row::Submenu {
        label: "Color".into(),
        items: ["Blue", "Green", "Orange", "Red"]
            .into_iter()
            .enumerate()
            .map(|(color, name)| SubItem {
                label: name.into(),
                tag: tag_of(color),
                checked: selected == color,
                enabled: true,
            })
            .collect(),
    }
}

// Note-menu tags.
const EDIT: i64 = 1;
const DELETE: i64 = 2;
const COPY: i64 = 3;
const DUPLICATE: i64 = 4;
const COMMENT: i64 = 5;
const COLOR: i64 = 10; // Color(i) == COLOR + i

fn note_action(tag: i64) -> Option<NoteAction> {
    Some(match tag {
        EDIT => NoteAction::Edit,
        DELETE => NoteAction::Delete,
        COPY => NoteAction::Copy,
        DUPLICATE => NoteAction::Duplicate,
        COMMENT => NoteAction::Comment,
        c if (COLOR..COLOR + 4).contains(&c) => NoteAction::Color((c - COLOR) as usize),
        _ => return None,
    })
}

// Element-menu tags: a comment on the element, then the drawing tools.
const EL_COMMENT: i64 = 1;
const EL_BOX: i64 = 2;
const EL_ARROW: i64 = 3;
const EL_PEN: i64 = 4;
const EL_TEXT: i64 = 5;

fn element_command(tag: i64) -> Option<&'static str> {
    Some(match tag {
        EL_BOX => "box",
        EL_ARROW => "arrow",
        EL_PEN => "pen",
        EL_TEXT => "text",
        _ => return None,
    })
}

// Tools-menu tags cover every toolbar command.
fn tool_command(tag: i64) -> Option<&'static str> {
    Some(match tag {
        1 => "inspect",
        2 => "box",
        3 => "arrow",
        4 => "pen",
        5 => "comment",
        6 => "text",
        10 => "undo",
        11 => "redo",
        12 => "save",
        13 => "delete",
        14 => "comments",
        c if (20..24).contains(&c) => return TOOL_COLORS.get((c - 20) as usize).copied(),
        _ => return None,
    })
}

const TOOL_COLORS: [&str; 4] = ["color-0", "color-1", "color-2", "color-3"];

impl AnnotationOverlay {
    pub(super) fn show_annotation_menu(
        &mut self,
        id: &str,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.store.annotation(id).is_err() {
            return;
        }
        self.selected_note = Some(id.into());
        self.open_context(ContextKind::Note(id.to_owned()), at, window, cx);
    }

    pub(super) fn show_element_menu(
        &mut self,
        target: Target,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_context(ContextKind::Element(target), at, window, cx);
    }

    pub(super) fn show_tools_menu(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_note = None;
        self.open_context(ContextKind::Tools, at, window, cx);
    }

    /// Right click on empty canvas: the element under the cursor, if any, gets
    /// the element menu (Comment adds to it); otherwise the tool list.
    pub(super) fn open_canvas_menu(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match crate::target::pick(window, cx, f32::from(at.x), f32::from(at.y)) {
            Some(target) => self.show_element_menu(target, at, window, cx),
            None => self.show_tools_menu(at, window, cx),
        }
    }

    fn open_context(
        &mut self,
        kind: ContextKind,
        at: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(all(target_os = "macos", not(test)))]
        {
            let rows = self.rows_for(&kind, cx);
            if let Ok(popup) = super::native_menu::Popup::new(_window, at, &rows) {
                let owner = cx.entity().downgrade();
                // Poll outside App's mutable update: NSMenu runs a nested loop.
                _window
                    .spawn(cx, async move |cx| {
                        if let Some(tag) = popup.open() {
                            let _ = cx.update(|window, cx| {
                                let _ = owner.update(cx, |this, cx| {
                                    this.apply_context(&kind, tag, window, cx)
                                });
                            });
                        }
                    })
                    .detach();
                cx.notify();
                return;
            }
        }
        self.context = Some(kind);
        self.context_number += 1;
        self.context_request = Some((self.context_number, at));
        cx.notify();
    }

    fn apply_context(
        &mut self,
        kind: &ContextKind,
        tag: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match kind {
            ContextKind::Note(id) => {
                if let Some(action) = note_action(tag) {
                    self.note_action(id, action, window, cx);
                }
            }
            ContextKind::Element(target) => self.element_action(target.clone(), tag, window, cx),
            ContextKind::Tools => {
                if let Some(command) = tool_command(tag) {
                    self.dispatch_command(
                        &AnnotationCommand {
                            name: command.into(),
                        },
                        window,
                        cx,
                    );
                }
            }
        }
    }

    fn rows_for(&self, kind: &ContextKind, cx: &App) -> Vec<Row> {
        match kind {
            ContextKind::Note(id) => self.note_rows(id),
            ContextKind::Element(_) => element_rows(),
            ContextKind::Tools => self.tool_rows(cx),
        }
    }

    fn note_rows(&self, id: &str) -> Vec<Row> {
        let note = self.store.annotation(id).ok();
        let shape = note.as_ref().and_then(|n| n.feedback.shape.clone());
        let selected = note
            .as_ref()
            .map_or(self.color, crate::Annotation::annotation_color);
        let is_shape = matches!(
            shape,
            Some(
                crate::AnnotationShape::Box { .. }
                    | crate::AnnotationShape::Arrow { .. }
                    | crate::AnnotationShape::Pen { .. }
                    | crate::AnnotationShape::Text { .. }
            )
        );
        if is_shape {
            vec![
                color_submenu(selected, |c| COLOR + c as i64),
                Row::Separator,
                item("Delete", DELETE),
                item("Duplicate", DUPLICATE),
                item("Comment…", COMMENT),
            ]
        } else {
            vec![
                item("Edit…", EDIT),
                item("Delete", DELETE),
                item("Copy", COPY),
            ]
        }
    }

    fn tool_rows(&self, cx: &App) -> Vec<Row> {
        let selected = self.active_tool_command();
        let mut rows = Vec::new();
        for (index, (_, _, label, command)) in TOOLS.iter().enumerate() {
            let tag = index as i64 + 1;
            let checked = *command == selected;
            rows.push(Row::Item {
                label: (*label).into(),
                tag,
                checked,
                enabled: true,
            });
            if *command == "inspect" {
                rows.push(Row::Separator);
            }
        }
        rows.push(Row::Separator);
        rows.push(color_submenu(self.color, |c| 20 + c as i64));
        rows.push(Row::Separator);
        rows.push(disabled("Undo", 10, self.command_enabled("undo", cx)));
        rows.push(disabled("Redo", 11, self.command_enabled("redo", cx)));
        rows.push(disabled(
            "Save feedback",
            12,
            self.command_enabled("save", cx),
        ));
        rows.push(disabled(
            "Delete annotation",
            13,
            self.command_enabled("delete", cx),
        ));
        rows.push(Row::Separator);
        rows.push(item("Comments…", 14));
        rows
    }

    /// The command name of the active tool, so its menu row shows as checked.
    fn active_tool_command(&self) -> &'static str {
        if self.inspecting {
            return "inspect";
        }
        TOOLS
            .iter()
            .find(|(tool, _, _, _)| tool.is_some() && *tool == self.tool)
            .map(|(_, _, _, command)| *command)
            .unwrap_or("inspect")
    }

    fn element_action(
        &mut self,
        target: Target,
        tag: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if tag == EL_COMMENT {
            // Mirror Inspect: outline the element, pin its center, open the editor.
            let size = viewport(window);
            let center = (
                target.bounds.x + target.bounds.width / 2.0,
                target.bounds.y + target.bounds.height / 2.0,
            );
            self.select(
                target,
                center.0 / size.0.max(1.0) * 100.0,
                center.1,
                window,
                cx,
            );
        } else if let Some(command) = element_command(tag) {
            self.dispatch_command(
                &AnnotationCommand {
                    name: command.into(),
                },
                window,
                cx,
            );
        }
    }

    pub(super) fn note_action(
        &mut self,
        id: &str,
        action: NoteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            NoteAction::Edit | NoteAction::Comment => self.edit(id, window, cx),
            NoteAction::Delete => {
                self.delete(id, window, cx);
            }
            NoteAction::Copy => match self.store.note_markdown(id) {
                Ok(text) => cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)),
                Err(error) => self.notice = error.to_string(),
            },
            NoteAction::Duplicate => self.duplicate(id),
            NoteAction::Color(color) if color < 4 => self.recolor(id, color),
            NoteAction::Color(_) => {}
        }
        cx.notify();
    }

    fn recolor(&mut self, id: &str, color: usize) {
        match self.store.annotation(id) {
            Ok(before) => {
                let mut after = before.clone();
                after.color = Some(color);
                if let Some(shape) = &mut after.feedback.shape {
                    match shape {
                        crate::AnnotationShape::Box { color: c, .. }
                        | crate::AnnotationShape::Arrow { color: c, .. }
                        | crate::AnnotationShape::Pen { color: c, .. }
                        | crate::AnnotationShape::Pin { color: c, .. }
                        | crate::AnnotationShape::Text { color: c, .. } => *c = color,
                    }
                }
                match self.store.replace_note(Some(&before), Some(&after)) {
                    Ok(()) => {
                        self.record_change(Some(before), Some(after.clone()));
                        if let Some(note) = self
                            .session
                            .annotations
                            .iter_mut()
                            .find(|note| note.id == id)
                        {
                            *note = after.clone();
                        }
                        if let Some(draft) =
                            self.draft.as_mut().filter(|d| d.id.as_deref() == Some(id))
                        {
                            draft.color = color;
                            draft.shape = after.feedback.shape;
                        }
                        self.notice.clear();
                    }
                    Err(error) => self.notice = error.to_string(),
                }
            }
            Err(error) => self.notice = error.to_string(),
        }
    }

    fn duplicate(&mut self, id: &str) {
        let Ok(original) = self.store.annotation(id) else {
            return;
        };
        let mut input = AnnotationInput {
            inspection: None,
            comment: original.feedback.comment.clone(),
            element_path: original.feedback.element_path.clone(),
            element: original.feedback.element.clone(),
            source_file: original.feedback.source_file.clone(),
            gpui_configuration: None,
            bounding_box: original.feedback.bounding_box,
            x: original.feedback.x,
            y: original.feedback.y,
            shape: original.feedback.shape.clone(),
        };
        // Nudge the copy so it does not land exactly on top of the original.
        const NUDGE: f32 = 0.03;
        if let Some(shape) = &mut input.shape {
            shape.nudge(NUDGE);
        }
        input.x += NUDGE * 100.0;
        input.y += 24.0;
        match self
            .store
            .add_colored(&self.session.id, input, original.color)
        {
            Ok(note) => {
                self.record_change(None, Some(note.clone()));
                if note.session_id == self.session.id {
                    self.session.annotations.push(note);
                }
                self.notice.clear();
            }
            Err(error) => self.notice = error.to_string(),
        }
    }

    /// The right-click menu rendered in-app off macOS and in tests.
    pub(super) fn context_layer(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(kind) = self.context.clone() else {
            return div().into_any_element();
        };
        let rows = self.rows_for(&kind, cx);
        let owner = cx.entity().downgrade();
        let make = move |tag: i64| {
            let kind = kind.clone();
            let owner = owner.clone();
            move |window: &mut Window, cx: &mut App| {
                let _ = owner.update(cx, |this, cx| this.apply_context(&kind, tag, window, cx));
            }
        };
        ContextMenu::new("context-menu", ely_menu(&rows, make))
            .manual(self.context_request)
            .into_any_element()
    }
}

fn ely_menu<H>(rows: &[Row], make: impl Fn(i64) -> H) -> Menu
where
    H: Fn(&mut Window, &mut App) + 'static,
{
    let mut menu = Menu::new();
    for row in rows {
        menu = match row {
            Row::Separator => menu.separator(),
            Row::Item {
                label,
                tag,
                checked,
                enabled,
            } => {
                let row = if *checked {
                    MenuItem::check(label.clone(), true)
                } else {
                    MenuItem::new(label.clone())
                };
                menu.item(row.disabled(!*enabled).on_click(make(*tag)))
            }
            Row::Submenu { label, items } => {
                let mut submenu = Menu::new();
                for sub in items {
                    submenu = submenu.item(
                        MenuItem::radio(sub.label.clone(), sub.checked)
                            .disabled(!sub.enabled)
                            .on_click(make(sub.tag)),
                    );
                }
                menu.item(MenuItem::submenu(label.clone(), submenu))
            }
        };
    }
    menu
}

fn element_rows() -> Vec<Row> {
    vec![
        item("Comment", EL_COMMENT),
        Row::Separator,
        item("Box", EL_BOX),
        item("Arrow", EL_ARROW),
        item("Pen", EL_PEN),
        item("Textbox", EL_TEXT),
    ]
}
