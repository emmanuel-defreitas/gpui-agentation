use agentation_kit_probe::{HostMode, prepare};
use gpui::component::{button::Button, *};
use gpui::*;

struct Probe;

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child("Agentation — Kit compatibility probe")
            .child(Button::new("probe-button").label("Kit button"))
    }
}

fn main() {
    let host_initialized = std::env::args().any(|arg| arg == "--host-initialized");
    let smoke = std::env::args().any(|arg| arg == "--smoke");
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
            for title in [
                "Agentation Kit host probe",
                "Agentation Kit companion probe",
            ] {
                let (window, _) = gpui::open_window(
                    WindowOptions {
                        titlebar: Some(TitlebarOptions {
                            title: Some(title.into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    cx,
                    |_, cx| cx.new(|_| Probe),
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
            }
            cx.activate(true);
            if smoke {
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
