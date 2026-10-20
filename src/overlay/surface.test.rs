use super::row_action;
use gpui::{Context, FocusHandle, Modifiers, Render, TestAppContext, div, point, prelude::*, px};
use std::{cell::Cell, rc::Rc};

struct ActionRow {
    focus: FocusHandle,
    actions: Rc<Cell<usize>>,
    row_presses: Rc<Cell<usize>>,
    row_keys: Rc<Cell<usize>>,
}

impl Render for ActionRow {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        let actions = self.actions.clone();
        let row = self.row_presses.clone();
        let down = self.row_keys.clone();
        let up = self.row_keys.clone();
        div()
            .id("selectable-row")
            .size_full()
            .on_click(move |_, _, _| row.set(row.get() + 1))
            .on_key_down(move |_, _, _| down.set(down.get() + 1))
            .on_key_up(move |_, _, _| up.set(up.get() + 1))
            .child(row_action(
                div()
                    .id("row-action")
                    .size(px(32.0))
                    .track_focus(&self.focus)
                    .on_click(move |_, _, _| actions.set(actions.get() + 1)),
            ))
    }
}

#[gpui::test]
fn trailing_actions_should_keep_mouse_and_keyboard_activation_out_of_the_row(
    cx: &mut TestAppContext,
) {
    let actions = Rc::new(Cell::new(0));
    let row = Rc::new(Cell::new(0));
    let keys = Rc::new(Cell::new(0));
    let (view, cx) = cx.add_window_view(|_, cx| ActionRow {
        focus: cx.focus_handle().tab_stop(true),
        actions: actions.clone(),
        row_presses: row.clone(),
        row_keys: keys.clone(),
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(16.0), px(16.0)), Modifiers::none());
    assert_eq!(actions.get(), 1);
    assert_eq!(row.get(), 0);
    cx.update(|window, cx| {
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    for key in ["enter", "space"] {
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse(key).unwrap(),
        });
    }
    assert_eq!(actions.get(), 3);
    assert_eq!(row.get(), 0);
    assert_eq!(keys.get(), 0);
}
