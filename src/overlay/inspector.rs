//! Native picking and element-anchored inspection annotations.
use super::*;
use crate::ely::{
    buttons::{Button, ButtonVariant},
    tooling,
};
use gpui::{Global, Inspector, WeakEntity, WindowId};
use std::collections::HashMap;

#[derive(Default)]
struct Sidebars(HashMap<WindowId, Sidebar>);
impl Global for Sidebars {}
struct Sidebar {
    overlay: WeakEntity<AnnotationOverlay>,
    inspector: Option<WeakEntity<Inspector>>,
    presented: bool,
}

pub(crate) fn install(cx: &mut App) {
    if cx.has_global::<Sidebars>() {
        return;
    }
    tooling::install_inspector(cx);
    cx.set_global(Sidebars::default());
    cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
        #[cfg(agentation_external_inspector)]
        let rendered_id = cx.entity_id();
        let target = tooling::inspector_target(window, cx);
        let native_owner = cx.entity().downgrade();
        let owner = cx
            .global_mut::<Sidebars>()
            .0
            .get_mut(&target.window_id())
            .map(|sidebar| {
                sidebar.inspector = Some(native_owner);
                sidebar.overlay.clone()
            });
        let toggle_owner = owner.clone();
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child("Inspector")
            .child(
                div()
                    .debug_selector(|| "feedback-inspect-stop".into())
                    .child(
                        Button::new("feedback-inspect-stop", "Stop")
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, _, cx| {
                                let owner = toggle_owner.clone();
                                cx.defer(move |cx| {
                                    let _ = target.update(cx, |_, host, cx| {
                                        if let Some(owner) = owner {
                                            let _ = owner.update(cx, |this, cx| {
                                                this.stop_inspection(host, cx)
                                            });
                                        }
                                    });
                                });
                            }),
                    ),
            )
            .into_any_element();
        tooling::inspector_panel_with_header(
            inspector,
            window,
            cx,
            Some(header),
            |inspector, window, cx| {
                #[cfg(agentation_external_inspector)]
                if target != window.window_handle() {
                    let _ = target.update(cx, |_, host, cx| {
                        if host
                            .inspector_entity()
                            .is_none_or(|active| active.entity_id() != rendered_id)
                            || !is_open(host, cx)
                        {
                            return;
                        }
                        if let Some(owner) = owner {
                            let _ = owner.update(cx, |this, cx| {
                                this.sync_inspector_selection(inspector, host, cx)
                            });
                        }
                    });
                    return None;
                }
                if let Some(owner) = owner {
                    let _ = owner.update(cx, |this, cx| {
                        this.sync_inspector_selection(inspector, window, cx)
                    });
                }
                None
            },
        )
    }));
}

pub(super) fn register(owner: WeakEntity<AnnotationOverlay>, window: &Window, cx: &mut App) {
    install(cx);
    cx.global_mut::<Sidebars>().0.insert(
        window.window_handle().window_id(),
        Sidebar {
            overlay: owner,
            inspector: None,
            presented: false,
        },
    );
}

pub(super) fn is_open(window: &Window, cx: &App) -> bool {
    cx.try_global::<Sidebars>()
        .and_then(|sidebars| sidebars.0.get(&window.window_handle().window_id()))
        .is_some_and(|sidebar| sidebar.presented)
}

fn native_inspector(window: &Window, cx: &App) -> Option<Entity<Inspector>> {
    #[cfg(agentation_external_inspector)]
    {
        let _ = cx;
        window.inspector_entity()
    }
    #[cfg(not(agentation_external_inspector))]
    cx.try_global::<Sidebars>()?
        .0
        .get(&window.window_handle().window_id())?
        .inspector
        .as_ref()?
        .upgrade()
}

pub(super) fn is_picking(window: &Window, cx: &App) -> bool {
    native_inspector(window, cx).is_some_and(|inspector| inspector.read(cx).is_picking())
}

pub(super) fn resume_picking(window: &mut Window, cx: &mut App) {
    if let Some(inspector) = native_inspector(window, cx) {
        inspector.update(cx, |inspector, cx| {
            inspector.start_picking();
            cx.notify();
        });
        window.refresh();
    }
}

