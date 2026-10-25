use super::NoteAction;
use cocoa::{
    appkit::{NSMenu, NSMenuItem},
    base::{BOOL, NO, YES, id, nil},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSString},
};
use gpui::{Pixels, Point, Window};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::OnceLock;

extern "C" fn pick(this: &mut Object, _: Sel, item: id) {
    // The target and its choice ivar live for the complete synchronous popup.
    unsafe {
        let tag: isize = msg_send![item, tag];
        this.set_ivar("choice", tag + 1);
    }
}

fn target_class() -> &'static Class {
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    CLASS.get_or_init(|| {
        let mut class = ClassDecl::new("GPUIAgentationContextTarget", class!(NSObject))
            .expect("unique menu target class");
        class.add_ivar::<isize>("choice");
        unsafe {
            class.add_method(
                sel!(agentationPick:),
                pick as extern "C" fn(&mut Object, Sel, id),
            );
        }
        class.register()
    })
}

unsafe fn string(text: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(text).autorelease() }
}

pub(super) struct Popup {
    menu: id,
    target: id,
    view: id,
    at: NSPoint,
}

impl Popup {
    pub(super) fn new(window: &Window, at: Point<Pixels>, color: usize) -> anyhow::Result<Self> {
        let handle = HasWindowHandle::window_handle(window)?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            anyhow::bail!("not an AppKit window")
        };
        // GPUI owns this NSView. Retaining it prevents a close from invalidating
        // the view before the foreground popup task has returned.
        unsafe {
            let view = handle.ns_view.as_ptr() as id;
            let view: id = msg_send![view, retain];
            let target: id = msg_send![target_class(), new];
            (*target).set_ivar("choice", 0isize);
            let menu = NSMenu::new(nil);
            menu.setAutoenablesItems(NO);
            for (tag, label) in [(0, "Edit…"), (1, "Delete")] {
                menu.addItem_(entry(label, tag, target));
            }
            menu.addItem_(NSMenuItem::separatorItem(nil));
            let colors = NSMenu::new(nil).autorelease();
            colors.setAutoenablesItems(NO);
            for (index, label) in ["Blue", "Green", "Orange", "Red"].into_iter().enumerate() {
                let item = entry(label, index as isize + 2, target);
                let _: () = msg_send![item,setState:if index==color {1isize}else{0isize}];
                colors.addItem_(item);
            }
            let parent = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(
                    string("Color"),
                    sel!(agentationPick:),
                    string(""),
                )
                .autorelease();
            parent.setSubmenu_(colors);
            menu.addItem_(parent);
            let bounds: NSRect = msg_send![view, bounds];
            let flipped: BOOL = msg_send![view, isFlipped];
            let at = NSPoint::new(
                f64::from(f32::from(at.x)),
                if flipped == YES {
                    f64::from(f32::from(at.y))
                } else {
                    bounds.size.height - f64::from(f32::from(at.y))
                },
            );
            Ok(Self {
                menu,
                target,
                view,
                at,
            })
        }
    }

    pub(super) fn open(self) -> Option<NoteAction> {
        // Called outside a GPUI App update so AppKit may run its modal loop.
        unsafe {
            let _: BOOL = msg_send![self.menu,popUpMenuPositioningItem:nil atLocation:self.at inView:self.view];
            match *(*self.target).get_ivar::<isize>("choice") {
                1 => Some(NoteAction::Edit),
                2 => Some(NoteAction::Delete),
                n @ 3..=6 => Some(NoteAction::Color((n - 3) as usize)),
                _ => None,
            }
        }
    }
}

unsafe fn entry(label: &str, tag: isize, target: id) -> id {
    unsafe {
        let item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(string(label), sel!(agentationPick:), string(""))
            .autorelease();
        item.setTarget_(target);
        let _: () = msg_send![item,setTag:tag];
        item
    }
}

impl Drop for Popup {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.menu, release];
            let _: () = msg_send![self.target, release];
            let _: () = msg_send![self.view, release];
        }
    }
}
