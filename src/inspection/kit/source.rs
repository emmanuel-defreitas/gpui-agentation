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
    let notice = window.use_keyed_state(
        format!("inspector-source-notice-{}", source.clipboard_path()),
        cx,
        |_, _| String::new(),
    );
    let labels: Vec<_> = source
        .relative
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
                return BreadcrumbItem::new(label);
            }
            let source = source.clone();
            let notice = notice.clone();
            BreadcrumbItem::new(format!("{label}:{}", source.line)).on_click(
                move |_, window, cx| {
                    activate_source(
                        &source,
                        &notice,
                        std::env::var("EDITOR").ok().as_deref(),
                        cx,
                    );
                    window.refresh();
                },
            )
        })
        .collect::<Vec<_>>();
    div()
        .id("inspector-source-path")
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .debug_selector(|| "inspector-source-breadcrumb".into())
        .child(Breadcrumb::new().children(items))
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
