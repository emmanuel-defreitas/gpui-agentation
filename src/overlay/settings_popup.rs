//! One menu-bar entry per process, targeting the last active consumer window.
use super::*;
use crate::ely::{
    buttons::IconButton,
    primitives::FocusScope,
    theme::{Elevation, Radius, TextSize},
};
use gpui::{
    AnyWindowHandle, Bounds, DisplayId, Global, Pixels, WeakEntity, WindowBounds, WindowId,
    WindowOptions, point, px, size,
};

#[cfg(target_os = "macos")]
#[path = "settings_popup/mac.rs"]
#[allow(deprecated, unexpected_cfgs)]
mod native;

#[cfg(target_os = "macos")]
#[path = "settings_popup/glass.rs"]
#[allow(deprecated, unexpected_cfgs)]
mod glass;

#[cfg(target_os = "macos")]
use native::ClickGuard;
#[cfg(not(target_os = "macos"))]
struct ClickGuard;
#[cfg(not(target_os = "macos"))]
impl ClickGuard {
    fn is_click_in_progress(&self) -> bool {
        false
    }
}

struct Manager(Entity<SettingsTray>);
impl Global for Manager {}
#[derive(Default)]
struct PopupOwner(Option<WindowId>);
impl Global for PopupOwner {}

#[derive(Clone, Copy)]
struct Anchor {
    button: Bounds<Pixels>,
    work_area: Bounds<Pixels>,
    display: Option<DisplayId>,
}

struct Host {
    window: AnyWindowHandle,
    owner: WeakEntity<AnnotationOverlay>,
    name: String,
    activated: u64,
}

struct SettingsTray {
    hosts: Vec<Host>,
    last_active: Option<WindowId>,
    selected: Option<WindowId>,
    popup: Option<gpui::WindowHandle<SettingsPopup>>,
    previous_focus: Option<FocusHandle>,
    restore_consumer: bool,
    generation: u64,
    activation_order: u64,
    #[cfg(target_os = "macos")]
    native: Option<native::StatusItem>,
    _closed: Subscription,
}

fn manager(cx: &mut App) -> Entity<SettingsTray> {
    if let Some(manager) = cx.try_global::<Manager>() {
        return manager.0.clone();
    }
    let entity = cx.new(|cx| {
        let weak = cx.entity().downgrade();
        let closed = cx.on_window_closed(move |cx, id| {
            let weak = weak.clone();
            cx.defer(move |cx| {
                let _ = weak.update(cx, |this: &mut SettingsTray, cx| this.window_closed(id, cx));
            });
        });
        SettingsTray {
            hosts: Vec::new(),
            last_active: None,
            selected: None,
            popup: None,
            previous_focus: None,
            restore_consumer: false,
            generation: 0,
            activation_order: 0,
            #[cfg(target_os = "macos")]
            native: None,
            _closed: closed,
        }
    });
    cx.set_global(Manager(entity.clone()));
    entity
}

pub(super) fn register(
    owner: WeakEntity<AnnotationOverlay>,
    name: String,
    window: &Window,
    cx: &mut App,
) {
    let manager = manager(cx);
    manager.update(cx, |this, cx| {
        let handle = window.window_handle();
        if this.selected == Some(handle.window_id())
            && this.hosts.iter().any(|host| {
                host.window.window_id() == handle.window_id()
                    && host.owner.entity_id() != owner.entity_id()
            })
        {
            this.close(false, cx);
        }
        this.hosts
            .retain(|host| host.window.window_id() != handle.window_id());
        let active = this.last_active.is_none() || window.is_window_active();
        let activated = if active {
            this.activation_order += 1;
            this.last_active = Some(handle.window_id());
            this.activation_order
        } else {
            0
        };
        this.hosts.push(Host {
            window: handle,
            owner,
            name,
            activated,
        });
        #[cfg(target_os = "macos")]
        if this.native.is_none() && native::supported(window) {
            match native::StatusItem::new(cx.entity().downgrade(), cx) {
                Ok(icon) => this.native = Some(icon),
                Err(error) => log::warn!("Agentation menu-bar icon unavailable: {error}"),
            }
        }
    });
}

pub(super) fn activate(id: WindowId, cx: &mut App) {
    let Some(manager) = cx.try_global::<Manager>().map(|manager| manager.0.clone()) else {
        return;
    };
    let was_open = cx
        .try_global::<PopupOwner>()
        .is_some_and(|owner| owner.0.is_some());
    cx.defer(move |cx| {
        manager.update(cx, |this, cx| {
            if let Some(host) = this
                .hosts
                .iter_mut()
                .find(|host| host.window.window_id() == id)
            {
                this.activation_order += 1;
                host.activated = this.activation_order;
                this.last_active = Some(id);
                if was_open && this.popup.is_some() {
                    this.close(false, cx);
                }
            }
        })
    });
}

