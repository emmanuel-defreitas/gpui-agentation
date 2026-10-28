use super::*;

impl AnnotationOverlay {
    pub(super) fn record_change(
        &mut self,
        before: Option<crate::Annotation>,
        after: Option<crate::Annotation>,
    ) {
        if before == after {
            return;
        }
        self.history.push((before, after));
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.future.clear();
    }

    pub(super) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inspecting {
            self.stop_inspection(window, cx);
        }
        if !self.dismiss_note(window, cx) {
            return;
        }
        let Some((before, after)) = self.history.last().cloned() else {
            return;
        };
        match self.store.replace_note(after.as_ref(), before.as_ref()) {
            Ok(()) => {
                self.history.pop();
                self.future.push((before, after));
                self.refresh_notes();
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    pub(super) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dismiss_note(window, cx) {
            return;
        }
        let Some((before, after)) = self.future.last().cloned() else {
            return;
        };
        match self.store.replace_note(before.as_ref(), after.as_ref()) {
            Ok(()) => {
                self.future.pop();
                self.history.push((before, after));
                self.refresh_notes();
            }
            Err(error) => self.notice = error.to_string(),
        }
        cx.notify();
    }

    fn refresh_notes(&mut self) {
        match self.store.session(&self.session.id) {
            Ok(session) => {
                self.session = session;
                self.draft = None;
                self.selected_note = None;
                self.notice.clear();
            }
            Err(error) => self.notice = error.to_string(),
        }
    }
}
