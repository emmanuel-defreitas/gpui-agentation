mod devtools;

use devtools::Devtools;
use gpui::{prelude::*, *};

actions!(consumer, [Quit, Increment, Navigate]);

#[derive(Clone, Copy)]
enum Page {
    Overview,
    Details,
}

impl Page {
    fn route(self) -> &'static str {
        match self {
            Self::Overview => "gpui://consumer/overview",
            Self::Details => "gpui://consumer/details",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Details => "Details",
        }
    }
}

// The host UI follows the operating system's light or dark setting. There is no
// manual toggle; `observe_window_appearance` re-renders when the OS changes.
struct Palette {
    page: Rgba,
    card: Rgba,
    text: Rgba,
    border: Rgba,
    button: Rgba,
    button_hover: Rgba,
    accent: Rgba,
}

fn palette(window: &Window) -> Palette {
    match window.appearance() {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Palette {
            page: rgb(0x15171b),
            card: rgb(0x1e2127),
            text: rgb(0xe6e8eb),
            border: rgb(0x3a3f47),
            button: rgb(0x262a31),
            button_hover: rgb(0x30353d),
            accent: rgb(0x60a5fa),
        },
        WindowAppearance::Light | WindowAppearance::VibrantLight => Palette {
            page: rgb(0xf7f8fa),
            card: rgb(0xffffff),
            text: rgb(0x20242a),
            border: rgb(0xc8cbd0),
            button: rgb(0xffffff),
            button_hover: rgb(0xf1f3f5),
            accent: rgb(0x2563eb),
        },
    }
}

// Ordinary host UI built with GPUI. Agentation owns its own UI dependencies.
#[derive(Clone, Copy)]
enum Command {
    Increment,
    Navigate,
}

impl Command {
    fn dispatch(self, window: &mut Window, cx: &mut App) {
        match self {
            Self::Increment => window.dispatch_action(Box::new(Increment), cx),
            Self::Navigate => window.dispatch_action(Box::new(Navigate), cx),
        }
    }
}

fn button(
    id: &'static str,
    label: &str,
    command: Command,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let focus = window
        .use_keyed_state(id, cx, |_, cx| {
            cx.focus_handle().tab_index(0).tab_stop(true)
        })
        .read(cx)
        .clone();
    let click_focus = focus.clone();
    let colors = palette(window);
    if window.focused(cx).is_none() {
        window.focus(&focus, cx);
    }
    div()
        .id(id)
        .debug_selector(move || id.into())
        .role(Role::Button)
        .aria_label(label)
        .track_focus(&focus)
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(colors.border)
        .bg(colors.button)
        .text_color(colors.text)
        .hover(|style| style.bg(colors.button_hover))
        .focus(|style| style.border_color(colors.accent))
        .on_click(move |_, window, cx| {
            window.focus(&click_focus, cx);
            command.dispatch(window, cx);
        })
        .on_key_down(move |event, window, cx| {
            let mut modifiers = event.keystroke.modifiers;
            modifiers.shift = false;
            if event.keystroke.key == "tab" && !modifiers.modified() {
                if event.keystroke.modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                cx.stop_propagation();
                return;
            }
            if matches!(event.keystroke.key.as_str(), "enter" | "space")
                && !event.keystroke.modifiers.modified()
            {
                command.dispatch(window, cx);
                cx.stop_propagation();
            }
        })
        .child(label.to_owned())
}

struct Consumer {
    devtools: Devtools,
    page: Page,
    count: usize,
    _appearance: Subscription,
}

impl Render for Consumer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = palette(window);
        let increment = button("increment", "Increment", Command::Increment, window, cx);
        let navigate = button(
            "navigate",
            match self.page {
                Page::Overview => "Open details",
                Page::Details => "Back to overview",
            },
            Command::Navigate,
            window,
            cx,
        );
        let content = div()
            .id("page-scroll")
            .key_context("Consumer")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .bg(colors.page)
            .text_color(colors.text)
            .font_family(".SystemUIFont")
            .on_action(cx.listener(|this, _: &Increment, _, cx| {
                this.count += 1;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Navigate, window, cx| {
                this.page = match this.page {
                    Page::Overview => Page::Details,
                    Page::Details => Page::Overview,
                };
                this.devtools.set_route(this.page.route(), window, cx);
                cx.notify();
            }))
            .child(
                div()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).child(self.page.title()))
                    .child(match self.page {
                        Page::Overview => "A plain GPUI app with one inspectable screen. No Ely dependency or per-control annotation wrappers.",
                        Page::Details => "This screen has its own route and annotations. Return to Overview to find its saved comments.",
                    })
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .bg(colors.card)
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Counter"))
                            .child(format!("Pressed {} times", self.count))
                            .child(increment),
                    )
                    .child(navigate),
            );
        self.devtools
            .frame(self.page.route(), self.page.title(), content, window, cx)
    }
}

fn main() {
    Devtools::application(gpui_platform::application()).run(|cx| {
        Devtools::init(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(900.), px(620.)),
                    cx,
                ))),
                window_min_size: Some(size(px(640.), px(480.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Consumer setup example".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| Consumer {
                    devtools: Devtools::mount(window, cx),
                    page: Page::Overview,
                    count: 0,
                    _appearance: cx.observe_window_appearance(window, |_, _, cx| cx.notify()),
                })
            },
        )
        .expect("open consumer window");
        cx.activate(true);
    });
}