fn present(window: &Window, presented: bool, cx: &mut App) {
    if let Some(sidebar) = cx
        .global_mut::<Sidebars>()
        .0
        .get_mut(&window.window_handle().window_id())
    {
        sidebar.presented = presented;
    }
}

pub(super) fn start(window: &mut Window, cx: &mut App) {
    present(window, true, cx);
    #[cfg(agentation_external_inspector)]
    window.set_inspector_external(true);
    if let Some(inspector) = native_inspector(window, cx) {
        inspector.update(cx, |inspector, cx| {
            inspector.start_picking();
            cx.notify();
        });
        window.refresh();
    } else {
        tooling::clear_inspector_selection(window, cx);
        window.toggle_inspector(cx);
    }
}

pub(super) fn close(window: &mut Window, cx: &mut App) {
    present(window, false, cx);
    if native_inspector(window, cx).is_some() || window.is_inspector_picking(cx) {
        window.toggle_inspector(cx);
    }
    tooling::clear_inspector_selection(window, cx);
}

fn configuration(window: &Window, cx: &App) -> Option<crate::GpuiConfiguration> {
    tooling::active_inspector_configuration(window, cx).map(|configuration| {
        crate::GpuiConfiguration {
            source: configuration.source,
            element_path: configuration.element_path,
            changes: configuration
                .changes
                .into_iter()
                .map(|change| crate::GpuiStyleChange {
                    property: change.property,
                    before: change.before,
                    after: change.after,
                })
                .collect(),
            rust: configuration.rust,
        }
    })
}