pub(super) fn unregister(id: WindowId, owner: gpui::EntityId, cx: &mut App) {
    let Some(manager) = cx.try_global::<Manager>().map(|manager| manager.0.clone()) else {
        return;
    };
    cx.defer(move |cx| {
        manager.update(cx, |this, cx| {
            if this
                .hosts
                .iter()
                .any(|host| host.window.window_id() == id && host.owner.entity_id() == owner)
            {
                this.window_closed(id, cx);
            }
        })
    });
}

pub(super) fn available(cx: &App) -> bool {
    #[cfg(target_os = "macos")]
    return cx
        .try_global::<Manager>()
        .is_some_and(|manager| manager.0.read(cx).native.is_some());
    #[cfg(not(target_os = "macos"))]
    {
        let _ = cx;
        false
    }
}

pub(super) fn is_open_for(id: WindowId, cx: &App) -> bool {
    cx.try_global::<PopupOwner>()
        .is_some_and(|owner| owner.0 == Some(id))
}

pub(super) fn open_for(id: WindowId, cx: &mut App) {
    let Some(manager) = cx.try_global::<Manager>().map(|manager| manager.0.clone()) else {
        return;
    };
    cx.defer(move |cx| manager.update(cx, |this, cx| this.open(id, None, cx)));
}

pub(super) fn close(cx: &mut App, restore: bool) {
    let Some(manager) = cx.try_global::<Manager>().map(|manager| manager.0.clone()) else {
        return;
    };
    cx.defer(move |cx| manager.update(cx, |this, cx| this.close(restore, cx)));
}

fn popup_bounds(anchor: Anchor, desired: gpui::Size<Pixels>) -> Bounds<Pixels> {
    let margin = px(8.);
    let work = anchor.work_area;
    let width = desired
        .width
        .min((work.size.width - margin * 2.).max(px(1.)));
    let height = desired
        .height
        .min((work.size.height - margin * 2.).max(px(1.)));
    let left = (anchor.button.right() - width).clamp(
        work.left() + margin,
        (work.right() - width - margin).max(work.left() + margin),
    );
    let top = (anchor.button.bottom() + px(6.)).clamp(
        work.top() + margin,
        (work.bottom() - height - margin).max(work.top() + margin),
    );
    Bounds::new(point(left, top), size(width, height))
}

impl SettingsTray {
    #[cfg(any(target_os = "macos", test))]
    fn toggle(&mut self, anchor: Anchor, cx: &mut Context<Self>) {
        if self.popup.is_some() {
            self.close(true, cx);
            return;
        }
        if let Some(id) = self.last_active {
            self.open(id, Some(anchor), cx);
        }
    }

