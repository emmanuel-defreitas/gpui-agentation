//! Native menu-bar button and mouse-only dismissal monitors.
use super::{Anchor, SettingsTray};
use block::{ConcreteBlock, RcBlock};
use cocoa::{
    appkit::{NSEvent, NSScreen, NSStatusBar, NSStatusItem, NSVariableStatusItemLength, NSWindow},
    base::{YES, id, nil},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString},
};
use gpui::{AsyncApp, Bounds, Context, DisplayId, WeakEntity, Window, point, px, size};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{ffi::CStr, rc::Rc, sync::OnceLock};

const MOUSE_DOWN: u64 = (1 << 1) | (1 << 3) | (1 << 25);

pub(super) fn supported(window: &Window) -> bool {
    HasWindowHandle::window_handle(window)
        .is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::AppKit(_)))
}

fn native_window(window: &Window) -> Option<id> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    unsafe {
        let window: id = msg_send![handle.ns_view.as_ptr() as id, window];
        (window != nil).then_some(window)
    }
}

pub(super) fn window_title(window: &Window) -> Option<String> {
    let native = native_window(window)?;
    unsafe {
        let title: id = msg_send![native, title];
        if title == nil {
            return None;
        }
        let text = CStr::from_ptr(NSString::UTF8String(title))
            .to_string_lossy()
            .into_owned();
        (!text.is_empty()).then_some(text)
    }
}

pub(super) struct ClickGuard {
    button: id,
}
impl ClickGuard {
    pub(super) fn is_click_in_progress(&self) -> bool {
        unsafe {
            let event: id = msg_send![cocoa::appkit::NSApp(), currentEvent];
            if event == nil {
                return false;
            }
            let kind: u64 = msg_send![event, type];
            if kind != 1 && kind != 2 {
                return false;
            }
            button_rect(self.button)
                .is_some_and(|(rect, _)| contains(rect, NSEvent::mouseLocation(nil)))
        }
    }
}
impl Drop for ClickGuard {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.button, release];
        }
    }
}

unsafe fn string(text: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(text).autorelease() }
}

unsafe fn button_rect(button: id) -> Option<(NSRect, id)> {
    unsafe {
        let window: id = msg_send![button, window];
        if window == nil {
            return None;
        }
        let bounds: NSRect = msg_send![button, bounds];
        let rect: NSRect = msg_send![button, convertRect: bounds toView: nil];
        Some((window.convertRectToScreen_(rect), window))
    }
}

unsafe fn anchor(button: id) -> Option<Anchor> {
    unsafe {
        let (rect, window) = button_rect(button)?;
        let screen: id = msg_send![window, screen];
        if screen == nil {
            return None;
        }
        let frame = NSScreen::frame(screen);
        let work = NSScreen::visibleFrame(screen);
        let convert = |r: NSRect| {
            Bounds::new(
                point(
                    px((r.origin.x - frame.origin.x) as f32),
                    px((frame.size.height - (r.origin.y - frame.origin.y) - r.size.height) as f32),
                ),
                size(px(r.size.width as f32), px(r.size.height as f32)),
            )
        };
        let description: id = msg_send![screen, deviceDescription];
        let number: id = msg_send![description, objectForKey: string("NSScreenNumber")];
        let display = if number == nil {
            None
        } else {
            let raw: u64 = msg_send![number, unsignedLongLongValue];
            Some(DisplayId::new(raw))
        };
        Some(Anchor {
            button: convert(rect),
            work_area: convert(work),
            display,
        })
    }
}

struct Route {
    app: AsyncApp,
    manager: WeakEntity<SettingsTray>,
}

impl Route {
    fn toggle(&self, anchor: Anchor) {
        let manager = self.manager.clone();
        self.app
            .spawn(async move |cx| {
                cx.update(|cx| {
                    let _ = manager.update(cx, |this, cx| this.toggle(anchor, cx));
                });
            })
            .detach();
    }
    fn dismiss(&self, generation: u64) {
        let manager = self.manager.clone();
        self.app
            .spawn(async move |cx| {
                cx.update(|cx| {
                    let _ = manager.update(cx, |this, cx| this.dismiss_generation(generation, cx));
                });
            })
            .detach();
    }
}

extern "C" fn clicked(target: &Object, _: Sel, button: id) {
    unsafe {
        let route = *target.get_ivar::<*const std::ffi::c_void>("route") as *const Route;
        if let Some(anchor) = anchor(button) {
            (*route).toggle(anchor);
        }
    }
}

fn target_class() -> &'static Class {
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    CLASS.get_or_init(|| {
        let mut class = ClassDecl::new("GPUIAgentationSettingsStatusTarget", class!(NSObject))
            .expect("unique Agentation status target");
        class.add_ivar::<*const std::ffi::c_void>("route");
        unsafe {
            class.add_method(
                sel!(agentationSettings:),
                clicked as extern "C" fn(&Object, Sel, id),
            );
        }
        class.register()
    })
}

struct WatchTarget {
    window: id,
    button: id,
    route: Rc<Route>,
    generation: u64,
}

