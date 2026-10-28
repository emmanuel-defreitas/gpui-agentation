//! AppKit owns the material; the existing GPUI view still owns all controls.
//! Native hierarchy changes run outside App's update borrow, on the main thread.
use super::SettingsPopup;
use crate::ely::theme::{ActiveTheme, Radius};
use block::{ConcreteBlock, RcBlock};
use cocoa::{
    appkit::{
        NSView, NSViewHeightSizable, NSViewWidthSizable, NSVisualEffectBlendingMode,
        NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    },
    base::{BOOL, NO, YES, id, nil},
    foundation::{NSAutoreleasePool, NSRect, NSString},
};
use gpui::{App, Window, WindowHandle};
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Owned Objective-C reference, created and released only on GPUI's foreground.
struct Owned(id);

impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

pub(super) struct Surface {
    _effect: Owned,
    appearance: (bool, bool),
    center: Owned,
    observer: Owned,
    _changed: RcBlock<(id,), ()>,
}

impl Surface {
    pub(super) fn accessibility(&self) -> (bool, bool) {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let reduce: BOOL = msg_send![workspace, accessibilityDisplayShouldReduceTransparency];
            let contrast: BOOL = msg_send![workspace, accessibilityDisplayShouldIncreaseContrast];
            (reduce == YES, contrast == YES)
        }
    }

    pub(super) fn sync_appearance(&mut self, dark: bool, window: &Window, cx: &App) {
        let appearance = (dark, self.accessibility().1);
        if self.appearance == appearance {
            return;
        }
        self.appearance = appearance;
        let effect = Owned(unsafe { msg_send![self._effect.0, retain] });
        let popup = window.window_handle();
        let app = cx.to_async();
        // GPUIView's appearance callback enters GPUI. As with installation,
        // change the native appearance after leaving the App update borrow.
        cx.foreground_executor()
            .spawn(async move {
                if app.update(|cx| cx.windows().contains(&popup)) {
                    unsafe { set_appearance(effect.0, appearance) }
                }
            })
            .detach();
    }
}

unsafe fn set_appearance(view: id, (dark, contrast): (bool, bool)) {
    unsafe {
        let name = match (dark, contrast) {
            (false, false) => "NSAppearanceNameAqua",
            (true, false) => "NSAppearanceNameDarkAqua",
            (false, true) => "NSAppearanceNameAccessibilityHighContrastAqua",
            (true, true) => "NSAppearanceNameAccessibilityHighContrastDarkAqua",
        };
        let name = NSString::alloc(nil).init_str(name).autorelease();
        let appearance: id = msg_send![class!(NSAppearance), appearanceNamed: name];
        let _: () = msg_send![view, setAppearance: appearance];
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.center.0, removeObserver: self.observer.0];
        }
        // The native parent owns the effect until window teardown. Do not detach
        // GPUI's rendering view while its platform window is being destroyed.
    }
}

struct Pending {
    view: Owned,
    parent: Owned,
    window: Owned,
    radius: f64,
    dark: bool,
}

impl Pending {
    unsafe fn apply(self, changed: RcBlock<(id,), ()>) -> Option<Surface> {
        unsafe {
            let frame: NSRect = msg_send![self.parent.0, bounds];
            let effect: id = msg_send![class!(NSVisualEffectView), alloc];
            let effect: id = msg_send![effect, initWithFrame: frame];
            if effect == nil {
                return None;
            }
            let effect = Owned(effect);
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let contrast: BOOL = msg_send![workspace, accessibilityDisplayShouldIncreaseContrast];
            let appearance = (self.dark, contrast == YES);
            set_appearance(effect.0, appearance);
            let mask = NSViewWidthSizable | NSViewHeightSizable;
            effect.0.setAutoresizingMask_(mask);
            let responder: id = msg_send![self.window.0, firstResponder];
            let responder = Owned(msg_send![responder, retain]);

            // The stock popover material supplies the conventional frosted
            // appearance on all supported macOS versions, including Tahoe.
            effect.0.setMaterial_(NSVisualEffectMaterial::Popover);
            effect
                .0
                .setBlendingMode_(NSVisualEffectBlendingMode::BehindWindow);
            effect
                .0
                .setState_(NSVisualEffectState::FollowsWindowActiveState);
            effect.0.addSubview_(self.view.0);
            effect.0.setWantsLayer(YES);
            let layer: id = msg_send![effect.0, layer];
            let _: () = msg_send![layer, setCornerRadius: self.radius];
            let _: () = msg_send![layer, setMasksToBounds: YES];
            self.parent.0.addSubview_(effect.0);
            let bounds: NSRect = msg_send![effect.0, bounds];
            let _: () = msg_send![self.view.0, setFrame: bounds];
            self.view.0.setAutoresizingMask_(mask);
            // Reparenting may clear AppKit's first responder. GPUI focus handles
            // and the NSWindow identity are unchanged.
            let _: BOOL = msg_send![self.window.0, makeFirstResponder: responder.0];
            let _: () = msg_send![self.window.0, setHasShadow: NO];
            let clear: id = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![self.window.0, setBackgroundColor: clear];
            let _: () = msg_send![self.window.0, invalidateShadow];

            let center: id = msg_send![workspace, notificationCenter];
            let queue: id = msg_send![class!(NSOperationQueue), mainQueue];
            let name = NSString::alloc(nil)
                .init_str("NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification")
                .autorelease();
            let observer: id = msg_send![center, addObserverForName: name
                object: nil queue: queue usingBlock: &*changed];
            Some(Surface {
                _effect: effect,
                appearance,
                center: Owned(msg_send![center, retain]),
                observer: Owned(msg_send![observer, retain]),
                _changed: changed,
            })
        }
    }
}

pub(super) fn install(popup: WindowHandle<SettingsPopup>, cx: &mut App) {
    let Ok(Some(pending)) = popup.update(cx, |_, window, cx| {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        unsafe {
            let view = handle.ns_view.as_ptr() as id;
            let parent: id = msg_send![view, superview];
            let native: id = msg_send![view, window];
            if parent == nil || native == nil {
                return None;
            }
            Some(Pending {
                view: Owned(msg_send![view, retain]),
                parent: Owned(msg_send![parent, retain]),
                window: Owned(msg_send![native, retain]),
                radius: f64::from(f32::from(
                    cx.theme().radius(Radius::Xl).to_pixels(window.rem_size()),
                )),
                dark: cx.theme().is_dark(),
            })
        }
    }) else {
        // Test/non-AppKit windows keep the existing opaque GPUI panel.
        return;
    };
    let app = cx.to_async();
    cx.foreground_executor()
        .spawn(async move {
            if !app.update(|cx| cx.windows().contains(&popup.into())) {
                return;
            }
            let notify_app = app.clone();
            let changed = ConcreteBlock::new(move |_: id| {
                notify_app
                    .spawn(async move |cx| {
                        let _ = popup.update(cx, |_, _, cx| cx.notify());
                    })
                    .detach();
            })
            .copy();
            let surface = unsafe { pending.apply(changed) };
            app.update(|cx| {
                let _ = popup.update(cx, |this, _, cx| {
                    this.surface = surface;
                    cx.notify();
                });
            });
        })
        .detach();
}
