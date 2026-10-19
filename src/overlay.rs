use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    forms::{Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ControlSize, IconSize},
};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, Render, Subscription, Task, Window, div,
    prelude::*,
};

use crate::{
    AnnotationInput, AnnotationPatch, AnnotationStatus, Error, Server, Session, Store,
    target::{self, Target},
};

struct Draft {
    index: Option<u64>,
    url: String,
    shape: Option<crate::AnnotationShape>,
    target: Target,
    id: Option<String>,
    x: f32,
    y: f32,
}

/// Native annotation toolbar, picker, comment editor, and feedback review panel.
pub struct AnnotationOverlay {
    store: Store,
    _server: Option<Server>,
    session: Session,
    selected_note: Option<String>,
    active: bool,
    hovered: Option<Target>,
    draft: Option<Draft>,
    input: Entity<TextInput>,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    notice: String,
    revision: u64,
    tool: Option<ely_gpui_component::media::Tool>,
    color: usize,
    stroke: Vec<(f32, f32)>,
    toolbar_at: Option<(f32, f32)>,
    toolbar_size: (f32, f32),
    toolbar_width: Option<surface::WidthMotion>,
    pointer: Option<(f32, f32)>,
    over_shape: bool,
    toolbar_drag: Option<(f32, f32)>,
    editor_size: (f32, f32),
    _refresh: Task<()>,
    _inputs: Vec<Subscription>,
}

