//! Native picking and inspection annotations on stock GPUI.
//!
//! GPUI draws its inspector as a fixed-width sidebar inside the inspected
//! window and lays the app out in what remains. Nothing public turns that off,
//! so the host's native content view is kept that much wider than the window
//! shows: GPUI lays the sidebar out past the right edge, where it is never
//! seen, and the app keeps its whole visible width. The companion window then
//! renders the same `Inspector` entity as its own panel, so the toolbar grows
//! while inspecting and the consumer does not move.
use super::*;
use crate::ely::{
    buttons::{Button, ButtonVariant},
    tooling,
};
use gpui::{Global, Inspector, InspectorElementId, Pixels, WeakEntity, WindowId, rems};
use std::collections::HashMap;

#[derive(Default)]
struct Sidebars(HashMap<WindowId, Sidebar>);
impl Global for Sidebars {}
struct Sidebar {
    overlay: WeakEntity<AnnotationOverlay>,
    inspector: Option<WeakEntity<Inspector>>,
    presented: bool,
    /// Set while this crate has GPUI's inspector turned on for the window.
    opened: bool,
    /// The reserved sidebar width, kept out of sight while the inspector is on.
    margin: Option<toolbar_window::HostMargin>,
    /// The selection the companion was last told about.
    announced: Option<Option<InspectorElementId>>,
}

/// The sidebar GPUI reserves at the window's right while its inspector is on.
/// Mirrors `Window::draw_roots` in `gpui-pre 0.3.8`.
pub(super) fn sidebar_width(window: &Window) -> Pixels {
    rems(30.0).to_pixels(window.rem_size())
}

pub(crate) fn install(cx: &mut App) {
    if cx.has_global::<Sidebars>() {
        return;
    }
    tooling::install_inspector(cx);
    cx.set_global(Sidebars::default());
    cx.set_inspector_renderer(Box::new(|inspector, window, cx| {
        let target = tooling::inspector_target(window, cx);
        let native_owner = cx.entity().downgrade();
        let in_host = target == window.window_handle();
        let active = inspector.active_element_id().cloned();
        let (owner, external, changed) = cx
            .global_mut::<Sidebars>()
            .0
            .get_mut(&target.window_id())
            .map(|sidebar| {
                sidebar.inspector = Some(native_owner);
                let external = sidebar
                    .margin
                    .as_ref()
                    .is_some_and(|margin| margin.is_hidden());
                let changed = in_host && external && sidebar.announced.as_ref() != Some(&active);
                if changed {
                    sidebar.announced = Some(active);
                }
                (Some(sidebar.overlay.clone()), external, changed)
            })
            .unwrap_or((None, false, false));
        if changed {
            // The companion shows this entity; wake it when the pick changes.
            cx.notify();
        }
        if in_host && external {
            // The sidebar lies past the window's edge: sync the selection for
            // the annotation and, once picked, outline the element in the app.
            // The panel itself renders in the companion.
            let color = owner.and_then(|owner| {
                owner
                    .update(cx, |this, cx| {
                        this.sync_inspector_selection(inspector, window, cx);
                        cx.theme().colors.chart[this.color]
                    })
                    .ok()
            });
            let outline = (!inspector.is_picking())
                .then(|| tooling::inspector_selection(window, cx))
                .flatten()
                .filter(|selection| Some(&selection.id) == inspector.active_element_id())
                .zip(color)
                .map(|(selection, color)| (selection.bounds, color));
            // GPUI lays this element out in the hidden strip; the quad takes
            // window coordinates and the root mask is the whole viewport.
            return gpui::canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    if let Some((bounds, color)) = outline {
                        window.paint_quad(
                            gpui::outline(bounds, color, gpui::BorderStyle::Solid)
                                .border_widths(gpui::px(2.)),
                        );
                    }
                },
            )
            .into_any_element();
        }
        let toggle_owner = owner.clone();
        // Ely already titles the panel "Inspector"; the header is only Stop.
        let header = div()
            .debug_selector(|| "feedback-inspect-stop".into())
            .child(
                Button::new("feedback-inspect-stop", "Stop")
                    .variant(ButtonVariant::Primary)
                    .size(ControlSize::Sm)
                    .on_click(move |_, _, cx| {
                        let owner = toggle_owner.clone();
                        cx.defer(move |cx| {
                            let _ = target.update(cx, |_, host, cx| {
                                if let Some(owner) = owner {
                                    let _ =
                                        owner.update(cx, |this, cx| this.stop_inspection(host, cx));
                                }
                            });
                        });
                    }),
            )
            .into_any_element();
        tooling::inspector_panel_with_header(
            inspector,
            window,
            cx,
            Some(header),
            |inspector, window, cx| {
                if in_host {
                    if let Some(owner) = owner {
                        let _ = owner.update(cx, |this, cx| {
                            this.sync_inspector_selection(inspector, window, cx)
                        });
                    }
                }
                None
            },
        )
    }));
}

/// The host's live inspector entity, for the companion to render.
pub(super) fn entity(window: &Window, cx: &App) -> Option<Entity<Inspector>> {
    native_inspector(window, cx)
}

