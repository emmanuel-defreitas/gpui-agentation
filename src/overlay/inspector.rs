//! One GPUI picker, property editor and feedback sidebar per window.
use super::*;
use crate::ely::{primitives::IconName, theme::TextSize, tooling};
use gpui::{AnyElement, Global, Inspector, WeakEntity, WindowId, px};
use std::collections::HashMap;

#[derive(Default)]
struct Sidebars(HashMap<WindowId, Sidebar>);
impl Global for Sidebars {}

struct Sidebar {
    overlay: WeakEntity<AnnotationOverlay>,
    inspector: Option<WeakEntity<Inspector>>,
    #[cfg(agentation_external_inspector)]
    footer: Option<WeakEntity<external::ExternalFeedback>>,
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
        let native_owner = cx.entity().downgrade();
        let target = tooling::inspector_target(window, cx);
        let owner = cx
            .global_mut::<Sidebars>()
            .0
            .get_mut(&target.window_id())
            .map(|sidebar| {
                sidebar.inspector = Some(native_owner);
                sidebar.overlay.clone()
            });
        tooling::inspector_panel(inspector, window, cx, |inspector, window, cx| {
            #[cfg(agentation_external_inspector)]
            if target != window.window_handle() {
                let ready = owner
                    .as_ref()
                    .and_then(|owner| {
                        target
                            .update(cx, |_, host, cx| {
                                // Exit animation retains the old view, never its model effects.
                                if host
                                    .inspector_entity()
                                    .is_none_or(|active| active.entity_id() != rendered_id)
                                {
                                    return Some(true);
                                }
                                owner
                                    .update(cx, |this, cx| {
                                        this.sync_inspector_selection(inspector, host, cx)
                                    })
                                    .ok()
                            })
                            .ok()
                            .flatten()
                    })
                    .unwrap_or(false);
                return ready
                    .then(|| {
                        cx.global::<Sidebars>()
                            .0
                            .get(&target.window_id())?
                            .footer
                            .as_ref()?
                            .upgrade()
                            .map(IntoElement::into_any_element)
                    })
                    .flatten();
            }
            owner
                .and_then(|owner| {
                    owner
                        .update(cx, |this, cx| this.inspector_footer(inspector, window, cx))
                        .ok()
                })
                .flatten()
        })
    }));
}

pub(super) fn register(owner: WeakEntity<AnnotationOverlay>, window: &Window, cx: &mut App) {
    install(cx);
    cx.global_mut::<Sidebars>().0.insert(
        window.window_handle().window_id(),
        Sidebar {
            overlay: owner,
            inspector: None,
            #[cfg(agentation_external_inspector)]
            footer: None,
        },
    );
}

