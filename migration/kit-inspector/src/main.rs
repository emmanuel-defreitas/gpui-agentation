use agentation_kit_inspector::presentation::{self, RendererPolicy};
use agentation_kit_probe::{HostMode, prepare};
use gpui::component::{
    ActiveTheme,
    button::Button,
    input::{Input, InputState},
};
use gpui::*;

struct Consumer {
    input: Entity<InputState>,
}
impl Render for Consumer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(div().text_xl().child("Agentation · Kit inspector"))
            .child("Pick a card, edit its properties, then Reset. This consumer links no Ely.")
            .child(Input::new(&self.input))
            .child(
                div()
                    .id("kit-inspector-card")
                    .debug_selector(|| "kit-inspector-card".into())
                    .w(px(280.))
                    .h(px(100.))
                    .p_4()
                    .bg(rgb(0x336699))
                    .text_color(rgb(0xffffff))
                    .rounded_lg()
                    .child("Inspect this card"),
            )
            .child(
                Button::new("toggle-inspector")
                    .label("Toggle inspector")
                    .on_click(|_, window, cx| window.toggle_inspector(cx)),
            )
    }
}
fn main() {
    let host_initialized = std::env::args().any(|arg| arg == "--host-initialized");
    let from_in_host = std::env::args().any(|arg| arg == "--from-in-host");
    let external = !std::env::args().any(|arg| arg == "--in-host");
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    gpui::application().with_assets(gpui::assets::Assets).run(move |cx| {
        let mode = if host_initialized {
            gpui::init(cx);
            HostMode::KitInitialized
        } else { HostMode::Standalone };
        prepare(mode, cx);
        presentation::install(RendererPolicy::Replace, cx);
        presentation::install(RendererPolicy::Replace, cx);
        let (handle, view) = gpui::open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1100.), px(820.)), cx))),
            titlebar: Some(TitlebarOptions { title: Some("Agentation · Kit inspector".into()), ..Default::default() }),
            ..Default::default()
        }, cx, |window, cx| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Host input keeps its normal editing behavior"));
            cx.new(|_| Consumer { input })
        }).expect("open Kit inspector consumer");
        let focus = view.read(cx).input.focus_handle(cx);
        let (original_size, root_id, external_opened) = handle.update(cx, |_, window, cx| {
            window.focus(&focus, cx);
            let original_size = window.bounds().size;
            let root_id = window.root::<gpui::base::Root>().flatten().unwrap().entity_id();
            let external_opened = external && !from_in_host && presentation::companion::open(window, cx);
            if !external_opened { window.toggle_inspector(cx); }
            (original_size, root_id, external_opened)
        }).unwrap();
        cx.activate(true);
        if smoke {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
                let external_opened = if from_in_host {
                    let opened = cx.update(|cx| handle.update(cx, |_, window, cx| presentation::companion::open(window, cx)).unwrap());
                    assert!(opened, "move existing in-host inspection to companion");
                    cx.background_executor().timer(std::time::Duration::from_millis(200)).await;
                    true
                } else { external_opened };
                let first_panel = cx.update(|cx| {
                    handle.update(cx, |_, window, cx| {
                        assert_eq!(window.bounds().size, original_size, "visible host size");
                        assert_eq!(window.root::<gpui::base::Root>().flatten().unwrap().entity_id(), root_id);
                        assert_eq!(window.focused(cx), Some(focus.clone()), "host focus");
                    }).unwrap();
                    assert_eq!(cx.windows().len(), if external_opened { 2 } else { 1 });
                    let panel = cx.windows().into_iter().find(|w| *w != handle);
                    if let Some(panel) = panel {
                        panel.update(cx, |_, window, _| {
                            assert!(window.root::<gpui::base::Root>().flatten().is_some());
                            window.remove_window();
                        }).unwrap();
                    }
                    panel
                });
                cx.background_executor().timer(std::time::Duration::from_millis(150)).await;
                if external_opened {
                    cx.update(|cx| {
                        handle.update(cx, |_, window, cx| assert!(presentation::companion::open(window, cx))).unwrap();
                    });
                    cx.background_executor().timer(std::time::Duration::from_millis(400)).await;
                    cx.update(|cx| {
                        assert_eq!(cx.windows().len(), 2);
                        assert!(!cx.windows().contains(&first_panel.unwrap()), "reopened panel must be new");
                        handle.update(cx, |_, window, _| assert_eq!(window.bounds().size, original_size)).unwrap();
                    });
                }
                cx.update(|cx| { handle.update(cx, |_, window, _| window.remove_window()).unwrap(); });
                cx.background_executor().timer(std::time::Duration::from_millis(150)).await;
                cx.update(|cx| {
                    assert!(cx.windows().is_empty(), "host close must remove companion");
                    println!("Kit inspector native acceptance passed: Root, focus, size, close/reopen and cleanup");
                    cx.quit();
                });
            }).detach();
        }
    });
}
