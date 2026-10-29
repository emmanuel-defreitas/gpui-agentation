//! Source breadcrumbs and editor navigation. Display paths separately from launch paths.
use crate::{
    navigation::{Breadcrumb, Crumb},
    theme::{ActiveTheme, TextSize},
};
use gpui::{
    AnyElement, App, ClipboardItem, Entity, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, prelude::*,
};
use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceLocation {
    pub path: PathBuf,
    pub relative: PathBuf,
    pub line: u32,
    pub column: u32,
}

fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn relative_path(path: &Path, cwd: &Path) -> PathBuf {
    let target: Vec<_> = path.components().collect();
    let base: Vec<_> = cwd.components().collect();
    let common = target.iter().zip(&base).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return path.file_name().map(PathBuf::from).unwrap_or_default();
    }
    let mut relative = PathBuf::new();
    for _ in common..base.len() {
        relative.push("..");
    }
    for part in &target[common..] {
        relative.push(part.as_os_str());
    }
    relative
}

impl SourceLocation {
    pub fn new(file: &str, line: u32, column: u32, cwd: &Path) -> Self {
        let mut path = PathBuf::from(file);
        let mut mapped = false;
        if let Some(output) = option_env!("OUT_DIR") {
            if let Ok(suffix) = path.strip_prefix(Path::new(output).join("ely")) {
                let editable = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("vendor/ely/src")
                    .join(suffix);
                if editable.is_file() {
                    path = editable;
                    mapped = true;
                }
            }
        }
        if path.is_relative() {
            // Rust's own relative spans belong to the compilation package, not
            // whichever directory the executable was launched from. Fall back
            // to the host working directory for external application sources.
            let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join(&path);
            path = if compiled.is_file() {
                compiled
            } else {
                cwd.join(path)
            };
        }
        let path = normalized(&path);
        let relative = relative_path(&path, &normalized(cwd));
        Self {
            path,
            relative,
            line: line.max(1),
            column: if mapped { 1 } else { column.max(1) },
        }
    }

    pub fn clipboard_path(&self) -> String {
        format!("{}:{}:{}", self.relative.display(), self.line, self.column)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct EditorCommand {
    pub program: OsString,
    pub arguments: Vec<OsString>,
}

/// Only IDEs with known file/line/column CLI conventions are launched.
pub(crate) fn editor_command(
    editor: Option<&str>,
    source: &SourceLocation,
) -> Option<EditorCommand> {
    let words = shlex::split(
        editor
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("zed"),
    )?;
    let (program, options) = words.split_first()?;
    let executable = Path::new(program)
        .file_name()?
        .to_str()?
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    let location = format!(
        "{}:{}:{}",
        source.path.display(),
        source.line,
        source.column
    );
    let mut arguments: Vec<OsString> = options.iter().map(OsString::from).collect();
    match executable.as_str() {
        "zed" | "zeditor" => {}
        "cli" if program.ends_with("Zed.app/Contents/MacOS/cli") => {}
        "code" | "code-insiders" | "cursor" | "codium" => arguments.push("--goto".into()),
        _ => return None,
    }
    arguments.push(location.into());
    Some(EditorCommand {
        program: program.into(),
        arguments,
    })
}

fn launch(mut command: EditorCommand) -> bool {
    // LaunchServices applications may have a smaller PATH than a login shell.
    #[cfg(target_os = "macos")]
    if command.program == "zed" {
        let on_path = std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join("zed").is_file())
        });
        if !on_path {
            let bundled = Path::new("/Applications/Zed.app/Contents/MacOS/cli");
            if bundled.is_file() {
                command.program = bundled.as_os_str().to_owned();
            }
        }
    }
    Command::new(command.program)
        .args(command.arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn copy_source(source: &SourceLocation, notice: &Entity<String>, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(source.clipboard_path()));
    notice.update(cx, |message, cx| {
        *message = "Path copied to clipboard".into();
        cx.notify();
    });
}

pub(crate) fn activate_source(
    source: &SourceLocation,
    notice: &Entity<String>,
    editor: Option<&str>,
    cx: &mut App,
) {
    let command = editor_command(editor, source);
    let Some(command) = command.filter(|_| source.path.is_file()) else {
        copy_source(source, notice, cx);
        return;
    };
    notice.update(cx, |message, cx| {
        *message = "Opening in editor…".into();
        cx.notify();
    });
    let notice = notice.downgrade();
    let source = source.clone();
    cx.spawn(async move |cx| {
        let opened = cx
            .background_executor()
            .spawn(async move { launch(command) })
            .await;
        let _ = notice.update(cx, |message, cx| {
            if opened {
                *message = "Opened in editor".into();
            } else {
                cx.write_to_clipboard(ClipboardItem::new_string(source.clipboard_path()));
                *message = "Path copied to clipboard".into();
            }
            cx.notify();
        });
    })
    .detach();
}

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
