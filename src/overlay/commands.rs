use super::*;
use crate::ToolbarPosition;
use crate::ely::media::Tool;

/// A Debug menu command dispatched to the active window's annotation entity.
#[derive(Clone, PartialEq, gpui::Action, serde::Deserialize)]
#[action(namespace = agentation, no_json)]
pub struct AnnotationCommand {
    /// Command identifier from [`debug_menu`], such as `toggle`, `text`, or `undo`.
    pub name: String,
}

mod menu;
pub use menu::debug_menu;

struct Bindings;
impl gpui::Global for Bindings {}

pub(super) fn shortcut(name: &str) -> Option<&'static str> {
    let mac = cfg!(target_os = "macos");
    Some(match name {
        "inspect" => "i",
        "box" => "b",
        "arrow" => "a",
        "pen" => "p",
        "comment" => "c",
        "text" => "t",
        "delete" => "delete",
        "undo" => {
            if mac {
                "cmd-z"
            } else {
                "ctrl-z"
            }
        }
        "redo" => {
            if mac {
                "cmd-shift-z"
            } else {
                "ctrl-shift-z"
            }
        }
        "save" => {
            if mac {
                "cmd-s"
            } else {
                "ctrl-s"
            }
        }
        "toggle" => {
            if mac {
                "cmd-alt-a"
            } else {
                "ctrl-alt-a"
            }
        }
        "toolbar" => {
            if mac {
                "cmd-alt-h"
            } else {
                "ctrl-alt-h"
            }
        }
        "floating" => {
            if mac {
                "cmd-alt-f"
            } else {
                "ctrl-alt-f"
            }
        }
        "banner" => {
            if mac {
                "cmd-alt-b"
            } else {
                "ctrl-alt-b"
            }
        }
        "comments" => {
            if mac {
                "cmd-alt-c"
            } else {
                "ctrl-alt-c"
            }
        }
        "settings" => {
            if mac {
                "cmd-alt-s"
            } else {
                "ctrl-alt-s"
            }
        }
        "mcp" => {
            if mac {
                "cmd-alt-m"
            } else {
                "ctrl-alt-m"
            }
        }
        "devtools" => {
            if mac {
                "cmd-alt-i"
            } else {
                "ctrl-alt-i"
            }
        }
        _ => return None,
    })
}

pub(super) fn init(cx: &mut App) {
    if cx.has_global::<Bindings>() {
        return;
    }
    let mut bindings = Vec::new();
    for name in [
        "inspect", "box", "arrow", "pen", "comment", "text", "delete", "undo", "redo", "save",
        "toggle", "toolbar", "floating", "banner", "comments", "settings", "mcp", "devtools",
    ] {
        let key = shortcut(name).expect("command shortcut");
        let context = match name {
            "save" => Some("AgentationMode"),
            "inspect" | "box" | "arrow" | "pen" | "comment" | "text" | "delete" | "undo"
            | "redo" => Some("AgentationTools"),
            _ => None,
        };
        bindings.push(gpui::KeyBinding::new(
            key,
            AnnotationCommand { name: name.into() },
            context,
        ));
        bindings.push(gpui::KeyBinding::new(
            key,
            menu::UnavailableCommand { name: name.into() },
            Some("AgentationUnavailable"),
        ));
    }
    bindings.push(gpui::KeyBinding::new(
        "backspace",
        AnnotationCommand {
            name: "delete".into(),
        },
        Some("AgentationTools"),
    ));
    cx.bind_keys(bindings);
    cx.set_global(Bindings);
}

