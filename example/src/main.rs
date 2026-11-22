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
    muted: Rgba,
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
            muted: rgb(0x9aa0a8),
            border: rgb(0x3a3f47),
            button: rgb(0x262a31),
            button_hover: rgb(0x30353d),
            accent: rgb(0x60a5fa),
        },
        WindowAppearance::Light | WindowAppearance::VibrantLight => Palette {
            page: rgb(0xf7f8fa),
            card: rgb(0xffffff),
            text: rgb(0x20242a),
            muted: rgb(0x6b7178),
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

// A rounded card surface. The building block for the inspectable sections below.
fn card(colors: &Palette) -> Div {
    div()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(colors.border)
        .bg(colors.card)
        .flex()
        .flex_col()
        .gap_3()
}

// A compact metric: a label over a large value. Pure display, no interaction.
fn stat(colors: &Palette, label: &str, value: &str) -> impl IntoElement + use<> {
    div()
        .flex_1()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(colors.border)
        .bg(colors.card)
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(colors.muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .text_color(colors.text)
                .child(value.to_owned()),
        )
}

// A colored status pill, like the ones the annotation notes carry.
fn badge(label: &str, color: Rgba) -> impl IntoElement + use<> {
    div()
        .px_2()
        .py_1()
        .rounded_full()
        .bg(color.opacity(0.16))
        .text_color(color)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .child(label.to_owned())
}

// A styled, non-interactive text field. Shows a label and placeholder so there
// is a realistic form control to inspect without a full text-input engine.
fn field(colors: &Palette, label: &str, placeholder: &str) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(colors.muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .px_3()
                .py_2()
                .rounded_md()
                .border_1()
                .border_color(colors.border)
                .bg(colors.page)
                .text_color(colors.muted)
                .child(placeholder.to_owned()),
        )
}

// A decorative button variant. Non-interactive, present only as a distinct
// element to inspect alongside the real Increment button.
fn chip(colors: &Palette, label: &str, bg: Rgba, fg: Rgba) -> impl IntoElement + use<> {
    div()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(colors.border)
        .bg(bg)
        .text_color(fg)
        .child(label.to_owned())
}

// A decorative on/off switch: a track with a knob pushed to one side.
fn switch(colors: &Palette, on: bool) -> impl IntoElement + use<> {
    div()
        .w(px(40.))
        .h(px(24.))
        .rounded_full()
        .p(px(3.))
        .flex()
        .when(on, |track| track.justify_end().bg(colors.accent))
        .when(!on, |track| track.bg(colors.border))
        .child(div().size(px(18.)).rounded_full().bg(rgb(0xffffff)))
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
                    .gap_5()
                    .child(self.devtools.target(
                        "intro",
                        "Intro text",
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.page.title()),
                            )
                            .child(div().text_color(colors.muted).child(match self.page {
                                Page::Overview => "A plain GPUI app showing text, cards, buttons, and inputs — each one inspectable and ready to annotate.",
                                Page::Details => "This screen has its own route and annotations. Return to Overview to find its saved comments.",
                            })),
                    ))
                    .child(self.devtools.target(
                        "stats",
                        "Stats row",
                        div()
                            .flex()
                            .gap_3()
                            .child(stat(&colors, "Presses", &self.count.to_string()))
                            .child(stat(&colors, "Routes", "2"))
                            .child(stat(&colors, "Tools", "6")),
                    ))
                    .child(self.devtools.target(
                        "controls",
                        "Controls card",
                        card(&colors)
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Controls"))
                            .child(
                                div()
                                    .text_color(colors.muted)
                                    .child(format!("Pressed {} times", self.count)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(increment)
                                    .child(chip(&colors, "Secondary", colors.button, colors.text))
                                    .child(chip(&colors, "Danger", rgb(0xef4444), rgb(0xffffff))),
                            ),
                    ))
                    .child(self.devtools.target(
                        "form",
                        "Form card",
                        card(&colors)
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Form"))
                            .child(field(&colors, "Name", "Ada Lovelace"))
                            .child(field(&colors, "Email", "ada@example.com"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().child("Email me updates"))
                                    .child(switch(&colors, true)),
                            ),
                    ))
                    .child(self.devtools.target(
                        "badges",
                        "Status badges",
                        card(&colors)
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Statuses"))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(badge("Pending", colors.accent))
                                    .child(badge("Acknowledged", rgb(0xf59e0b)))
                                    .child(badge("Resolved", rgb(0x22c55e)))
                                    .child(badge("Dismissed", colors.muted)),
                            ),
                    ))
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