/// Whether the panel renders in the companion, with GPUI's sidebar out of sight.
pub(super) fn is_external(window: &Window, cx: &App) -> bool {
    cx.try_global::<Sidebars>()
        .and_then(|sidebars| sidebars.0.get(&window.window_handle().window_id()))
        .and_then(|sidebar| sidebar.margin.as_ref())
        .is_some_and(|margin| margin.is_hidden())
}

pub(super) fn register(owner: WeakEntity<AnnotationOverlay>, window: &Window, cx: &mut App) {
    install(cx);
    cx.global_mut::<Sidebars>().0.insert(
        window.window_handle().window_id(),
        Sidebar {
            overlay: owner,
            inspector: None,
            presented: false,
            opened: false,
            margin: None,
            announced: None,
        },
    );
}

pub(super) fn is_open(window: &Window, cx: &App) -> bool {
    cx.try_global::<Sidebars>()
        .and_then(|sidebars| sidebars.0.get(&window.window_handle().window_id()))
        .is_some_and(|sidebar| sidebar.presented)
}

/// Whether GPUI's inspector sidebar currently takes width from this window.
pub(super) fn sidebar_open(window: &Window, cx: &App) -> bool {
    window.is_inspector_picking(cx)
        || cx
            .try_global::<Sidebars>()
            .and_then(|sidebars| sidebars.0.get(&window.window_handle().window_id()))
            .is_some_and(|sidebar| sidebar.opened)
}

fn native_inspector(window: &Window, cx: &App) -> Option<Entity<Inspector>> {
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

fn sidebar_mut<'a>(window: &Window, cx: &'a mut App) -> Option<&'a mut Sidebar> {
    cx.global_mut::<Sidebars>()
        .0
        .get_mut(&window.window_handle().window_id())
}

fn present(window: &Window, presented: bool, cx: &mut App) {
    if let Some(sidebar) = sidebar_mut(window, cx) {
        sidebar.presented = presented;
    }
}

/// Turn picking on. With `companion`, the panel renders in the toolbar window
/// and GPUI's sidebar is kept out of sight; without it (toolbar hidden, or a
/// window filling its display, where the companion does not show) the panel
/// stays inside the host.
pub(super) fn start(window: &mut Window, companion: bool, cx: &mut App) {
    present(window, true, cx);
    if let Some(inspector) = native_inspector(window, cx) {
        inspector.update(cx, |inspector, cx| {
            inspector.start_picking();
            cx.notify();
        });
        window.refresh();
        return;
    }
    // Already toggled on and awaiting its first draw: toggling again would
    // close it and widen the window twice.
    if sidebar_mut(window, cx).is_some_and(|sidebar| sidebar.opened) {
        return;
    }
    tooling::clear_inspector_selection(window, cx);
    window.toggle_inspector(cx);
    let margin = (companion && !toolbar_window::fills_display(window))
        .then(|| toolbar_window::HostMargin::reserve(window, sidebar_width(window), cx));
    if let Some(sidebar) = sidebar_mut(window, cx) {
        sidebar.opened = true;
        sidebar.margin = margin;
        sidebar.announced = None;
    }
}

pub(super) fn close(window: &mut Window, cx: &mut App) {
    let (opened, margin) = sidebar_mut(window, cx).map_or((false, None), |sidebar| {
        sidebar.presented = false;
        sidebar.announced = None;
        (std::mem::take(&mut sidebar.opened), sidebar.margin.take())
    });
    if opened || native_inspector(window, cx).is_some() || window.is_inspector_picking(cx) {
        window.toggle_inspector(cx);
    }
    if let Some(margin) = margin {
        margin.release(cx);
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

/// Whether a saved style override should be re-applied to the picked element.
///
/// Only a fresh pick restores it. Once the element is the current selection,
/// later edits and Reset must reach the update logic instead of being undone.
pub(super) fn should_restore(
    restore_pending: bool,
    already_selected: bool,
    saved: &crate::GpuiConfiguration,
    live: Option<&crate::GpuiConfiguration>,
) -> bool {
    !restore_pending && !already_selected && Some(saved) != live
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

    /// Resume a saved inspection. Stock GPUI cannot reselect an element by
    /// identity, so picking reopens and the next pick of that element continues
    /// the note with its saved overrides.
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
        self.review_open = false;
        self.inspecting = true;
        self.inspected_id = None;
        self.inspected_configuration = None;
        self.restore_configuration = None;
        self.inspection_note = Some(note.id.clone());
        self.selected_note = Some(note.id);
        self.notice = "Pick the element again to continue this inspection.".into();
        inspector::start(window, !self.toolbar_hidden, cx);
        cx.notify();
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
        // GPUI's own sidebar is inspectable while picking; a pick that lands
        // there is noise, so picking simply resumes.
        let (content_width, _) = surface::viewport(window, cx);
        if f32::from(selection.bounds.origin.x) >= content_width {
            let host = window.window_handle();
            cx.defer(move |cx| {
                let _ = host.update(cx, |_, window, cx| resume_picking(window, cx));
            });
            return false;
        }
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
        if let Some(note) = &existing
            && let Some(saved) = &note.feedback.gpui_configuration
            && should_restore(
                self.restore_configuration.is_some(),
                self.inspected_id.as_ref() == Some(&selection.id),
                saved,
                config.as_ref(),
            )
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
            let (w, _) = surface::viewport(window, cx);
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
