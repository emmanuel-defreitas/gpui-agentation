use super::*;
use crate::ely::menus::{ContextMenu, Menu, MenuItem};
use gpui::{AnyElement, Pixels, Point};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum NoteAction {
    Edit,
    Delete,
    Color(usize),
}

#[cfg(all(target_os = "macos", not(test)))]
#[allow(deprecated, unexpected_cfgs)]
mod mac;

impl AnnotationOverlay {
    pub(super) fn show_annotation_menu(
        &mut self,
        id: &str,
        at: Point<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(note) = self.store.annotation(id) else {
            return;
        };
        self.selected_note = Some(id.into());
        #[cfg(all(target_os = "macos", not(test)))]
        if let Ok(popup) = mac::Popup::new(_window, at, note.annotation_color()) {
            let owner = cx.entity().downgrade();
            let id = id.to_owned();
            // Poll outside App's mutable update: NSMenu runs a nested native loop.
            _window
                .spawn(cx, async move |cx| {
                    if let Some(action) = popup.open() {
                        let _ = cx.update(|window, cx| {
                            let _ = owner
                                .update(cx, |this, cx| this.note_action(&id, action, window, cx));
                        });
                    }
                })
                .detach();
            cx.notify();
            return;
        }
        self.context_note = Some(note.id);
        self.context_number += 1;
        self.context_request = Some((self.context_number, at));
        cx.notify();
    }

    pub(super) fn note_action(
        &mut self,
        id: &str,
        action: NoteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            NoteAction::Edit => self.edit(id, window, cx),
            NoteAction::Delete => {
                self.delete(id, window, cx);
            }
            NoteAction::Color(color) if color < 4 => match self.store.annotation(id) {
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
            },
            _ => {}
        }
        cx.notify();
    }

    pub(super) fn annotation_context_layer(&self, cx: &mut Context<Self>) -> AnyElement {
        let id = self.context_note.clone().unwrap_or_default();
        let owner = cx.entity().downgrade();
        let run = move |action| {
            let id = id.clone();
            let owner = owner.clone();
            move |window: &mut Window, cx: &mut App| {
                let _ = owner.update(cx, |this, cx| this.note_action(&id, action, window, cx));
            }
        };
        let selected = self
            .context_note
            .as_ref()
            .and_then(|id| self.session.annotations.iter().find(|n| &n.id == id))
            .map(|n| n.annotation_color());
        let mut colors = Menu::new();
        for (color, name) in ["Blue", "Green", "Orange", "Red"].into_iter().enumerate() {
            colors = colors.item(
                MenuItem::radio(name, selected == Some(color))
                    .on_click(run(NoteAction::Color(color))),
            );
        }
        let menu = Menu::new()
            .item(MenuItem::new("Edit…").on_click(run(NoteAction::Edit)))
            .item(
                MenuItem::new("Delete")
                    .keys("delete")
                    .on_click(run(NoteAction::Delete)),
            )
            .separator()
            .item(MenuItem::submenu("Color", colors));
        ContextMenu::new("annotation-context-menu", menu)
            .manual(self.context_request)
            .into_any_element()
    }
}