    fn open(&mut self, id: WindowId, anchor: Option<Anchor>, cx: &mut Context<Self>) {
        if self.popup.is_some() && self.selected == Some(id) {
            return;
        }
        if self.popup.is_some() {
            self.close(false, cx);
        }
        let Some(host) = self.hosts.iter().find(|host| host.window.window_id() == id) else {
            return;
        };
        let Some(owner) = host.owner.upgrade() else {
            return;
        };
        let handle = host.window;
        let name = host.name.clone();
        let Ok((focus, fallback, rem_size, display, was_active, title)) =
            handle.update(cx, |_, window, cx| {
                #[cfg(target_os = "macos")]
                let title = native::window_title(window);
                #[cfg(not(target_os = "macos"))]
                let title: Option<String> = None;
                let display = window.display(cx);
                (
                    window.focused(cx),
                    window.bounds(),
                    window.rem_size(),
                    display.map(|display| (display.id(), display.visible_bounds())),
                    window.is_window_active() || owner.read(cx).companion_toolbar_active,
                    title,
                )
            })
        else {
            return;
        };
        let name = title.unwrap_or(name);
        self.activation_order += 1;
        self.last_active = Some(id);
        if let Some(host) = self
            .hosts
            .iter_mut()
            .find(|host| host.window.window_id() == id)
        {
            host.activated = self.activation_order;
        }
        #[cfg(target_os = "macos")]
        let anchor = anchor.or_else(|| self.native.as_ref().and_then(native::StatusItem::anchor));
        let anchor = anchor.unwrap_or_else(|| {
            let work = display.as_ref().map_or(fallback, |(_, bounds)| *bounds);
            Anchor {
                button: Bounds::new(
                    point(work.right() - px(16.), work.top()),
                    size(px(0.), px(0.)),
                ),
                work_area: work,
                display: display.map(|(id, _)| id),
            }
        });
        let bounds = popup_bounds(anchor, size(px(380.), px(620.)));
        self.previous_focus = focus;
        self.restore_consumer = was_active;
        self.selected = Some(id);
        cx.default_global::<PopupOwner>().0 = Some(id);
        self.generation += 1;
        let generation = self.generation;
        let manager = cx.entity().downgrade();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            kind: gpui::WindowKind::PopUp,
            focus: true,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: anchor.display,
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };
        #[cfg(target_os = "macos")]
        let click_guard = self.native.as_ref().map(native::StatusItem::click_guard);
        #[cfg(not(target_os = "macos"))]
        let click_guard = None;
        match cx.open_window(options, |window, cx| {
            window.set_window_title("Agentation Settings");
            window.set_rem_size(rem_size);
            cx.new(|cx| {
                SettingsPopup::new(
                    owner.clone(),
                    name,
                    manager,
                    generation,
                    click_guard,
                    window,
                    cx,
                )
            })
        }) {
            Ok(popup) => {
                self.popup = Some(popup);
                #[cfg(target_os = "macos")]
                glass::install(popup, cx);
                #[cfg(target_os = "macos")]
                if let Some(native) = &mut self.native {
                    let _ = popup.update(cx, |_, window, _| native.watch(window, generation));
                }
                owner.update(cx, |_, cx| cx.notify());
            }
            Err(error) => {
                self.selected = None;
                cx.default_global::<PopupOwner>().0 = None;
                self.previous_focus = None;
                log::warn!("Could not open Agentation Settings: {error}");
            }
        }
    }

    fn close(&mut self, restore: bool, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        if let Some(native) = &mut self.native {
            native.unwatch();
        }
        let restore = restore && std::mem::take(&mut self.restore_consumer);
        let selected = self.selected.take();
        cx.default_global::<PopupOwner>().0 = None;
        let focus = self.previous_focus.take();
        if let Some(popup) = self.popup.take() {
            let _ = popup.update(cx, |_, window, _| window.remove_window());
        }
        if let Some(host) = self
            .hosts
            .iter()
            .find(|host| Some(host.window.window_id()) == selected)
        {
            let owner = host.owner.clone();
            let handle = host.window;
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, cx| {
                    if restore {
                        window.activate_window();
                        if let Some(focus) = focus {
                            window.focus(&focus, cx);
                        }
                    }
                    let _ = owner.update(cx, |this, cx| {
                        this.sync_toolbar_window(window, cx);
                        cx.notify();
                    });
                });
            });
        }
    }

    fn window_closed(&mut self, id: WindowId, cx: &mut Context<Self>) {
        if self.selected == Some(id) || self.popup.is_some_and(|popup| popup.window_id() == id) {
            self.close(false, cx);
        }
        self.hosts.retain(|host| host.window.window_id() != id);
        if self.last_active == Some(id) {
            self.last_active = self
                .hosts
                .iter()
                .max_by_key(|host| host.activated)
                .map(|host| host.window.window_id());
        }
        #[cfg(target_os = "macos")]
        if self.hosts.is_empty() {
            self.native = None;
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn dismiss_generation(&mut self, generation: u64, cx: &mut Context<Self>) {
        if self.generation == generation {
            self.close(false, cx);
        }
    }
}

struct SettingsPopup {
    #[cfg(target_os = "macos")]
    surface: Option<glass::Surface>,
    owner: WeakEntity<AnnotationOverlay>,
    name: String,
    manager: WeakEntity<SettingsTray>,
    generation: u64,
    focus: FocusHandle,
    config: Entity<TextInput>,
    copied: bool,
    expanded: bool,
    _feedback: Option<Task<()>>,
    _owner_changed: Subscription,
    _activation: Subscription,
    click_guard: Option<ClickGuard>,
}

impl SettingsPopup {
    fn new(
        owner: Entity<AnnotationOverlay>,
        name: String,
        manager: WeakEntity<SettingsTray>,
        generation: u64,
        click_guard: Option<ClickGuard>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let config = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .label("MCP config")
                .multi_line(1, usize::MAX)
                .highlighter(crate::ely::forms::json_highlights);
            let text = owner.read(cx).mcp_config_input.read(cx).text().to_string();
            input.set_text(text, cx);
            input.set_disabled(true, cx);
            input
        });
        let changed = cx.observe(&owner, |this, owner, cx| {
            let text = owner.read(cx).mcp_config_input.read(cx).text().to_string();
            this.config.update(cx, |input, cx| {
                if input.text() != text {
                    input.set_text(text, cx);
                }
            });
            cx.notify();
        });
        let activation = cx.observe_window_activation(window, |this: &mut Self, window, cx| {
            if !window.is_window_active() {
                if this
                    .click_guard
                    .as_ref()
                    .is_some_and(ClickGuard::is_click_in_progress)
                {
                    return;
                }
                this.close(false, cx);
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            #[cfg(target_os = "macos")]
            surface: None,
            owner: owner.downgrade(),
            name,
            manager,
            generation,
            focus,
            config,
            copied: false,
            expanded: false,
            _feedback: None,
            _owner_changed: changed,
            _activation: activation,
            click_guard,
        }
    }

    fn close(&self, restore: bool, cx: &mut App) {
        let manager = self.manager.clone();
        let generation = self.generation;
        cx.defer(move |cx| {
            let _ = manager.update(cx, |this, cx| {
                if this.generation == generation {
                    this.close(restore, cx);
                }
            });
        });
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            self.config.read(cx).text().to_string(),
        ));
        self.copied = true;
        self._feedback = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = owner.update(cx, |this, cx| {
                this.copied = false;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

impl Render for SettingsPopup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.owner.upgrade() else {
            return div().into_any_element();
        };
        let copy = cx.entity().downgrade();
        let toggle = copy.clone();
        let controls = super::settings::ConfigControls {
            input: self.config.clone(),
            copied: self.copied,
            expanded: self.expanded,
            copy: std::rc::Rc::new(move |cx| {
                let _ = copy.update(cx, |this, cx| this.copy(cx));
            }),
            toggle: std::rc::Rc::new(move |cx| {
                let _ = toggle.update(cx, |this, cx| {
                    this.expanded = !this.expanded;
                    cx.notify();
                });
            }),
        };
        let content =
            super::settings::settings_content(owner.read(cx), owner.clone(), controls, cx);
        #[cfg(target_os = "macos")]
        if let Some(surface) = &mut self.surface {
            surface.sync_appearance(cx.theme().is_dark(), cx);
        }
        let theme = cx.theme();
        #[cfg(target_os = "macos")]
        let (native_surface, reduce_transparency, increase_contrast) = self
            .surface
            .as_ref()
            .map_or((false, false, false), |surface| {
                let (reduce, contrast) = surface.accessibility();
                (true, reduce, contrast)
            });
        #[cfg(not(target_os = "macos"))]
        let (native_surface, reduce_transparency, increase_contrast) = (false, false, false);
        let panel = div()
            .debug_selector(|| "feedback-settings-popup".into())
            .size_full()
            .flex()
            .flex_col()
            .text_color(theme.colors.fg)
            .rounded(theme.radius(Radius::Lg))
            .when(native_surface && reduce_transparency, |panel| {
                panel.bg(theme.colors.surface.opacity(1.))
            })
            .when(!native_surface, |panel| {
                panel
                    .bg(theme.colors.surface)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow(theme.elevation(Elevation::Raised))
            })
            .when(increase_contrast, |panel| {
                panel.border_1().border_color(theme.colors.fg)
            })
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(Icon::new(crate::ely::primitives::IconName::Settings))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Settings"),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(theme.colors.fg_muted)
                                    .child(self.name.clone()),
                            ),
                    )
                    .child(
                        IconButton::new(
                            "feedback-settings-popup-close",
                            crate::ely::primitives::IconName::X,
                        )
                        .variant(crate::ely::buttons::ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip("Close Settings")
                        .on_click(cx.listener(|this, _, _, cx| this.close(true, cx))),
                    ),
            )
            .child(div().flex_1().min_h_0().w_full().child(content));
        div()
            .size_full()
            .key_context("AgentationSettingsPopup")
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close(true, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(
                cx.listener(|this, action: &commands::AnnotationCommand, _, cx| {
                    let host_owner = this.owner.clone();
                    let manager = this.manager.clone();
                    let name = action.name.clone();
                    cx.defer(move |cx| {
                        let _ = manager.update(cx, |manager, cx| {
                            let selected = manager.selected;
                            let host = manager
                                .hosts
                                .iter()
                                .find(|host| Some(host.window.window_id()) == selected)
                                .map(|host| host.window);
                            manager.close(name == "settings", cx);
                            if name != "settings" {
                                if let Some(host) = host {
                                    let _ = host.update(cx, |_, window, cx| {
                                        let _ = host_owner.update(cx, |this, cx| {
                                            this.dispatch_command(
                                                &commands::AnnotationCommand { name },
                                                window,
                                                cx,
                                            )
                                        });
                                        window.activate_window();
                                    });
                                }
                            }
                        });
                    });
                    cx.stop_propagation();
                }),
            )
            .child(
                FocusScope::new(&self.focus)
                    .root()
                    .trap()
                    .size_full()
                    .child(panel),
            )
            .into_any_element()
    }
}

#[cfg(test)]
#[path = "settings_popup.test.rs"]
mod tests;
