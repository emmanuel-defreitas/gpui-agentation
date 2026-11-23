use agentation_kit_probe::{HostMode, prepare};
use gpui::component::input::{Input, InputState};
use gpui::component::{button::Button, *};
use gpui::*;

mod acceptance;

struct Probe {
    input: Entity<InputState>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child("Agentation — Kit compatibility probe")
            .child(Input::new(&self.input))
            .child(Button::new("probe-button").label("Kit button"))
    }
}

fn main() {
    let host_initialized = std::env::args().any(|arg| arg == "--host-initialized");
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    let acceptance = std::env::args().any(|arg| arg == "--acceptance");
    gpui::application()
        .with_assets(gpui::assets::Assets)
        .run(move |cx| {
            let mode = if host_initialized {
                gpui::init(cx);
                Theme::change(ThemeMode::Dark, None, cx);
                HostMode::KitInitialized
            } else {
                HostMode::Standalone
            };
            prepare(mode, cx);
            prepare(mode, cx);
            let mut windows = Vec::new();
            for title in [
                "Agentation Kit host probe",
                "Agentation Kit companion probe",
            ] {
                let (window, view) = gpui::open_window(
                    WindowOptions {
                        titlebar: Some(TitlebarOptions {
                            title: Some(title.into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    cx,
                    |window, cx| {
                        let input =
                            cx.new(|cx| InputState::new(window, cx).placeholder("Host text input"));
                        cx.new(|_| Probe { input })
                    },
                )
                .expect("open Kit probe");
                window
                    .update(cx, |_, window, cx| {
                        assert!(window.root::<gpui::base::Root>().flatten().is_some());
                        if host_initialized {
                            assert_eq!(cx.theme().mode, ThemeMode::Dark);
                        }
                    })
                    .expect("validate native root");
                windows.push((window, view));
            }
            cx.activate(true);
            if acceptance {
                acceptance::run(windows, cx);
            } else if smoke {
                cx.spawn(async |cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;
                    cx.update(|cx| {
                        assert_eq!(cx.windows().len(), 2);
                        println!(
                            "Kit native smoke passed: two Root windows, initialization preserved"
                        );
                        cx.quit();
                    });
                })
                .detach();
            }
        });
}
