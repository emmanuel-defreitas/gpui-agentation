use gpui::{App, Application, Context, Window, WindowOptions, div, prelude::*, px};

#[cfg(debug_assertions)]
use gpui::{Entity, FocusHandle};
#[cfg(debug_assertions)]
use gpui_agentation::{AnnotationOverlay, begin_frame, ely, target};

struct Demo {
    #[cfg(debug_assertions)]
    feedback: Entity<AnnotationOverlay>,
    #[cfg(debug_assertions)]
    focus: FocusHandle,
}

impl Demo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        #[cfg(debug_assertions)]
        let focus = {
            let handle = cx.focus_handle();
            window.focus(&handle, cx);
            handle
        };
        #[cfg(not(debug_assertions))]
        let _ = (window, cx);
        Self {
            #[cfg(debug_assertions)]
            feedback: AnnotationOverlay::mount(
                "Consumer demo",
                "gpui://consumer/home",
                "target/agentation/feedback.json",
                4748,
                window,
                cx,
            )
            .expect("open annotation store"),
            #[cfg(debug_assertions)]
            focus,
        }
    }
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(debug_assertions)]
        begin_frame(window, cx);
        #[cfg(not(debug_assertions))]
        let _ = (window, cx);

        let content = div().p_8().child("Your native app, ready for feedback.");
        // Register meaningful component roots, rather than every widget.
        #[cfg(debug_assertions)]
        let content = target("welcome", "Welcome panel", content);

        let root = div()
            .relative()
            .size_full()
            .bg(gpui::rgb(0xf5f4f0))
            .text_color(gpui::rgb(0x242422))
            .text_size(px(22.))
            .child(content);
        #[cfg(debug_assertions)]
        let root = ely::primitives::FocusScope::new(&self.focus)
            .root()
            .size_full()
            .child(root.child(self.feedback.clone()));
        root
    }
}

fn main() {
    let application: Application = gpui_platform::application();
    #[cfg(debug_assertions)]
    let application = application.with_assets(ely::Assets);
    application.run(|cx: &mut App| {
        #[cfg(debug_assertions)]
        ely::init(cx).expect("initialize bundled annotation UI");
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| Demo::new(window, cx))
        })
        .expect("open window");
        cx.activate(true);
    });
}