pub(super) fn is_open(window: &Window, cx: &App) -> bool {
    native_inspector(window, cx).is_some() || window.is_inspector_picking(cx)
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

pub(super) fn start(window: &mut Window, cx: &mut App) {
    #[cfg(agentation_external_inspector)]
    window.set_inspector_external(true);
    if let Some(inspector) = native_inspector(window, cx) {
        inspector.update(cx, |inspector, cx| {
            inspector.start_picking();
            cx.notify();
        });
        window.refresh();
    } else if !window.is_inspector_picking(cx) {
        tooling::clear_inspector_selection(window, cx);
        window.toggle_inspector(cx);
    }
}

pub(super) fn close(window: &mut Window, cx: &mut App) {
    if is_open(window, cx) {
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
    fn sync_inspector_selection(
        &mut self,
        inspector: &Inspector,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if inspector.is_picking() {
            // Keep the previous draft's cached edits until its feedback is saved.
            if self.inspected_id.is_some() && self.dismiss_note(window, cx) {
                self.inspected_id = None;
                self.inspected_configuration = None;
            }
            return false;
        }
        let selection = tooling::inspector_selection(window, cx)
            .filter(|selection| Some(&selection.id) == inspector.active_element_id());
        let Some(selection) = selection else {
            return false;
        };
        if self.inspected_id.as_ref() != Some(&selection.id) || self.draft.is_none() {
            if !self.dismiss_note(window, cx) {
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
            let bounds = selection.bounds;
            let target = Target {
                id: format!("gpui:{path}@{source}#{}", selection.id.instance_id),
                label: if path.is_empty() {
                    "GPUI element".into()
                } else {
                    path
                },
                source,
                bounds: crate::Rect {
                    x: bounds.origin.x.into(),
                    y: bounds.origin.y.into(),
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                },
            };
            let (w, _) = surface::viewport(window);
            let x = (target.bounds.x + target.bounds.width / 2.0) / w.max(1.0) * 100.0;
            let y = target.bounds.y + target.bounds.height / 2.0;
            self.select(target, x.clamp(0.0, 100.0), y, window, cx);
            self.inspected_id = Some(selection.id);
            // Comments are optional. Picking leaves keyboard focus on the tools.
            window.focus(&self.focus, cx);
        }
        let configuration = configuration(window, cx);
        if self.inspected_configuration != configuration {
            self.inspected_configuration = configuration;
            cx.notify();
        }
        true
    }

    fn inspector_footer(
        &mut self,
        inspector: &Inspector,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.sync_inspector_selection(inspector, window, cx) {
            if !inspector.is_picking() && !self.notice.is_empty() {
                return Some(
                    div()
                        .text_color(cx.theme().colors.danger)
                        .child(self.notice.clone())
                        .into_any_element(),
                );
            }
            return None;
        }
        let mut comment = Input::new(&self.input)
            .render(window, cx)
            .into_any_element();
        if let Some(frame) = comment.downcast_mut::<gpui::Div>() {
            frame
                .style()
                .refine(&gpui::StyleRefinement::default().min_h(px(96.0)));
        }
        let theme = cx.theme();
        let can_save =
            !self.input.read(cx).text().trim().is_empty() || self.inspected_configuration.is_some();
        let saved = self
            .draft
            .as_ref()
            .and_then(|draft| draft.id.as_ref())
            .is_some();
        Some(
            div()
                .debug_selector(|| "feedback-inspector-sidebar".into())
                .flex()
                .flex_col()
                .flex_none()
                .gap_2()
                .px_3()
                .pt_3()
                .border_t_1()
                .border_color(theme.colors.border)
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Feedback"),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_muted)
                        .child(
                            "Add an optional comment. Saved feedback includes your style changes.",
                        ),
                )
                .child(
                    div()
                        .id("feedback-inspector-comment-scroll")
                        .max_h(px(160.0))
                        .overflow_y_scroll()
                        .debug_selector(|| "feedback-inspector-comment".into())
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                window.prevent_default();
                                window.focus(&this.input.focus_handle(cx), cx);
                            }),
                        )
                        .child(comment),
                )
                .when(!self.notice.is_empty(), |body| {
                    body.child(self.notice.clone())
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            div()
                                .debug_selector(|| "feedback-inspector-save".into())
                                .child(
                                    Button::new(
                                        "feedback-inspector-save",
                                        if saved {
                                            "Update feedback"
                                        } else {
                                            "Save feedback"
                                        },
                                    )
                                    .icon(IconName::Save)
                                    .disabled(!can_save)
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            this.save(window, cx);
                                        },
                                    )),
                                ),
                        )
                        .when(saved, |row| {
                            row.child(
                                div()
                                    .debug_selector(|| "feedback-inspector-delete".into())
                                    .child(
                                        Button::new("feedback-inspector-delete", "Delete feedback")
                                            .variant(ButtonVariant::Ghost)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if let Some(id) = this
                                                    .draft
                                                    .as_ref()
                                                    .and_then(|draft| draft.id.clone())
                                                {
                                                    this.delete(&id, window, cx);
                                                }
                                            })),
                                    ),
                            )
                        })
                        .child(
                            Button::new("feedback-inspector-done", "Done")
                                .variant(ButtonVariant::Ghost)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.close(window, cx)),
                                ),
                        ),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_muted)
                        .child(if saved && !can_save {
                            "Saved feedback remains after Reset. Delete feedback to remove it."
                        } else {
                            "Preview only · Pick another element to discard its temporary styles."
                        }),
                )
                .pb(px(8.0))
                .into_any_element(),
        )
    }
}

#[cfg(agentation_external_inspector)]
pub(super) mod external;
