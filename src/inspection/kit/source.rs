//! Kit breadcrumb presentation delegates activation to the owned source service.
use super::*;
use crate::inspection::source::{SourceLocation, activate_source};
use gpui::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use std::path::PathBuf;

pub(super) fn breadcrumb(
    location: &'static std::panic::Location<'static>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let source = SourceLocation::new(location.file(), location.line(), location.column(), &cwd);
    render_source(source, window, cx)
}

fn render_source(source: SourceLocation, window: &mut Window, cx: &mut App) -> AnyElement {
    let notice = window.use_keyed_state(
        format!("inspector-source-notice-{}", source.clipboard_path()),
        cx,
        |_, _| String::new(),
    );
    let labels: Vec<_> = source
        .display_path()
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let last = labels.len().saturating_sub(1);
    let message = notice.read(cx).clone();
    let items = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            if index != last {
                return BreadcrumbItem::new(label).flex_shrink_0().max_w_full();
            }
            let source = source.clone();
            let notice = notice.clone();
            BreadcrumbItem::new(format!("{label}:{}", source.line))
                .flex_shrink_0()
                .max_w_full()
                .on_click(move |_, window, cx| {
                    activate_source(
                        &source,
                        &notice,
                        std::env::var("EDITOR").ok().as_deref(),
                        cx,
                    );
                    window.refresh();
                })
        })
        .collect::<Vec<_>>();
    div()
        .id("inspector-source-path")
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .min_w_0()
        .debug_selector(|| "inspector-source-breadcrumb".into())
        .child(
            Breadcrumb::new()
                .w_full()
                .min_w_0()
                .flex_wrap()
                .children(items),
        )
        .when(!message.is_empty(), |body| {
            body.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(message),
            )
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, Render, TestAppContext};

    struct Sample(f32, bool);
    impl Render for Sample {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let source = SourceLocation {
                path: "/nonexistent-agentation-source/example/src/main.rs".into(),
                relative: if self.1 {
                    format!("example/src/{}.rs", "long_filename_".repeat(8)).into()
                } else {
                    "example/src/main.rs".into()
                },
                line: 42,
                column: 7,
            };
            div().w(px(self.0)).child(render_source(source, window, cx))
        }
    }

    #[gpui::test]
    fn source_breadcrumb_wraps_and_keeps_file_clickable(cx: &mut TestAppContext) {
        cx.update(crate::integration::init_for_tests);
        let (sample, cx) = cx.add_window_view(|_, _| Sample(600., false));
        cx.run_until_parked();
        let wide = cx.debug_bounds("inspector-source-breadcrumb").unwrap();
        cx.update(|_, cx| {
            sample.update(cx, |sample, cx| {
                sample.0 = 140.;
                cx.notify();
            })
        });
        cx.run_until_parked();
        let narrow = cx.debug_bounds("inspector-source-breadcrumb").unwrap();
        assert!(
            narrow.size.height > wide.size.height,
            "path should wrap in a narrow inspector"
        );
        assert!(narrow.size.width <= px(140.));
        // The file wraps to the start of the second line. Click that line to
        // verify its editor/copy action survives wrapping.
        cx.simulate_click(
            point(
                narrow.left() + px(10.),
                narrow.bottom() - wide.size.height / 2.,
            ),
            Modifiers::none(),
        );
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().unwrap(),
                "example/src/main.rs:42:7"
            )
        });
        cx.update(|_, cx| {
            sample.update(cx, |sample, cx| {
                sample.1 = true;
                cx.notify();
            })
        });
        cx.run_until_parked();
        let long = cx.debug_bounds("inspector-source-breadcrumb").unwrap();
        assert!(
            long.size.height > narrow.size.height,
            "a long filename wraps within the available width"
        );
    }
}
