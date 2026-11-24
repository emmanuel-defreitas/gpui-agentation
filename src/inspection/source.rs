//! Agentation-owned source navigation, independent of UI component libraries.
//! Extracted from the locally patched previous presentation inspector; provenance is in ORIGIN.md.
use gpui::{App, ClipboardItem, Entity};
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
            column: column.max(1),
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