impl AnnotationOverlay {
    /// Execute a Debug menu action in this window, preserving the host's menus.
    pub fn dispatch_command(
        &mut self,
        action: &AnnotationCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.command_enabled(&action.name, cx) {
            return;
        }
        match action.name.as_str() {
            "toggle" => self.toggle(window, cx),
            "stop" => {
                if self.inspecting { self.stop_inspection(window, cx); }
                else { self.close(window, cx); }
            },
            "resume-inspection" => inspector::resume_picking(window, cx),
            "inspect" => self.choose_tool(None, window, cx),
            "box" => self.choose_tool(Some(Tool::Box), window, cx),
            "arrow" => self.choose_tool(Some(Tool::Arrow), window, cx),
            "pen" => self.choose_tool(Some(Tool::Pen), window, cx),
            "comment" => self.choose_tool(Some(Tool::Pin), window, cx),
            // Ely's Select tool is unused by the drawing layer. It denotes our
            // plain textbox tool without modifying the pinned vendor snapshot.
            "text" => self.choose_tool(Some(Tool::Select), window, cx),
            "save" => {
                self.save(window, cx);
            }
            "undo" => self.undo(window, cx),
            "redo" => self.redo(window, cx),
            "delete" => {
                if let Some(id) = self
                    .draft
                    .as_ref()
                    .and_then(|d| d.id.clone())
                    .or(self.selected_note.clone())
                {
                    self.delete(&id, window, cx);
                }
            }
            "comments" => {
                if self.review_open {
                    self.dismiss_sheet(window, cx)
                } else {
                    self.open_sheet(false, window, cx)
                }
            }
            "settings" => {
                if settings_popup::is_open_for(window.window_handle().window_id(), cx) {
                    settings_popup::close(cx, true);
                } else if self.settings_open {
                    self.dismiss_sheet(window, cx)
                } else {
                    self.open_sheet(true, window, cx)
                }
            }
            "copy" => match self.store.app_markdown(&self.session.app_name) {
                Ok(text) => cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)),
                Err(error) => self.notice = error.to_string(),
            },
            "mcp" => self.toggle_mcp(cx),
            "devtools" => self.choose_tool(None, window, cx),
            "banner" => self.set_toolbar_banner(true, window, cx),
            "floating" => self.set_toolbar_banner(false, window, cx),
            "none" | "close-toolbar" => self.set_toolbar_position(ToolbarPosition::None, cx),
            "toolbar" => self.set_toolbar_position(
                if self.toolbar_hidden {
                    if self.banner {
                        ToolbarPosition::Banner
                    } else {
                        ToolbarPosition::Floating
                    }
                } else {
                    ToolbarPosition::None
                },
                cx,
            ),
            "report" => cx.open_url(
                "https://github.com/emmanuel-defreitas/gpui-agentation/issues/new?template=bug_report.yml",
            ),
            "star" => cx.open_url("https://github.com/emmanuel-defreitas/gpui-agentation"),
            name => {
                if let Some(color) = name
                    .strip_prefix("color-")
                    .and_then(|s| s.parse::<usize>().ok())
                    .filter(|c| *c < 4)
                {
                    self.color = color;
                }
            }
        }
        cx.notify();
    }

    // GPUI resolves host key bindings before raw key events, and an unscoped
    // host binding can outrank an ancestor context. Intercept only our reserved
    // keys, only in the owning window, before action resolution.
    pub(super) fn intercept_shortcut(
        &mut self,
        event: &gpui::KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.mention_key(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        if event.keystroke.key == "enter" && self.draft.is_none() && self.focus.is_focused(window) {
            if let Some(id) = self.selected_note.clone() {
                self.edit(&id, window, cx);
                cx.stop_propagation();
                return;
            }
        }
        if event.keystroke.key == "escape"
            && inspector::is_open(window, cx)
            && !self.settings_open
            && !self.review_open
        {
            if self.inspecting {
                inspector::resume_picking(window, cx);
            } else {
                self.close(window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if !self.active || self.settings_open || self.review_open {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let editing = self.draft.as_ref().is_some_and(|d| d.editing)
            && self.input.focus_handle(cx).is_focused(window);
        // Native property fields own their text-editing shortcuts too.
        if self.inspected_id.is_some()
            && !editing
            && !self.focus.is_focused(window)
            && !((modifiers.control || modifiers.platform) && key == "s")
        {
            return;
        }
        let name = if (modifiers.control || modifiers.platform) && !modifiers.alt {
            match key {
                "s" => Some("save"),
                "z" if !editing => Some(if modifiers.shift { "redo" } else { "undo" }),
                "y" if !editing => Some("redo"),
                _ => None,
            }
        } else if !modifiers.modified() && !editing {
            match key {
                "i" => Some("inspect"),
                "b" => Some("box"),
                "a" => Some("arrow"),
                "p" => Some("pen"),
                "c" => Some("comment"),
                "t" => Some("text"),
                "delete" | "backspace" => Some("delete"),
                _ => None,
            }
        } else {
            None
        };
        if let Some(name) = name {
            cx.stop_propagation();
            self.dispatch_command(&AnnotationCommand { name: name.into() }, window, cx);
        }
    }

    pub(super) fn shortcut_context(&self, window: &Window, cx: &App) -> &'static str {
        let editing = self.draft.as_ref().is_some_and(|d| d.editing)
            && self.input.focus_handle(cx).is_focused(window);
        if editing {
            "AgentationMode"
        } else {
            "AgentationMode AgentationTools"
        }
    }
}
