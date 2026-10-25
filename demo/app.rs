use gpui::{prelude::*, *};
use gpui_agentation::ely::{
    buttons::Button,
    primitives::FocusScope,
    theme::{ActiveTheme, Mode, Theme},
};
use gpui_agentation::{AnnotationOverlay, begin_frame, target};

actions!(demo, [Quit]);

struct Demo {
    focus: FocusHandle,
    overlay: Entity<AnnotationOverlay>,
    count: usize,
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        begin_frame(window, cx);

        let overlay = self.overlay.clone();
        let colors = &cx.theme().colors;
        FocusScope::new(&self.focus).root().size_full().child(
            div()
                .id("demo-root")
                .capture_action(move |action: &gpui_agentation::AnnotationCommand, window, cx| {
                    overlay.update(cx, |overlay, cx| overlay.dispatch_command(action, window, cx));
                    cx.stop_propagation();
                })
                .relative()
                .size_full()
                .bg(colors.bg)
                .text_color(colors.fg)
                .flex()
                .flex_col()
                .child(self.overlay.clone())
                .child(div().flex().flex_col().flex_1().min_h_0().p_8().gap_6()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_xl().child("Annotation playground"))
                        .child(
                            Button::new("theme", "Switch appearance").on_click(|_, _, cx| {
                                let next = if cx.theme().is_dark() {
                                    Mode::Light
                                } else {
                                    Mode::Dark
                                };
                                Theme::set_mode(next, cx);
                            }),
                        ),
                )
                .child(div().text_color(colors.fg_muted).child(
                    "Turn annotations on, then choose Inspect, Box, Arrow, Pen, Comment, or Textbox. Hover a tool to see its shortcut.",
                ))
                .child(target(
                    "demo-counter",
                    "Counter",
                    div()
                        .p_6()
                        .rounded_xl()
                        .bg(colors.surface)
                        .border_1()
                        .border_color(colors.border)
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child("A real GPUI component")
                        .child(format!("Button pressed {} times", self.count))
                        .child(Button::new("increment", "Increment counter").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }),
                        )),
                ))
                .child(
                    div().flex().flex_1().gap_4().children(
                        ["Inspect", "Draw", "Review"]
                            .into_iter()
                            .zip(colors.chart)
                            .map(|(label, color)| {
                                target(
                                    format!("demo-{label}"),
                                    label,
                                    div()
                                        .flex_1()
                                        .rounded_xl()
                                        .p_6()
                                        .bg(color.opacity(0.35))
                                        .child(label),
                                )
                            }),
                    ),
                )
                ),
        )
    }
}

pub(super) fn run() {
    gpui_platform::application()
        .with_assets(gpui_agentation::Assets)
        .run(|cx| {
            gpui_agentation::ely::init(cx).expect("initialize Ely");
            cx.set_menus(vec![
                Menu::new("Agentation demo").items([MenuItem::action("Quit", Quit)]),
                gpui_agentation::debug_menu(),
            ]);
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            let _window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                            None,
                            size(px(900.), px(620.)),
                            cx,
                        ))),
                        titlebar: Some(TitlebarOptions {
                            title: Some("GPUI Agentation demo".into()),
                            ..Default::default()
                        }),
                        window_min_size: Some(size(px(640.), px(480.))),
                        ..Default::default()
                    },
                    |window, cx| {
                        cx.new(|cx| {
                            let focus = cx.focus_handle();
                            window.focus(&focus, cx);
                            let overlay = AnnotationOverlay::mount(
                                "Agentation demo",
                                "gpui://demo/home",
                                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                    .join("target/demo/feedback.json"),
                                4749,
                                window,
                                cx,
                            )
                            .expect("mount annotation overlay");
                            Demo {
                                focus,
                                overlay,
                                count: 0,
                            }
                        })
                    },
                )
                .expect("open demo window");
            #[cfg(feature = "test-support")]
            if let Some(dir) = std::env::args().nth(1) {
                export_preview(_window, std::path::PathBuf::from(dir), cx);
            }
            cx.activate(true);
        });
}

// Export actual Metal-rendered frames without recording other desktop windows.
#[cfg(feature = "test-support")]
fn export_preview(window: WindowHandle<Demo>, dir: std::path::PathBuf, cx: &mut App) {
    std::fs::create_dir_all(&dir).expect("create preview directory");
    Theme::update(cx, |theme| theme.reduced_motion = true);
    cx.spawn(async move |cx| {
        for frame in 0..4 {
            window
                .update(cx, |demo, _, cx| {
                    Theme::set_mode_now(if frame < 2 { Mode::Light } else { Mode::Dark }, cx);
                    demo.count = frame % 2;
                    cx.notify();
                })
                .expect("update preview");
            cx.background_executor()
                .timer(std::time::Duration::from_millis(800))
                .await;
            cx.update_window(window.into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                window
                    .render_to_image()
                    .expect("render preview frame")
                    .save(dir.join(format!("frame-{frame:02}.png")))
                    .expect("save preview frame");
            })
            .expect("export preview");
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
}
