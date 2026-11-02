//! Drawn annotations ride along with the registered host element beneath them,
//! so a pin, box, arrow, pen stroke or textbox scrolls with the consumer's
//! content instead of staying fixed to the glass.
//!
//! Stored coordinates are window pixels at the moment a note was anchored. The
//! live displacement of its anchor element is applied only when drawing and
//! hit-testing. Before an interaction reads stored coordinates (edit, drag,
//! resize, menu) [`AnnotationOverlay::commit_scroll`] folds that displacement
//! into the stored note, so the interaction code never sees a scrolled view.
use super::{surface::viewport, *};
use crate::Annotation;

/// The registered element a drawn note follows and where it sat when anchored.
pub(super) struct Anchor {
    /// `None` when nothing registered lay under the note: it stays put.
    target: Option<String>,
    base: (f32, f32),
}

fn drawn(note: &Annotation) -> bool {
    note.feedback.shape.is_some()
}

impl AnnotationOverlay {
    /// Anchor drawn notes that have none yet and forget notes that are gone.
    ///
    /// A note anchors where it is first seen, so it must be laid out while the
    /// content still sits where the note was made. Creation freezes consumer
    /// scrolling, which holds for new notes; reopened sessions anchor on
    /// first render.
    pub(super) fn sync_anchors(&mut self, window: &Window, cx: &App) {
        let notes = &self.session.annotations;
        self.anchors
            .retain(|id, _| notes.iter().any(|note| &note.id == id && drawn(note)));
        let (w, h) = viewport(window, cx);
        for note in notes.iter().filter(|note| drawn(note)) {
            if self.anchors.contains_key(&note.id) {
                continue;
            }
            let Some(shape) = &note.feedback.shape else {
                continue;
            };
            let bounds = shape.bounds(w, h);
            let anchor = target::pick(
                window,
                cx,
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            )
            .and_then(|found| {
                target::origin(window, cx, &found.id).map(|base| Anchor {
                    target: Some(found.id),
                    base,
                })
            })
            .unwrap_or(Anchor {
                target: None,
                base: (0.0, 0.0),
            });
            self.anchors.insert(note.id.clone(), anchor);
        }
    }

    /// How far the note's anchor has moved since anchoring, or `None` when
    /// the anchor element is not on screen.
    pub(super) fn scroll_shift(
        &self,
        note: &Annotation,
        window: &Window,
        cx: &App,
    ) -> Option<(f32, f32)> {
        match self.anchors.get(&note.id) {
            Some(Anchor {
                target: Some(target),
                base,
            }) => {
                let at = target::origin(window, cx, target)?;
                Some((at.0 - base.0, at.1 - base.1))
            }
            _ => Some((0.0, 0.0)),
        }
    }

    /// The note as it appears now, or `None` while its anchor is off screen.
    pub(super) fn displayed(
        &self,
        note: &Annotation,
        window: &Window,
        cx: &App,
    ) -> Option<Annotation> {
        let shift = self.scroll_shift(note, window, cx)?;
        Some(if shift == (0.0, 0.0) {
            note.clone()
        } else {
            moved(note, shift, viewport(window, cx), false)
        })
    }

    /// Bake the displacement of every scrolled note into its stored position.
    pub(super) fn commit_scroll(&mut self, window: &Window, cx: &App) {
        let editing = self.draft.as_ref().and_then(|draft| draft.id.clone());
        let pending: Vec<_> = self
            .session
            .annotations
            .iter()
            .filter(|note| drawn(note) && editing.as_deref() != Some(note.id.as_str()))
            .filter_map(|note| {
                let shift = self.scroll_shift(note, window, cx)?;
                (shift != (0.0, 0.0)).then(|| (note.clone(), shift))
            })
            .collect();
        let size = viewport(window, cx);
        for (before, shift) in pending {
            let after = moved(&before, shift, size, true);
            match self.store.replace_note(Some(&before), Some(&after)) {
                Ok(()) => {
                    if let Some(anchor) = self.anchors.get_mut(&before.id) {
                        if let Some(target) = &anchor.target {
                            if let Some(at) = target::origin(window, cx, target) {
                                anchor.base = at;
                            }
                        }
                    }
                    if let Some(stored) = self
                        .session
                        .annotations
                        .iter_mut()
                        .find(|note| note.id == before.id)
                    {
                        *stored = after;
                    }
                }
                Err(error) => self.notice = error.to_string(),
            }
        }
    }
}

/// `note` translated by `shift` window pixels.
fn moved(note: &Annotation, shift: (f32, f32), size: (f32, f32), clamp: bool) -> Annotation {
    let (w, h) = (size.0.max(1.0), size.1.max(1.0));
    let mut moved = note.clone();
    if let Some(shape) = &note.feedback.shape {
        moved.feedback.shape = Some(shape.translated(shift.0 / w, shift.1 / h, clamp));
    }
    moved.feedback.x += shift.0 / w * 100.0;
    moved.feedback.y += shift.1;
    moved.feedback.bounding_box.x += shift.0;
    moved.feedback.bounding_box.y += shift.1;
    moved
}
