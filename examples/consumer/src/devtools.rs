//! App-local integration boundary. This module also compiles in release builds.
use gpui::{App, IntoElement, ParentElement, Styled, Window, div};

pub(super) struct Devtools {
    #[cfg(debug_assertions)]
    feedback: Option<gpui_agentation::Agentation>,
}

impl Devtools {
    pub(super) fn application(app: gpui::Application) -> gpui::Application {
        #[cfg(debug_assertions)]
        {
            app.with_assets(gpui_agentation::Assets)
        }
        #[cfg(not(debug_assertions))]
        {
            app
        }
    }

    pub(super) fn init(cx: &mut App) {
        #[cfg(debug_assertions)]
        {
            gpui_agentation::init(cx).expect("initialize Agentation");
            cx.set_menus(vec![
                gpui::Menu::new("Agentation consumer")
                    .items([gpui::MenuItem::action("Quit", crate::Quit)]),
                gpui_agentation::debug_menu(),
            ]);
        }
        #[cfg(not(debug_assertions))]
        let _ = cx;
    }

    pub(super) fn mount(window: &mut Window, cx: &mut App) -> Self {
        #[cfg(debug_assertions)]
        {
            // Choose a distinct port and store for each app. Running from any
            // working directory uses the same workspace-local ignored store.
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/consumer/feedback.json");
            let feedback = gpui_agentation::Agentation::mount(
                "Consumer example",
                "gpui://consumer/overview",
                path,
                4750,
                window,
                cx,
            )
            .map_err(|error| eprintln!("Devtools unavailable: {error}"))
            .ok();
            Self { feedback }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (window, cx);
            Self {}
        }
    }

    /// One screen-level target; individual controls need no wrappers or cfgs.
    #[track_caller]
    pub(super) fn frame(
        &self,
        id: &str,
        label: &str,
        content: impl IntoElement,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        #[cfg(debug_assertions)]
        if let Some(feedback) = &self.feedback {
            return feedback.root(id, label, content, window, cx);
        }
        let _ = (id, label, window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(content)
            .into_any_element()
    }

    pub(super) fn set_route(&self, url: &str, window: &mut Window, cx: &mut App) {
        #[cfg(debug_assertions)]
        if let Some(feedback) = &self.feedback {
            feedback.set_route(url, window, cx);
        }
        #[cfg(not(debug_assertions))]
        let _ = (url, window, cx);
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use gpui::{Context, Modifiers, Render, TestAppContext, VisualTestContext, prelude::*, px};

    struct Host {
        tools: Devtools,
        presses: usize,
    }

    impl Render for Host {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.tools.frame(
                "screen",
                "Screen",
                div().flex_1().min_h_0().child(
                    div()
                        .id("app-button")
                        .size(px(40.))
                        .debug_selector(|| "app-button".into())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.presses += 1;
                            cx.notify();
                        })),
                ),
                window,
                cx,
            )
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }

    #[gpui::test]
    fn plain_host_controls_work_without_any_ui_initialization(cx: &mut TestAppContext) {
        // Same host fallback used in release. No Agentation/Ely setup at all.
        let (host, cx) = cx.add_window_view(|_, _| crate::Consumer {
            devtools: Devtools { feedback: None },
            page: crate::Page::Overview,
            count: 0,
        });
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        cx.update(|_, cx| assert_eq!(host.read(cx).count, 1));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.simulate_keystrokes("enter");
        draw(cx);
        cx.update(|_, cx| assert!(matches!(host.read(cx).page, crate::Page::Details)));
        let navigate = cx.debug_bounds("navigate").unwrap();
        cx.simulate_click(navigate.center(), Modifiers::none());
        draw(cx);
        cx.update(|_, cx| assert!(matches!(host.read(cx).page, crate::Page::Overview)));
    }

    #[gpui::test]
    fn root_adapter_preserves_controls_and_routes_picked_comments(cx: &mut TestAppContext) {
        cx.update(gpui_agentation::init_for_tests);
        let dir = tempfile::tempdir().unwrap();
        let (host, cx) = cx.add_window_view(|window, cx| Host {
            tools: Devtools {
                feedback: Some(
                    gpui_agentation::Agentation::mount(
                        "Consumer test",
                        "gpui://consumer/overview",
                        dir.path().join("feedback.json"),
                        0,
                        window,
                        cx,
                    )
                    .unwrap(),
                ),
            },
            presses: 0,
        });
        draw(cx);
        let button = cx.debug_bounds("app-button").unwrap();
        cx.simulate_click(button.center(), Modifiers::none());
        draw(cx);
        cx.update(|_, cx| assert_eq!(host.read(cx).presses, 1));
        cx.update(|window, cx| {
            host.update(cx, |host, cx| {
                host.tools.set_route("gpui://consumer/details", window, cx)
            });
        });
        // No host FocusScope or Ely initializer: Agentation owns Tab/Enter.
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        draw(cx);
        cx.simulate_mouse_move(button.center(), None, Modifiers::none());
        draw(cx);
        assert!(cx.debug_bounds("feedback-inspector-source").is_some());
        cx.simulate_click(button.center(), Modifiers::none());
        draw(cx);
        cx.update(|_, cx| assert_eq!(host.read(cx).presses, 1));
        assert!(cx.debug_bounds("feedback-inspector-source").is_none());
        cx.simulate_input("Consumer feedback");
        draw(cx);
        let save = cx.debug_bounds("feedback-editor-primary").unwrap();
        cx.simulate_click(save.center(), Modifiers::none());
        draw(cx);
        let store = gpui_agentation::Store::open(dir.path().join("feedback.json")).unwrap();
        let notes = store.app_annotations("Consumer test").unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].url, "gpui://consumer/details");
        assert_eq!(notes[0].feedback.comment, "Consumer feedback");
        assert_eq!(notes[0].feedback.element_path, "screen");
        let comments = cx.debug_bounds("banner-notes").unwrap();
        cx.simulate_click(comments.center(), Modifiers::none());
        draw(cx);
        assert!(cx.debug_bounds("feedback-sheet-header").is_some());
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(cx.debug_bounds("feedback-sheet-header").is_none());
        cx.update(|window, cx| assert!(window.focused(cx).is_some()));
    }
}