impl WatchTarget {
    fn outside(&self, event: id) {
        unsafe {
            let window: id = msg_send![event, window];
            if window == self.window {
                return;
            }
            // The status button's mouse-up action owns toggling. Do not close on
            // its mouse-down too, or that same click would reopen the popup.
            if let Some((rect, _)) = button_rect(self.button) {
                let point = NSEvent::mouseLocation(nil);
                if contains(rect, point) {
                    return;
                }
            }
            self.route.dismiss(self.generation);
        }
    }
}

fn contains(rect: NSRect, at: NSPoint) -> bool {
    at.x >= rect.origin.x
        && at.x <= rect.origin.x + rect.size.width
        && at.y >= rect.origin.y
        && at.y <= rect.origin.y + rect.size.height
}

impl Drop for WatchTarget {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.window, release];
            let _: () = msg_send![self.button, release];
        }
    }
}

struct Watch {
    local: id,
    global: id,
    _local_block: RcBlock<(id,), id>,
    _global_block: RcBlock<(id,), ()>,
}
impl Drop for Watch {
    fn drop(&mut self) {
        unsafe {
            for monitor in [self.local, self.global] {
                if monitor != nil {
                    let _: () = msg_send![class!(NSEvent), removeMonitor: monitor];
                    let _: () = msg_send![monitor, release];
                }
            }
        }
    }
}

pub(super) struct StatusItem {
    item: id,
    target: id,
    route: Rc<Route>,
    watch: Option<Watch>,
}

impl StatusItem {
    pub(super) fn new(
        manager: WeakEntity<SettingsTray>,
        cx: &mut Context<SettingsTray>,
    ) -> anyhow::Result<Self> {
        let route = Rc::new(Route {
            app: cx.to_async(),
            manager,
        });
        unsafe {
            let bytes = include_bytes!("../../assets/menubar/tray.png");
            let data: id =
                msg_send![class!(NSData), dataWithBytes: bytes.as_ptr() length: bytes.len()];
            let image: id = msg_send![class!(NSImage), alloc];
            let image: id = msg_send![image, initWithData: data];
            anyhow::ensure!(image != nil, "missing Agentation status image");
            let image = image.autorelease();
            let _: () = msg_send![image, setSize: NSSize::new(14., 14.)];
            let _: () =
                msg_send![image, setAccessibilityDescription: string("Agentation Settings")];
            let _: () = msg_send![image, setTemplate: YES];
            let item =
                NSStatusBar::systemStatusBar(nil).statusItemWithLength_(NSVariableStatusItemLength);
            let item: id = msg_send![item, retain];
            let button = item.button();
            if button == nil {
                NSStatusBar::systemStatusBar(nil).removeStatusItem_(item);
                let _: () = msg_send![item, release];
                anyhow::bail!("status item has no button");
            }
            let target: id = msg_send![target_class(), new];
            (*target).set_ivar("route", Rc::as_ptr(&route) as *const std::ffi::c_void);
            let _: () = msg_send![button, setImage: image];
            let _: () = msg_send![button, setToolTip: string("Agentation Settings")];
            let _: () = msg_send![button, setAccessibilityLabel: string("Agentation Settings")];
            let _: () = msg_send![button, setTarget: target];
            let _: () = msg_send![button, setAction: sel!(agentationSettings:)];
            Ok(Self {
                item,
                target,
                route,
                watch: None,
            })
        }
    }

    pub(super) fn click_guard(&self) -> ClickGuard {
        unsafe {
            ClickGuard {
                button: msg_send![self.item.button(), retain],
            }
        }
    }

    pub(super) fn anchor(&self) -> Option<Anchor> {
        unsafe { anchor(self.item.button()) }
    }

    pub(super) fn watch(&mut self, window: &Window, generation: u64) {
        self.unwatch();
        let Some(window) = native_window(window) else {
            return;
        };
        unsafe {
            let button = self.item.button();
            let target = Rc::new(WatchTarget {
                window: msg_send![window, retain],
                button: msg_send![button, retain],
                route: self.route.clone(),
                generation,
            });
            let local_target = target.clone();
            let local_block = ConcreteBlock::new(move |event: id| -> id {
                local_target.outside(event);
                event
            })
            .copy();
            let global_block = ConcreteBlock::new(move |event: id| {
                target.outside(event);
            })
            .copy();
            let local: id = msg_send![class!(NSEvent), addLocalMonitorForEventsMatchingMask: MOUSE_DOWN handler: &*local_block];
            let global: id = msg_send![class!(NSEvent), addGlobalMonitorForEventsMatchingMask: MOUSE_DOWN handler: &*global_block];
            self.watch = Some(Watch {
                local: if local == nil {
                    nil
                } else {
                    msg_send![local, retain]
                },
                global: if global == nil {
                    nil
                } else {
                    msg_send![global, retain]
                },
                _local_block: local_block,
                _global_block: global_block,
            });
        }
    }
    pub(super) fn unwatch(&mut self) {
        self.watch = None;
    }
}

impl Drop for StatusItem {
    fn drop(&mut self) {
        self.unwatch();
        unsafe {
            let _: () = msg_send![self.item.button(), setTarget: nil];
            NSStatusBar::systemStatusBar(nil).removeStatusItem_(self.item);
            let _: () = msg_send![self.item, release];
            let _: () = msg_send![self.target, release];
        }
    }
}