impl AnnotationOverlay {
    /// Mount feedback using a dedicated file and local port. A busy port leaves local notes usable.
    pub fn mount(
        app_name: &str,
        url: &str,
        path: impl Into<PathBuf>,
        port: u16,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<Self>, Error> {
        let store = Store::open(path)?;
        let session = store.ensure_session(app_name, url)?;
        Ok(cx.new(|cx| Self::new(store, session, port, window, cx)))
    }

    fn new(
        store: Store,
        session: Session,
        port: u16,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (server, notice) = match Server::start(store.clone(), port) {
            Ok(server) => (Some(server), String::new()),
            Err(error) => (
                None,
                format!("API offline: {error}. Local notes still work."),
            ),
        };
        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line(3, 6)
                .max_len(8000)
                .label("Feedback")
                .placeholder("Describe what should change…")
        });
        let inputs = vec![cx.observe(&input, |_, _, cx| cx.notify())];
        let watch = store.clone();
        let refresh = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(300))
                    .await;
                let Ok(revision) = watch.revision() else {
                    break;
                };
                if this
                    .update(cx, |this, cx| {
                        if revision != this.revision {
                            this.revision = revision;
                            match this.store.session(&this.session.id) {
                                Ok(session) => this.session = session,
                                Err(error) => this.notice = error.to_string(),
                            }
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            store,
            _server: server,
            session,
            selected_note: None,
            active: false,
            hovered: None,
            draft: None,
            input,
            focus: cx.focus_handle(),
            previous_focus: None,
            notice,
            revision: 0,
            tool: None,
            color: 0,
            stroke: vec![],
            toolbar_at: None,
            toolbar_size: (0.0, 0.0),
            toolbar_width: None,
            pointer: None,
            over_shape: false,
            toolbar_drag: None,
            editor_size: (384.0, 240.0),
            _refresh: refresh,
            _inputs: inputs,
        }
    }

    /// Switch route buckets. Saved notes remain; the unfinished editor is cancelled.
    pub fn set_route(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.session.url == url {
            return;
        }
        match self.store.ensure_session(&self.session.app_name, url) {
            Ok(session) => {
                self.session = session;
                self.draft = None;
                self.return_focus(window, cx);
                self.stroke.clear();
                self.hovered = None;
                self.input.update(cx, |input, cx| input.set_text("", cx));
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    /// Toggle picking mode, returning keyboard focus when leaving it.
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active || self.draft.is_some() {
            self.close(window, cx);
        } else {
            self.previous_focus = window.focused(cx);
            self.active = true;
            self.tool = None;
            self.pointer = None;
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active = false;
        self.draft = None;
        self.stroke.clear();
        self.hovered = None;
        self.pointer = None;
        self.over_shape = false;
        if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn return_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active {
            window.focus(&self.focus, cx);
        } else if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus, cx);
        }
    }

    fn select(
        &mut self,
        target: Target,
        x: f32,
        y: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.draft = Some(Draft {
            index: None,
            url: self.session.url.clone(),
            shape: None,
            target,
            id: None,
            x,
            y,
        });
        self.notice.clear();
        self.input.update(cx, |input, cx| input.set_text("", cx));
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    fn edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.active && self.draft.is_none() {
            self.previous_focus = window.focused(cx);
        }
        match self.store.annotation(id) {
            Ok(a) => {
                self.draft = Some(Draft {
                    index: Some(a.index),
                    url: a.url,
                    shape: a.feedback.shape,
                    target: Target {
                        id: a.feedback.element_path,
                        label: a.feedback.element,
                        source: a.feedback.source_file.unwrap_or_default(),
                        bounds: a.feedback.bounding_box,
                    },
                    id: Some(a.id),
                    x: a.feedback.x,
                    y: a.feedback.y,
                });
                self.input
                    .update(cx, |input, cx| input.set_text(a.feedback.comment, cx));
                self.notice.clear();
                window.focus(&self.input.focus_handle(cx), cx);
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = &self.draft else {
            return;
        };
        let comment = self.input.read(cx).text().trim().to_string();
        let result = match &draft.id {
            Some(id) => self.store.update(
                id,
                AnnotationPatch {
                    comment: Some(comment),
                    status: None,
                },
            ),
            None => self.store.add(
                &self.session.id,
                AnnotationInput {
                    comment,
                    element_path: draft.target.id.clone(),
                    element: draft.target.label.clone(),
                    source_file: Some(draft.target.source.clone()).filter(|s| !s.is_empty()),
                    bounding_box: draft.target.bounds,
                    x: draft.x,
                    y: draft.y,
                    shape: draft.shape.clone(),
                },
            ),
        };
        match result {
            Ok(note) => {
                if note.session_id == self.session.id {
                    if let Some(existing) = self
                        .session
                        .annotations
                        .iter_mut()
                        .find(|a| a.id == note.id)
                    {
                        *existing = note;
                    } else {
                        self.session.annotations.push(note);
                    }
                }
                self.draft = None;
                self.return_focus(window, cx);
                self.notice.clear();
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    fn delete(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Err(error) = self.store.delete(id) {
            self.notice = error.to_string();
            cx.notify();
            return false;
        }
        self.session.annotations.retain(|note| note.id != id);
        if self.selected_note.as_deref() == Some(id) {
            self.selected_note = None;
        }
        if self.draft.as_ref().and_then(|draft| draft.id.as_deref()) == Some(id) {
            self.draft = None;
            self.return_focus(window, cx);
        }
        self.notice.clear();
        cx.notify();
        true
    }

    fn editor(&self, window: &mut Window, cx: &mut Context<Self>) -> gpui::Div {
        let Some(draft) = &self.draft else {
            return div();
        };
        let index = draft
            .index
            .unwrap_or_else(|| self.store.next_index(&self.session.id).unwrap_or(1));
        let confirm = IconButton::new("feedback-save", IconName::Check)
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Ghost)
            .tooltip(if draft.id.is_some() {
                "Update note"
            } else {
                "Save note"
            })
            .disabled(self.input.read(cx).text().trim().is_empty())
            .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)));
        let delete = IconButton::new("feedback-delete", IconName::Trash2)
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Ghost)
            .tooltip(if draft.id.is_some() {
                "Delete note"
            } else {
                "Discard note"
            })
            .on_click(cx.listener(|this, _, window, cx| {
                if let Some(id) = this.draft.as_ref().and_then(|draft| draft.id.clone()) {
                    if !this.delete(&id, window, cx) {
                        return;
                    }
                }
                this.draft = None;
                this.return_focus(window, cx);
                this.notice.clear();
                cx.notify();
            }));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(surface::annotation_icon(draft.shape.as_ref()))
                            .size(IconSize::Sm),
                    )
                    .child(div().flex_1().child(format!("Note {index}")))
                    .child(delete)
                    .child(confirm),
            )
            .child(surface::comment_input(&self.input, window, cx))
    }
}

mod surface;

impl Render for AnnotationOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.surface(window, cx)
    }
}
