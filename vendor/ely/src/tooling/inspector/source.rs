//! Temporary Ely breadcrumb presentation for Agentation-owned source navigation.
// Relative bridge intentionally avoids build/ely.rs rewriting `crate::` into
// `crate::ely::`. Four parents reach the Agentation crate from this module.
use super::super::super::super::inspection::source::{SourceLocation, activate_source};
use crate::{
    navigation::{Breadcrumb, Crumb},
    theme::{ActiveTheme, TextSize},
};
use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, prelude::*,
};
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
    let crumbs = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let label = if index == last {
                format!("{label}:{}", source.line)
            } else {
                label
            };
            Crumb::new(index.to_string(), label)
        })
        .collect::<Vec<_>>();
    let action_notice = notice.clone();
    let message = notice.read(cx).clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .debug_selector(|| "inspector-source-breadcrumb".into())
        .child(
            Breadcrumb::new("inspector-source-path", crumbs).on_current(move |_, _, cx| {
                activate_source(
                    &source,
                    &action_notice,
                    std::env::var("EDITOR").ok().as_deref(),
                    cx,
                );
            }),
        )
        .when(!message.is_empty(), |body| {
            body.child(
                div()
                    .debug_selector(|| "inspector-source-feedback".into())
                    .text_size(cx.theme().text_size(TextSize::Xs))
                    .text_color(cx.theme().colors.fg_muted)
                    .child(message),
            )
        })
        .into_any_element()
}
