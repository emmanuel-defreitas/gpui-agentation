use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::ely::{
    buttons::{Button, ButtonVariant},
    forms::{Input, TextInput},
    primitives::Icon,
    theme::{ActiveTheme, ControlSize, IconSize},
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
    editing: bool,
    original_comment: String,
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
    api_port: u16,
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
    tool: Option<crate::ely::media::Tool>,
    color: usize,
    stroke: Vec<(f32, f32)>,
    toolbar_at: Option<(f32, f32)>,
    toolbar_size: (f32, f32),
    toolbar_width: Option<surface::WidthMotion>,
    banner: bool,
    banner_bottom: f32,
    settings_open: bool,
    review_open: bool,
    sheet_focus: FocusHandle,
    sheet_previous_focus: Option<FocusHandle>,
    settings_status: String,
    mcp_starting: bool,
    mcp_requested: bool,
    mcp_config_input: Entity<TextInput>,
    mcp_config_copied: bool,
    _copy_feedback: Option<Task<()>>,
    pointer: Option<(f32, f32)>,
    over_shape: bool,
    toolbar_drag: Option<(f32, f32)>,
    editor_size: (f32, f32),
    inspector_height: f32,
    hot_reload: crate::hot_reload::HotReload,
    hot_reload_status: crate::hot_reload::Status,
    hot_reload_pending: bool,
    _hot_reload_write: Option<Task<()>>,
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
        Self::new_with_hot_reload(
            store,
            session,
            port,
            window,
            cx,
            crate::hot_reload::HotReload::from_environment(),
        )
    }

    fn new_with_hot_reload(
        store: Store,
        session: Session,
        port: u16,
        window: &mut Window,
        cx: &mut Context<Self>,
        hot_reload: crate::hot_reload::HotReload,
    ) -> Self {
        let (server, notice) = match Server::start(store.clone(), port) {
            Ok(server) => (Some(server), String::new()),
            Err(error) => (
                None,
                format!("API offline: {error}. Local notes still work."),
            ),
        };
        let mcp_config_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .label("MCP config")
                .multi_line(1, usize::MAX)
                .highlighter(crate::ely::forms::json_highlights);
            input.set_text(
                server
                    .as_ref()
                    .map(|server| settings::mcp_config(&format!("http://{}/mcp", server.address())))
                    .unwrap_or_default(),
                cx,
            );
            input.set_disabled(true, cx);
            input
        });
        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line(1, 12)
                .max_len(8000)
                .label("Feedback")
                .placeholder("")
        });
        let inputs = vec![cx.observe(&input, |_, _, cx| cx.notify())];
        let hot_reload_status = hot_reload.status();
        let reload_watch = hot_reload.clone();
        let watch = store.clone();
        let refresh = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(300))
                    .await;
                let control = reload_watch.clone();
                let reload_status = cx
                    .background_executor()
                    .spawn(async move { control.status() })
                    .await;
                let Ok(revision) = watch.revision() else {
                    break;
                };
                if this
                    .update(cx, |this, cx| {
                        if !this.hot_reload_pending && this.hot_reload_status != reload_status {
                            this.hot_reload_status = reload_status;
                            cx.notify();
                        }
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
            api_port: port,
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
            banner: true,
            banner_bottom: 0.,
            settings_open: false,
            review_open: false,
            sheet_focus: cx.focus_handle(),
            sheet_previous_focus: None,
            settings_status: String::new(),
            mcp_starting: false,
            mcp_requested: true,
            mcp_config_input,
            mcp_config_copied: false,
            _copy_feedback: None,
            pointer: None,
            over_shape: false,
            toolbar_drag: None,
            editor_size: (384.0, 240.0),
            inspector_height: 144.0,
            hot_reload,
            hot_reload_status,
            hot_reload_pending: false,
            _hot_reload_write: None,
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

    fn choose_tool(
        &mut self,
        tool: Option<crate::ely::media::Tool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        if tool.is_none() && self.active && self.tool.is_none() {
            self.close(window, cx);
            return;
        }
        if !self.active {
            self.previous_focus = window.focused(cx);
        }
        self.active = true;
        self.tool = tool;
        self.stroke.clear();
        self.hovered = None;
        self.pointer = None;
        self.over_shape = false;
        window.focus(&self.focus, cx);
        cx.notify();
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
            editing: true,
            original_comment: String::new(),
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
        if !self.dismiss_note(window, cx) {
            return;
        }
        if !self.active && self.draft.is_none() {
            self.previous_focus = window.focused(cx);
        }
        match self.store.annotation(id) {
            Ok(a) => {
                self.draft = Some(Draft {
                    index: Some(a.index),
                    editing: false,
                    original_comment: a.feedback.comment.clone(),
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
                window.focus(&self.focus, cx);
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    fn start_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(draft) = &mut self.draft {
            draft.editing = true;
            window.focus(&self.input.focus_handle(cx), cx);
            cx.notify();
        }
    }

    fn dismiss_note(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(draft) = &self.draft else {
            return true;
        };
        let comment = self.input.read(cx).text().trim();
        let changed = draft.editing && comment != draft.original_comment.trim();
        if changed && (!comment.is_empty() || draft.id.is_some()) {
            if !self.save(window, cx) {
                return false;
            }
        }
        self.draft = None;
        self.notice.clear();
        self.return_focus(window, cx);
        cx.notify();
        true
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(draft) = &self.draft else {
            return false;
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
        let saved = result.is_ok();
        match result {
            Ok(note) => {
                if let Some(draft) = &mut self.draft {
                    draft.id = Some(note.id.clone());
                    draft.index = Some(note.index);
                    draft.editing = false;
                    draft.original_comment = note.feedback.comment.clone();
                }
                self.input.update(cx, |input, cx| {
                    input.set_text(note.feedback.comment.clone(), cx)
                });
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
                window.focus(&self.focus, cx);
                self.notice.clear();
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
        saved
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
        let editing = draft.editing;
        let confirm = surface_button::SurfaceButton::new(
            "feedback-save",
            if editing { "Save" } else { "Edit" },
            if editing {
                crate::ely::primitives::IconName::Save
            } else {
                crate::ely::primitives::IconName::Pencil
            },
            false,
        )
        .colors(cx.theme().colors.warning, cx.theme().colors.bg)
        .disabled(editing && self.input.read(cx).text().trim().is_empty())
        .on_click(cx.listener(|this, _, window, cx| {
            if this.draft.as_ref().is_some_and(|draft| draft.editing) {
                this.save(window, cx);
            } else {
                this.start_editing(window, cx);
            }
        }));
        let delete = surface_button::SurfaceButton::new(
            "feedback-delete",
            "Delete",
            crate::ely::primitives::IconName::Trash2,
            false,
        )
        .colors(cx.theme().colors.warning, cx.theme().colors.bg)
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
                            .size(IconSize::Sm)
                            .color(cx.theme().colors.bg),
                    )
                    .child(div().flex_1().child(format!("Note {index}")))
                    .child(
                        div()
                            .debug_selector(|| "feedback-editor-delete".into())
                            .child(delete),
                    )
                    .child(
                        div()
                            .debug_selector(|| "feedback-editor-primary".into())
                            .child(confirm),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .debug_selector(|| "feedback-editor-text".into())
                    .when(editing, |body| {
                        body.child(surface::comment_input(&self.input, window, cx))
                    })
                    .when(!editing, |body| {
                        body.child(
                            div()
                                .w_full()
                                .whitespace_normal()
                                .text_size(cx.theme().text_size(crate::ely::theme::TextSize::Base))
                                .line_height(gpui::relative(1.4))
                                .flex()
                                .flex_col()
                                .children(self.input.read(cx).text().split('\n').map(|line| {
                                    div()
                                        .w_full()
                                        .whitespace_normal()
                                        .child(if line.is_empty() {
                                            "\u{200b}".to_string()
                                        } else {
                                            line.trim_end_matches('\r').to_string()
                                        })
                                })),
                        )
                    }),
            )
    }
}

mod banner;
mod settings;
mod surface;
mod surface_button;

impl Render for AnnotationOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface = self.surface(window, cx);
        let owner = cx.entity().downgrade();
        div()
            .debug_selector(|| "feedback-inline-root".into())
            .w_full()
            .flex_none()
            .when(self.banner, |root| {
                root.child(
                    crate::ely::primitives::Measure::new(
                        "feedback-banner-measure",
                        move |bounds, _, cx| {
                            let bottom = f32::from(bounds.bottom());
                            let _ = owner.update(cx, |this, cx| {
                                if this.banner_bottom != bottom {
                                    this.banner_bottom = bottom;
                                    cx.notify();
                                }
                            });
                        },
                    )
                    .w_full()
                    .child(self.banner_toolbar(window, cx)),
                )
            })
            .child(surface)
    }
}

#[cfg(test)]
#[path = "overlay/editor.test.rs"]
mod editor_tests;
