//! Companion-owned feedback input for the external-inspector prototype.
use super::*;

pub(crate) struct ExternalFeedback {
    owner: WeakEntity<AnnotationOverlay>,
    host: gpui::AnyWindowHandle,
    input: Entity<TextInput>,
    _changes: Vec<Subscription>,
}

pub(crate) fn mount(
    owner: WeakEntity<AnnotationOverlay>,
    host: gpui::AnyWindowHandle,
    window: &mut Window,
    cx: &mut App,
) -> Entity<ExternalFeedback> {
    let view = cx.new(|cx| {
        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line(4, 8)
                .max_len(8000)
                .label("Feedback")
        });
        ExternalFeedback {
            owner: owner.clone(),
            host,
            input,
            _changes: Vec::new(),
        }
    });
    if let Some(sidebar) = cx.global_mut::<Sidebars>().0.get_mut(&host.window_id()) {
        sidebar.footer = Some(view.downgrade());
    }
    tooling::set_inspector_target(window.window_handle().window_id(), Some(host), cx);
    let weak = view.downgrade();
    cx.defer(move |cx| {
        let _ = weak.update(cx, |this, cx| this.connect(cx));
    });
    view
}

impl ExternalFeedback {
    fn connect(&mut self, cx: &mut Context<Self>) {
        let mut changes = Vec::new();
        if let Some(overlay) = self.owner.upgrade() {
            let source = overlay.read(cx).input.clone();
            let initial = source.read(cx).text().to_string();
            self.input
                .update(cx, |input, cx| input.set_text(initial, cx));
            let weak_source = source.downgrade();
            changes.push(cx.observe(&self.input, move |_, input, cx| {
                let text = input.read(cx).text().to_string();
                let _ = weak_source.update(cx, |source, cx| {
                    if source.text() != text {
                        source.set_text(text, cx);
                    }
                });
            }));
            changes.push(
                cx.observe(&source, |this: &mut ExternalFeedback, source, cx| {
                    let text = source.read(cx).text().to_string();
                    this.input.update(cx, |input, cx| {
                        if input.text() != text {
                            input.set_text(text, cx);
                        }
                    });
                    cx.notify();
                }),
            );
            changes.push(cx.observe(&overlay, |_, _, cx| cx.notify()));
        }
        self._changes = changes;
    }

    fn action(&self, save: bool, cx: &mut App) {
        let owner = self.owner.clone();
        let host = self.host;
        // Flush text before Save, even if the input's observer is still queued.
        let text = self.input.read(cx).text().to_string();
        cx.defer(move |cx| {
            let _ = host.update(cx, |_, window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    this.input.update(cx, |input, cx| input.set_text(text, cx));
                    if save {
                        this.save(window, cx);
                    } else {
                        this.close(window, cx);
                    }
                });
            });
        });
    }
}

impl Render for ExternalFeedback {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.owner.upgrade() else {
            return div();
        };
        let overlay = owner.read(cx);
        let can_save = !self.input.read(cx).text().trim().is_empty()
            || overlay.inspected_configuration.is_some();
        div()
            .debug_selector(|| "external-inspector-feedback".into())
            .flex()
            .flex_col()
            .flex_none()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(cx.theme().colors.border)
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Feedback"),
            )
            .child(
                div()
                    .text_size(cx.theme().text_size(TextSize::Xs))
                    .child("Add an optional comment. Saved feedback includes your style changes."),
            )
            .child(
                div()
                    .id("external-inspector-comment-scroll")
                    .debug_selector(|| "external-inspector-comment-scroll".into())
                    .max_h(gpui::px(
                        (f32::from(window.viewport_size().height) * 0.25).clamp(60., 160.),
                    ))
                    .overflow_y_scroll()
                    .child(Input::new(&self.input)),
            )
            .when(!overlay.notice.is_empty(), |body| {
                body.child(overlay.notice.clone())
            })
            .child(
                div()
                    .debug_selector(|| "external-inspector-feedback-actions".into())
                    .flex()
                    .flex_none()
                    .gap_2()
                    .child(
                        Button::new("external-inspector-save", "Save feedback")
                            .icon(IconName::Save)
                            .disabled(!can_save)
                            .on_click(cx.listener(|this, _, _, cx| this.action(true, cx))),
                    )
                    .child(
                        Button::new("external-inspector-done", "Done")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|this, _, _, cx| this.action(false, cx))),
                    ),
            )
    }
}
