//! Comments use the same companion surface and width transition as inspection.
use super::*;
use crate::ely::buttons::{ButtonVariant, IconButton};

pub(super) struct Comments {
    owner: WeakEntity<AnnotationOverlay>,
    _changes: Option<Subscription>,
}

pub(super) fn mount(
    owner: WeakEntity<AnnotationOverlay>,
    _host: AnyWindowHandle,
    cx: &mut App,
) -> Entity<Comments> {
    cx.new(|cx| {
        let changes = owner
            .upgrade()
            .map(|owner| cx.observe(&owner, |_, _, cx| cx.notify()));
        Comments {
            owner,
            _changes: changes,
        }
    })
}

impl Render for Comments {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.owner.upgrade() else {
            return div().into_any_element();
        };
        let close_owner = owner.clone();
        let close: std::rc::Rc<dyn Fn(&mut Window, &mut App)> =
            std::rc::Rc::new(move |window, cx| {
                surface::consumer_action(close_owner.clone(), window, cx, |this, window, cx| {
                    this.dismiss_sheet(window, cx)
                });
            });
        let overlay = owner.read(cx);
        let body = surface::notes_body(overlay, owner.clone(), close.clone(), true, window, cx);
        div()
            .debug_selector(|| "companion-comments-panel".into())
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .border_l_1()
            .border_color(cx.theme().colors.border)
            .child(
                div()
                    .debug_selector(|| "feedback-sheet-header".into())
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .flex_none()
                    .border_b_1()
                    .border_color(cx.theme().colors.border)
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Comments"),
                    )
                    .child(
                        IconButton::new("companion-comments-close", IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .tooltip("Close comments")
                            .on_click(move |_, window, cx| close(window, cx)),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "feedback-sheet-body".into())
                    .flex_1()
                    .min_h_0()
                    .child(body),
            )
            .into_any_element()
    }
}
