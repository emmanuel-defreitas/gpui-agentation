use super::*;

impl AnnotationOverlay {
    pub(super) fn set_toolbar_banner(
        &mut self,
        banner: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_toolbar_position(
            if banner {
                crate::ToolbarPosition::Banner
            } else {
                crate::ToolbarPosition::Floating
            },
            cx,
        );
    }

    pub(super) fn set_toolbar_position(
        &mut self,
        position: crate::ToolbarPosition,
        cx: &mut Context<Self>,
    ) {
        let preference = crate::store::ToolbarPreference {
            position,
            at: self.toolbar_at,
            last_position: if position == crate::ToolbarPosition::None {
                if self.banner {
                    crate::ToolbarPosition::Banner
                } else {
                    crate::ToolbarPosition::Floating
                }
            } else {
                position
            },
        };
        if let Err(error) = self
            .store
            .set_toolbar_preference(&self.session.app_name, preference)
        {
            self.notice = error.to_string();
            cx.notify();
            return;
        }
        if position != crate::ToolbarPosition::None {
            self.banner = position == crate::ToolbarPosition::Banner;
        }
        self.toolbar_hidden = position == crate::ToolbarPosition::None;
        self.companion_toolbar_failure = None;
        cx.notify();
    }
}

#[cfg(test)]
#[path = "banner.test.rs"]
mod tests;
