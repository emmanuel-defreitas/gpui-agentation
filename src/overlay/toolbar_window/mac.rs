//! AppKit child-window docking. All calls run on GPUI's foreground thread.
use cocoa::{
    appkit::{NSScreen, NSWindow},
    base::{BOOL, NO, YES, id, nil},
    foundation::{NSPoint, NSRect, NSSize},
};
use gpui::{App, Pixels, Window};
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn native_window(window: &Window) -> Option<id> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // GPUI retains the view/window for the duration of this foreground borrow.
    unsafe {
        let native: id = msg_send![handle.ns_view.as_ptr() as id, window];
        (native != nil).then_some(native)
    }
}

pub(super) fn fills_screen(window: &Window) -> bool {
    let Some(window) = native_window(window) else {
        return false;
    };
    unsafe {
        let minimized: BOOL = msg_send![window, isMiniaturized];
        let zoomed: BOOL = msg_send![window, isZoomed];
        let screen: id = msg_send![window, screen];
        if minimized == YES || zoomed == YES {
            return true;
        }
        if screen == nil {
            return false;
        }
        let frame = NSWindow::frame(window);
        let visible = NSScreen::visibleFrame(screen);
        // macOS Fill need not report isZoomed. Compare the usable display area.
        frame.origin.x <= visible.origin.x + 2.0
            && frame.origin.y <= visible.origin.y + 2.0
            && frame.origin.x + frame.size.width >= visible.origin.x + visible.size.width - 2.0
            && frame.origin.y + frame.size.height >= visible.origin.y + visible.size.height - 2.0
    }
}

// AppKit frame/order calls synchronously invoke GPUI's resize/activation callbacks.
// Execute outside App's RefCell borrow, as GPUI's own native resize implementation does.
struct Placement {
    host: id,
    child: id,
    width: Pixels,
    visible: bool,
}

impl Placement {
    fn apply(&self) {
        let Self {
            host,
            child,
            width,
            visible,
        } = *self;
        unsafe {
            let parent: id = msg_send![child, parentWindow];
            if parent != host {
                if parent != nil {
                    let _: () = msg_send![parent, removeChildWindow: child];
                }
                let _: () = msg_send![host, addChildWindow: child ordered: 1isize];
                let _: () = msg_send![child, setExcludedFromWindowsMenu: YES];
            }
            let frame = NSWindow::frame(host);
            let target = NSRect::new(
                NSPoint::new(
                    frame.origin.x + frame.size.width + f64::from(super::GAP),
                    frame.origin.y,
                ),
                NSSize::new(f64::from(f32::from(width)), frame.size.height),
            );
            let current = NSWindow::frame(child);
            if current.origin.x != target.origin.x
                || current.origin.y != target.origin.y
                || current.size.width != target.size.width
                || current.size.height != target.size.height
            {
                child.setFrame_display_(target, NO);
            }
            let minimized: BOOL = msg_send![host, isMiniaturized];
            let shown: BOOL = msg_send![child, isVisible];
            let parent_shown: BOOL = msg_send![host, isVisible];
            if visible && minimized != YES && parent_shown == YES {
                if shown != YES {
                    let _: () = msg_send![child, orderFront: nil];
                }
            } else if shown == YES {
                let _: () = msg_send![child, orderOut: nil];
            }
        }
    }
}

impl Drop for Placement {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.child, release];
            let _: () = msg_send![self.host, release];
        }
    }
}

pub(super) fn place(
    host: &Window,
    toolbar: &Window,
    width: Pixels,
    visible: bool,
    cx: &App,
) -> bool {
    let (Some(parent), Some(child)) = (native_window(host), native_window(toolbar)) else {
        return false;
    };
    let host_handle = host.window_handle();
    let child_handle = toolbar.window_handle();
    let placement = unsafe {
        Placement {
            host: msg_send![parent, retain],
            child: msg_send![child, retain],
            width,
            visible,
        }
    };
    let app = cx.to_async();
    cx.foreground_executor()
        .spawn(async move {
            // A close may have won the race with this queued placement. Never revive it.
            let alive = app.update(|cx| {
                let windows = cx.windows();
                windows.contains(&host_handle) && windows.contains(&child_handle)
            });
            if alive {
                placement.apply();
            }
        })
        .detach();
    true
}
