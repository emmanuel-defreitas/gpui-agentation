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
    AnnotationInput, AnnotationStatus, Error, Server, Session, Store,
    target::{self, Target},
};

#[derive(Default)]
struct WindowStores(std::collections::HashMap<PathBuf, Store>);
impl gpui::Global for WindowStores {}

// Resolve existing ancestors too: a new notes file can be mounted through a
// symlinked directory before its first durable write creates the file.
fn store_path(path: &std::path::Path) -> std::io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
            match (parent, path.file_name()) {
                (Some(parent), Some(name)) => Ok(store_path(parent)?.join(name)),
                _ => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

struct Draft {
    index: Option<u64>,
    editing: bool,
    original_comment: String,
    color: usize,
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
    banner: bool,
    toolbar_hidden: bool,
    companion_toolbar: Option<gpui::WindowHandle<toolbar_window::ToolbarWindow>>,
    companion_toolbar_spec: Option<toolbar_window::ToolbarSpec>,
    toolbar_sync_pending: bool,
    companion_toolbar_active: bool,
    companion_toolbar_failure: Option<toolbar_window::ToolbarSpec>,
    history: Vec<(Option<crate::Annotation>, Option<crate::Annotation>)>,
    future: Vec<(Option<crate::Annotation>, Option<crate::Annotation>)>,
    pin_drag: Option<(crate::Annotation, (f32, f32))>,
    textbox_drag: Option<(crate::Annotation, (f32, f32))>,
    annotation_press: Option<gpui::Point<gpui::Pixels>>,
    textbox_bounds: std::collections::HashMap<String, gpui::Bounds<gpui::Pixels>>,
    mentions: mentions::Mentions,
    context_request: Option<(u64, gpui::Point<gpui::Pixels>)>,
    context_note: Option<String>,
    context_number: u64,
    settings_open: bool,
    review_open: bool,
    sheet_focus: FocusHandle,
    sheet_previous_focus: Option<FocusHandle>,
    settings_status: String,
    mcp_starting: bool,
    mcp_requested: bool,
    mcp_config_input: Entity<TextInput>,
    mcp_config_copied: bool,
    mcp_config_expanded: bool,
    _copy_feedback: Option<Task<()>>,
    pointer: Option<(f32, f32)>,
    over_shape: bool,
    editor_size: (f32, f32),
    inspected_id: Option<gpui::InspectorElementId>,
    inspected_configuration: Option<crate::GpuiConfiguration>,
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
        let path = path.into();
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        let path = store_path(&path)?;
        if !cx.has_global::<WindowStores>() {
            cx.set_global(WindowStores::default());
        }
        let store = if let Some(store) = cx.global::<WindowStores>().0.get(&path) {
            store.clone()
        } else {
            let store = Store::open(&path)?;
            cx.global_mut::<WindowStores>()
                .0
                .insert(path, store.clone());
            store
        };
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
        commands::init(cx);
        inspector::register(cx.entity().downgrade(), window, cx);
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
        let owner = cx.entity().downgrade();
        let window_id = window.window_handle().window_id();
        settings_popup::register(
            cx.entity().downgrade(),
            session.app_name.clone(),
            window,
            cx,
        );
        let shortcuts = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() == window_id {
                let _ = owner.update(cx, |this, cx| this.intercept_shortcut(event, window, cx));
            }
        });
        let menu_updates = cx.observe_self(move |this, cx| {
            this.sync_mentions(cx);
            this.sync_debug_menu(window_id, cx);
        });
        let menu_activation = cx.observe_window_activation(window, move |this, window, cx| {
            if window.is_window_active() {
                settings_popup::activate(window_id, cx);
                this.sync_debug_menu(window_id, cx);
            }
            this.sync_toolbar_window(window, cx);
            cx.notify();
        });
        let bounds_updates = cx.observe_window_bounds(window, |this, window, cx| {
            this.sync_toolbar_window(window, cx);
            cx.notify();
        });
        let overlay_id = cx.entity_id();
        let toolbar_cleanup = cx.on_release(move |this, cx| {
            settings_popup::unregister(window_id, overlay_id, cx);
            if let Some(handle) = this.companion_toolbar.take() {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        });
        let visibility_owner = cx.entity().downgrade();
        let toolbar_visibility = window.observe_window_visibility(move |_, window, cx| {
            let _ = visibility_owner.update(cx, |this, cx| {
                this.sync_toolbar_window(window, cx);
                cx.notify();
            });
        });
        let closed_owner = cx.entity().downgrade();
        let host_cleanup = cx.on_window_closed(move |cx, closed| {
            if closed == window_id {
                let _ = closed_owner.update(cx, |this, cx| {
                    if let Some(handle) = this.companion_toolbar.take() {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    }
                    this.companion_toolbar_spec = None;
                });
            }
        });
        let inputs = vec![
            bounds_updates,
            toolbar_cleanup,
            host_cleanup,
            toolbar_visibility,
            cx.observe(&input, |this, _, cx| {
                this.sync_mentions(cx);
                cx.notify();
            }),
            shortcuts,
            menu_updates,
            menu_activation,
        ];
        let hot_reload_status = hot_reload.status();
        let reload_watch = hot_reload.clone();
        let watch = store.clone();
        let preference = store
            .toolbar_preference(&session.app_name)
            .unwrap_or_default();
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
                            if let Ok(preference) =
                                this.store.toolbar_preference(&this.session.app_name)
                            {
                                this.banner = if preference.position == crate::ToolbarPosition::None
                                {
                                    preference.last_position == crate::ToolbarPosition::Banner
                                } else {
                                    preference.position == crate::ToolbarPosition::Banner
                                };
                                this.toolbar_hidden =
                                    preference.position == crate::ToolbarPosition::None;
                                this.toolbar_at = preference.at;
                            }
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
            toolbar_at: preference.at,
            banner: if preference.position == crate::ToolbarPosition::None {
                preference.last_position == crate::ToolbarPosition::Banner
            } else {
                preference.position == crate::ToolbarPosition::Banner
            },
            toolbar_hidden: preference.position == crate::ToolbarPosition::None,
            companion_toolbar: None,
            companion_toolbar_spec: None,
            toolbar_sync_pending: false,
            companion_toolbar_active: false,
            companion_toolbar_failure: None,
            history: vec![],
            future: vec![],
            pin_drag: None,
            textbox_drag: None,
            annotation_press: None,
            textbox_bounds: Default::default(),
            mentions: Default::default(),
            context_request: None,
            context_note: None,
            context_number: 0,
            settings_open: false,
            review_open: false,
            sheet_focus: cx.focus_handle(),
            sheet_previous_focus: None,
            settings_status: String::new(),
            mcp_starting: false,
            mcp_requested: true,
            mcp_config_input,
            mcp_config_copied: false,
            mcp_config_expanded: false,
            _copy_feedback: None,
            pointer: None,
            over_shape: false,
            editor_size: (384.0, 240.0),
            inspected_id: None,
            inspected_configuration: None,
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
                inspector::close(window, cx);
                self.inspected_id = None;
                self.inspected_configuration = None;
                self.session = session;
                self.history.clear();
                self.future.clear();
                self.selected_note = None;
                self.pin_drag = None;
                self.textbox_drag = None;
                self.textbox_bounds.clear();
                self.mentions = Default::default();
                self.context_request = None;
                self.context_note = None;
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
        self.companion_toolbar_failure = None;
        if self.active || self.draft.is_some() {
            self.close(window, cx);
        } else {
            self.previous_focus = window.focused(cx);
            self.active = true;
            self.tool = None;
            inspector::start(window, cx);
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
        if !self.active {
            self.previous_focus = window.focused(cx);
        }
        self.active = true;
        self.tool = tool;
        self.inspected_id = None;
        self.inspected_configuration = None;
        if tool.is_none() {
            inspector::start(window, cx);
        } else {
            inspector::close(window, cx);
        }
        self.stroke.clear();
        self.hovered = None;
        self.pointer = None;
        self.over_shape = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        inspector::close(window, cx);
        self.inspected_id = None;
        self.inspected_configuration = None;
        self.active = false;
        self.draft = None;
        self.pin_drag = None;
        if self.textbox_drag.take().is_some() {
            if let Ok(session) = self.store.session(&self.session.id) {
                self.session = session;
            }
        }
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
            color: self.color,
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
        inspector::close(window, cx);
        self.inspected_id = None;
        self.inspected_configuration = None;
        if !self.active && self.draft.is_none() {
            self.previous_focus = window.focused(cx);
        }
        match self.store.annotation(id) {
            Ok(a) => {
                self.selected_note = Some(a.id.clone());
                self.draft = Some(Draft {
                    index: Some(a.index),
                    editing: matches!(a.feedback.shape, Some(crate::AnnotationShape::Text { .. })),
                    original_comment: a.feedback.comment.clone(),
                    color: a.annotation_color(),
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
                if self.draft.as_ref().is_some_and(|d| d.editing) {
                    window.focus(&self.input.focus_handle(cx), cx);
                } else {
                    window.focus(&self.focus, cx);
                }
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
        let changed = (draft.editing || self.inspected_id.is_some())
            && comment != draft.original_comment.trim();
        if changed
            && (!comment.is_empty() || draft.id.is_some() || self.inspected_configuration.is_some())
        {
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
        let before = match &draft.id {
            Some(id) => match self.store.annotation(id) {
                Ok(note) => Some(note),
                Err(error) => {
                    self.notice = error.to_string();
                    cx.notify();
                    return false;
                }
            },
            None => None,
        };
        let mut shape = draft.shape.clone();
        if let Some(crate::AnnotationShape::Text { text, .. }) = &mut shape {
            *text = comment.clone();
        }
        let result = if let Some(previous) = &before {
            let mut note = previous.clone();
            note.feedback.comment = comment;
            note.feedback.shape = shape;
            if self.inspected_id.is_some() {
                note.feedback.gpui_configuration = self.inspected_configuration.clone();
            }
            self.store
                .replace_note(Some(previous), Some(&note))
                .map(|_| note)
        } else {
            self.store.add_colored(
                &self.session.id,
                AnnotationInput {
                    comment,
                    element_path: draft.target.id.clone(),
                    element: draft.target.label.clone(),
                    source_file: Some(draft.target.source.clone()).filter(|s| !s.is_empty()),
                    gpui_configuration: self
                        .inspected_id
                        .as_ref()
                        .and(self.inspected_configuration.clone()),
                    bounding_box: draft.target.bounds,
                    x: draft.x,
                    y: draft.y,
                    shape,
                },
                Some(draft.color),
            )
        };
        let saved = result.is_ok();
        match result {
            Ok(note) => {
                self.record_change(before, Some(note.clone()));
                if let Some(draft) = &mut self.draft {
                    draft.id = Some(note.id.clone());
                    draft.index = Some(note.index);
                    draft.editing = self.inspected_id.is_some();
                    draft.original_comment = note.feedback.comment.clone();
                    draft.shape = note.feedback.shape.clone();
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
                if matches!(
                    self.draft.as_ref().and_then(|d| d.shape.as_ref()),
                    Some(crate::AnnotationShape::Text { .. })
                ) {
                    self.draft = None;
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
        let before = self.store.annotation(id).ok();
        if let Err(error) = self.store.replace_note(before.as_ref(), None) {
            self.notice = error.to_string();
            cx.notify();
            return false;
        }
        self.record_change(before, None);
        self.session.annotations.retain(|note| note.id != id);
        self.textbox_bounds.remove(id);
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
        let textbox = matches!(draft.shape, Some(crate::AnnotationShape::Text { .. }));
        let background = cx.theme().colors.chart[draft.color];
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
        .icon_only()
        .colors(background, cx.theme().colors.bg)
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
        .icon_only()
        .colors(background, cx.theme().colors.bg)
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
                    .child(
                        div()
                            .flex_1()
                            .text_size(cx.theme().text_size(crate::ely::theme::TextSize::Sm))
                            .child(if textbox {
                                "Textbox".to_string()
                            } else {
                                format!("Note {index}")
                            }),
                    )
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
                        body.child(surface::comment_input(
                            &self.input,
                            background,
                            textbox,
                            window,
                            cx,
                        ))
                    })
                    .when(!editing, |body| {
                        body.child(
                            div()
                                .w_full()
                                .whitespace_normal()
                                .text_color(cx.theme().colors.bg.opacity(0.5))
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

#[cfg(test)]
use crate::AnnotationPatch;

mod annotation_menu;
mod banner;
pub mod commands;
mod history;
pub(crate) mod inspector;
mod mentions;
#[cfg(test)]
#[path = "overlay/release.test.rs"]
mod release_tests;
mod settings;
mod settings_popup;
mod surface;
mod surface_button;
mod textbox;
mod toolbar_window;

impl Render for AnnotationOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_toolbar_window(window, cx);
        let surface = self.surface(window, cx);
        div()
            .debug_selector(|| "feedback-inline-root".into())
            .when(
                self.active && !self.settings_open && !self.review_open,
                |root| root.key_context(self.shortcut_context(window, cx)),
            )
            .capture_action(cx.listener(
                |this, action: &commands::AnnotationCommand, window, cx| {
                    this.dispatch_command(action, window, cx);
                    cx.stop_propagation();
                },
            ))
            .w_full()
            .flex_none()
            .child(surface)
    }
}

#[cfg(test)]
#[path = "overlay/editor.test.rs"]
mod editor_tests;
