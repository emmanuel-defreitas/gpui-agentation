use super::*;
use gpui::{Menu, MenuItem, OwnedMenu, OwnedMenuItem, WindowId};

// AppKit validates action availability again when opening a menu. An unhandled
// action keeps unavailable entries disabled while preserving native shortcuts.
#[derive(Clone, PartialEq, gpui::Action, serde::Deserialize)]
#[action(namespace = agentation, no_json)]
pub(super) struct UnavailableCommand {
    pub name: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MenuState {
    active: bool,
    undo: bool,
    redo: bool,
    save: bool,
    delete: bool,
    hidden: bool,
    comments: bool,
    settings: bool,
    mcp: bool,
    color: usize,
}

#[derive(Default)]
struct LastMenu(Option<(WindowId, MenuState)>);
impl gpui::Global for LastMenu {}

fn item(label: &str, command: &str, enabled: bool) -> MenuItem {
    if enabled {
        MenuItem::action(
            label.to_owned(),
            AnnotationCommand {
                name: command.into(),
            },
        )
    } else {
        MenuItem::action(
            label.to_owned(),
            UnavailableCommand {
                name: command.into(),
            },
        )
        .disabled(true)
    }
}

/// Build Agentation's native Debug menu beside the host's application menu.
///
/// Mounted overlays update this menu for the active window. Hosts using
/// [`crate::Agentation::root`] receive its commands automatically; direct
/// overlay hosts forward [`AnnotationCommand`] from their root.
pub fn debug_menu() -> Menu {
    build_menu(MenuState::default())
}

fn build_menu(state: MenuState) -> Menu {
    Menu::new("Debug").items([
        item("Undo", "undo", state.undo),
        item("Redo", "redo", state.redo),
        item("Inspect element", "devtools", true),
        MenuItem::submenu(Menu::new("New").disabled(!state.active).items([
            item("Square", "box", state.active),
            item("Arrow", "arrow", state.active),
            item("Pen", "pen", state.active),
            item("Textbox", "text", state.active),
        ])),
        MenuItem::submenu(
            Menu::new("Color").disabled(!state.active).items(
                ["Blue", "Green", "Orange", "Red"]
                    .into_iter()
                    .enumerate()
                    .map(|(color, name)| {
                        item(name, &format!("color-{color}"), state.active)
                            .checked(state.color == color)
                    }),
            ),
        ),
        MenuItem::separator(),
        item("Save feedback", "save", state.save),
        item("Delete annotation", "delete", state.delete),
        item(
            if state.hidden {
                "Show Toolbar"
            } else {
                "Hide Toolbar"
            },
            "toolbar",
            true,
        ),
        MenuItem::submenu(Menu::new("Comments").items([
            item(
                if state.comments { "Hide" } else { "Show…" },
                "comments",
                true,
            ),
            item("Copy as Markdown", "copy", true),
        ])),
        MenuItem::separator(),
        MenuItem::submenu(Menu::new("Settings").items([
            item(
                if state.settings { "Hide" } else { "Show…" },
                "settings",
                true,
            ),
            item(
                if state.mcp {
                    "Stop MCP Server"
                } else {
                    "Start MCP Server"
                },
                "mcp",
                true,
            ),
            item("Annotation Mode", "toggle", true).checked(state.active),
            MenuItem::separator(),
            item("Report an Issue…", "report", true),
            item("Star on GitHub…", "star", true),
        ])),
    ])
}

fn restore_menu(menu: OwnedMenu) -> Menu {
    Menu::new(menu.name)
        .disabled(menu.disabled)
        .items(menu.items.into_iter().map(|entry| match entry {
            OwnedMenuItem::Separator => MenuItem::Separator,
            OwnedMenuItem::Submenu(menu) => MenuItem::submenu(restore_menu(menu)),
            OwnedMenuItem::SystemMenu(menu) => MenuItem::SystemMenu(gpui::OsMenu {
                name: menu.name,
                menu_type: menu.menu_type,
            }),
            OwnedMenuItem::Action {
                name,
                action,
                os_action,
                checked,
                disabled,
            } => MenuItem::Action {
                name: name.into(),
                action,
                os_action,
                checked,
                disabled,
            },
        }))
}

fn is_agentation(menu: &OwnedMenu) -> bool {
    menu.name.as_ref() == "Debug" && menu.items.iter().any(|entry| {
        matches!(entry, OwnedMenuItem::Action { action, .. }
            if action.as_any().is::<AnnotationCommand>() || action.as_any().is::<UnavailableCommand>())
    })
}

impl AnnotationOverlay {
    pub(in crate::overlay) fn command_enabled(&self, name: &str, cx: &App) -> bool {
        match name {
            "save" => {
                self.draft.is_some()
                    && (!self.input.read(cx).text().trim().is_empty()
                        || self.inspected_configuration.is_some())
            }
            "delete" => {
                self.draft
                    .as_ref()
                    .and_then(|draft| draft.id.as_ref())
                    .is_some()
                    || self.selected_note.is_some()
            }
            "undo" => !self.history.is_empty(),
            "redo" => !self.future.is_empty(),
            "color-0" | "color-1" | "color-2" | "color-3" => self.active,
            "box" | "arrow" | "pen" | "comment" | "text" => self.active,
            _ => true,
        }
    }

    pub(in crate::overlay) fn sync_debug_menu(&self, window: WindowId, cx: &mut App) {
        if !self.companion_toolbar_active
            && !settings_popup::is_open_for(window, cx)
            && cx
                .active_window()
                .is_none_or(|active| active.window_id() != window)
        {
            return;
        }
        let state = MenuState {
            active: self.active,
            undo: !self.history.is_empty(),
            redo: !self.future.is_empty(),
            save: self.command_enabled("save", cx),
            delete: self
                .draft
                .as_ref()
                .and_then(|draft| draft.id.as_ref())
                .is_some()
                || self.selected_note.is_some(),
            hidden: self.toolbar_hidden,
            comments: self.review_open,
            settings: self.settings_open || settings_popup::is_open_for(window, cx),
            mcp: if self.mcp_starting {
                self.mcp_requested
            } else {
                self._server.as_ref().is_some_and(Server::is_mcp_enabled)
            },
            color: self.color,
        };
        if cx.has_global::<LastMenu>() && cx.global::<LastMenu>().0 == Some((window, state)) {
            return;
        }
        let Some(menus) = cx.get_menus() else { return };
        if !menus.iter().any(is_agentation) {
            return;
        }
        cx.set_menus(menus.into_iter().map(|menu| {
            if is_agentation(&menu) {
                build_menu(state)
            } else {
                restore_menu(menu)
            }
        }));
        cx.set_global(LastMenu(Some((window, state))));
    }
}

#[cfg(test)]
#[path = "menu.test.rs"]
mod tests;
