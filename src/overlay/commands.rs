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

/// Build Agentation's Debug menu. Include this alongside the host's own menus.
///
/// Hosts using [`crate::Agentation::root`] receive its actions automatically.
/// Direct `AnnotationOverlay` hosts should forward actions to
/// [`AnnotationOverlay::dispatch_command`] from their root element.
pub fn debug_menu() -> gpui::Menu {
    let item = |name: &str, command: &str| {
        gpui::MenuItem::action(
            name.to_owned(),
            AnnotationCommand {
                name: command.into(),
            },
        )
    };
    gpui::Menu::new("Debug").items([
        item("Annotation mode on/off", "toggle"),
        gpui::MenuItem::separator(),
        item("Inspect · I", "inspect"),
        item("Box · B", "box"),
        item("Arrow · A", "arrow"),
        item("Pen · P", "pen"),
        item("Comment · C", "comment"),
        item("Textbox · T", "text"),
        gpui::MenuItem::submenu(gpui::Menu::new("Annotation color").items([
            item("Color 1", "color-0"),
            item("Color 2", "color-1"),
            item("Color 3", "color-2"),
            item("Color 4", "color-3"),
        ])),
        gpui::MenuItem::separator(),
        item("Save annotation", "save"),
        item("Undo annotation", "undo"),
        item("Redo annotation", "redo"),
        item("Delete annotation", "delete"),
        item("Comments", "comments"),
        item("Copy comments as Markdown", "copy"),
        item("MCP on/off", "mcp"),
        item("Settings", "settings"),
        gpui::MenuItem::submenu(gpui::Menu::new("Annotation toolbar").items([
            item("Banner", "banner"),
            item("Floating", "floating"),
            item("None", "none"),
        ])),
    ])
}

struct Bindings;
impl gpui::Global for Bindings {}

pub(super) fn init(cx: &mut App) {
    if cx.has_global::<Bindings>() {
        return;
    }
    let mut bindings = Vec::new();
    for (key, name) in [
        ("i", "inspect"),
        ("b", "box"),
        ("a", "arrow"),
        ("p", "pen"),
        ("c", "comment"),
        ("t", "text"),
        ("delete", "delete"),
        ("backspace", "delete"),
        ("ctrl-z", "undo"),
        ("cmd-z", "undo"),
        ("ctrl-shift-z", "redo"),
        ("cmd-shift-z", "redo"),
        ("ctrl-y", "redo"),
    ] {
        bindings.push(gpui::KeyBinding::new(
            key,
            AnnotationCommand { name: name.into() },
            Some("AgentationTools"),
        ));
    }
    for key in ["ctrl-s", "cmd-s"] {
        bindings.push(gpui::KeyBinding::new(
            key,
            AnnotationCommand {
                name: "save".into(),
            },
            Some("AgentationMode"),
        ));
    }
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
        match action.name.as_str() {
            "toggle" => self.toggle(window, cx),
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
            "comments" => self.open_sheet(false, window, cx),
            "settings" => self.open_sheet(true, window, cx),
            "copy" => match self.store.app_markdown(&self.session.app_name) {
                Ok(text) => cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)),
                Err(error) => self.notice = error.to_string(),
            },
            "mcp" => self.toggle_mcp(cx),
            "banner" => self.set_toolbar_position(ToolbarPosition::Banner, cx),
            "floating" => self.set_toolbar_position(ToolbarPosition::Floating, cx),
            "none" => self.set_toolbar_position(ToolbarPosition::None, cx),
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
        if !self.active || self.settings_open || self.review_open {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let editing = self.draft.as_ref().is_some_and(|d| d.editing)
            && self.input.focus_handle(cx).is_focused(window);
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