impl AnnotationOverlay {
    pub(super) fn stop_inspection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        inspector::close(window, cx);
        self.inspecting = false;
        self.inspected_id = None;
        self.inspection_note = None;
        self.restore_configuration = None;
        self.inspected_configuration = None;
        self.stroke.clear();
        cx.notify();
    }

    #[cfg(agentation_external_inspector)]
    pub(super) fn keep_inspection_anchors(&self, window: &mut Window, cx: &mut App) {
        if window.inspector_entity().is_none()
            && self
                .session
                .annotations
                .iter()
                .any(|note| note.feedback.inspection.is_some())
        {
            window.set_inspector_external(true);
            window.toggle_inspector(cx);
            if let Some(inspector) = window.inspector_entity() {
                inspector.update(cx, |inspector, _| inspector.stop_picking());
            }
        }
    }

    pub(super) fn open_inspection(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Ok(note) = self.store.annotation(id) else {
            return;
        };
        if note.url != self.session.url || note.feedback.inspection.is_none() {
            return;
        }
        if !self.dismiss_note(window, cx) {
            return;
        }
        #[cfg(agentation_external_inspector)]
        {
            self.keep_inspection_anchors(window, cx);
            if !window.select_inspection_anchor(&note.feedback.element_path, cx) {
                self.notice = "This element is not available in the current view.".into();
                cx.notify();
                return;
            }
            present(window, true, cx);
            self.inspecting = true;
            self.inspected_id = window
                .inspector_entity()
                .and_then(|inspector| inspector.read(cx).active_element_id().cloned());
            self.restore_configuration = note.feedback.gpui_configuration.clone();
            self.review_open = false;
            self.inspection_note = Some(note.id.clone());
            self.selected_note = Some(note.id);
            self.notice.clear();
            cx.notify();
        }
        #[cfg(not(agentation_external_inspector))]
        {
            self.notice =
                "Reopening an anchored element requires the external inspector build.".into();
            cx.notify();
        }
    }

    fn sync_inspector_selection(
        &mut self,
        inspector: &Inspector,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if inspector.is_picking() || !self.inspecting {
            return false;
        }
        let Some(selection) = tooling::inspector_selection(window, cx)
            .filter(|selection| Some(&selection.id) == inspector.active_element_id())
        else {
            return false;
        };
        if let Some(configuration) = self.restore_configuration.take() {
            let id = selection.id.clone();
            let owner = cx.entity().downgrade();
            let host = window.window_handle();
            cx.defer(move |cx| {
                let _ = host.update(cx, |_, window, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if !this.inspecting || this.inspected_id.as_ref() != Some(&id) {
                            return;
                        }
                        let configuration = tooling::InspectorStyleConfiguration {
                            source: configuration.source,
                            element_path: configuration.element_path,
                            rust: configuration.rust,
                            changes: configuration
                                .changes
                                .into_iter()
                                .map(|change| tooling::InspectorStyleChange {
                                    property: change.property,
                                    before: change.before,
                                    after: change.after,
                                })
                                .collect(),
                        };
                        tooling::restore_inspector_configuration(&id, configuration, window, cx);
                    });
                });
            });
            return false;
        }
        let location = selection.id.path.source_location;
        let source = format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
        let path = selection.id.path.global_id.to_string();
        let key = format!("gpui:{path}@{source}#{}", selection.id.instance_id);
        let bounds = selection.bounds;
        let snapshot = crate::InspectionSnapshot {
            properties: selection.properties,
        };
        let config = configuration(window, cx);
        let existing = self
            .session
            .annotations
            .iter()
            .find(|note| {
                note.feedback.inspection.is_some()
                    && (note.feedback.element_path == key
                        || self.inspection_note.as_deref() == Some(&note.id)
                            && self.inspected_id.as_ref() == Some(&selection.id))
            })
            .cloned();
        // A fresh pick of an existing inspection must restore its saved style
        // before capturing the selection again; otherwise the baseline would
        // overwrite the annotation's last saved configuration.
        if self.restore_configuration.is_none()
            && let Some(note) = &existing
            && let Some(saved) = &note.feedback.gpui_configuration
            && Some(saved) != config.as_ref()
        {
            self.inspected_id = Some(selection.id.clone());
            self.inspection_note = Some(note.id.clone());
            self.restore_configuration = Some(saved.clone());
            return false;
        }
        let changed = existing.as_ref().is_none_or(|note| {
            note.feedback.inspection.as_ref() != Some(&snapshot)
                || note.feedback.gpui_configuration != config
        });
        self.inspected_id = Some(selection.id);
        self.inspected_configuration = config.clone();
        if changed {
            let rect = crate::Rect {
                x: bounds.origin.x.into(),
                y: bounds.origin.y.into(),
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            };
            let (w, _) = surface::viewport(window);
            let feedback = AnnotationInput {
                inspection: Some(snapshot),
                comment: existing
                    .as_ref()
                    .map_or_else(String::new, |note| note.feedback.comment.clone()),
                element_path: key,
                element: if path.is_empty() {
                    "GPUI element".into()
                } else {
                    path
                },
                x: ((rect.x + rect.width / 2.) / w.max(1.) * 100.).clamp(0., 100.),
                y: rect.y + rect.height / 2.,
                bounding_box: rect,
                source_file: Some(source),
                gpui_configuration: config,
                shape: None,
            };
            let result = if let Some(previous) = &existing {
                let mut next = previous.clone();
                next.feedback = feedback;
                self.store
                    .replace_note(Some(previous), Some(&next))
                    .map(|_| next)
            } else {
                self.store
                    .add_colored(&self.session.id, feedback, Some(self.color))
            };
            match result {
                Ok(note) => {
                    if existing.is_none() {
                        self.record_change(None, Some(note.clone()));
                    } else if let Some(previous) = existing {
                        self.record_inspection_change(previous, note.clone());
                    }
                    self.inspection_note = Some(note.id.clone());
                    self.selected_note = Some(note.id.clone());
                    if let Some(old) = self
                        .session
                        .annotations
                        .iter_mut()
                        .find(|old| old.id == note.id)
                    {
                        *old = note;
                    } else {
                        self.session.annotations.push(note);
                    }
                    self.notice.clear();
                    cx.notify();
                }
                Err(error) => {
                    self.notice = error.to_string();
                    cx.notify();
                }
            }
        } else if let Some(note) = existing {
            self.inspection_note = Some(note.id);
        }
        true
    }
}
