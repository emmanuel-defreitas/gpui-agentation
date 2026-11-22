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
#[derive(Clone)]
pub(super) struct Anchor {
    /// `None` when nothing registered lay under the note: it stays put.
    target: Option<String>,
    base: (f32, f32),
    /// Geometry of the note when anchored. Any other stored geometry (a move,
    /// a resize, an undo) is in a different frame, so the note anchors again.
    geometry: [f32; 6],
}

/// A note's stored extent and position, ignoring its text and color.
fn geometry(note: &Annotation) -> [f32; 6] {
    let bounds = note
        .feedback
        .shape
        .as_ref()
        .map_or_else(crate::Rect::default, |shape| shape.bounds(1.0, 1.0));
    [
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        note.feedback.x,
        note.feedback.y,
    ]
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
        self.anchors.retain(|id, anchor| {
            notes
                .iter()
                .any(|note| &note.id == id && drawn(note) && geometry(note) == anchor.geometry)
        });
        let (w, h) = viewport(window, cx);
        for note in notes.iter().filter(|note| drawn(note)) {
            if self.anchors.contains_key(&note.id) {
                continue;
            }
            let Some(shape) = &note.feedback.shape else {
                continue;
            };
            let bounds = shape.bounds(w, h);
            let anchor = target::nearest(
                window,
                cx,
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            )
            .and_then(|found| {
                target::origin(window, cx, &found.id).map(|base| Anchor {
                    target: Some(found.id),
                    base,
                    geometry: geometry(note),
                })
            })
            .unwrap_or(Anchor {
                target: None,
                base: (0.0, 0.0),
                geometry: geometry(note),
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
                ..
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
            moved(note, shift, viewport(window, cx))
        })
    }

    /// Anchors an undo or redo must carry across its change: those of notes
    /// whose scroll displacement is still pending.
    ///
    /// Restoring an earlier version puts stored geometry back in the frame its
    /// anchor already describes, pending displacement included, so such an
    /// anchor outlives the change. Without pending displacement the stored
    /// geometry is in the current frame and the anchor is rebuilt, so a note
    /// restored over a different target follows that target, not the one it
    /// was moved to.
    pub(super) fn anchors_of(
        &self,
        ids: &[Option<&str>],
        window: &Window,
        cx: &App,
    ) -> Vec<(String, Anchor)> {
        ids.iter()
            .flatten()
            .filter_map(|id| {
                let note = self
                    .session
                    .annotations
                    .iter()
                    .find(|note| note.id == **id && drawn(note))?;
                let shift = self.scroll_shift(note, window, cx)?;
                (shift != (0.0, 0.0)).then_some(())?;
                Some((id.to_string(), self.anchors.get(*id)?.clone()))
            })
            .collect()
    }

    /// The registered target a note follows, for tests.
    #[cfg(test)]
    pub(super) fn anchor_target(&self, id: &str) -> Option<String> {
        self.anchors.get(id)?.target.clone()
    }

    /// Put anchors saved by [`Self::anchors_of`] back for notes that still
    /// exist, matching the geometry they now have.
    pub(super) fn restore_anchors(&mut self, kept: Vec<(String, Anchor)>) {
        for (id, mut anchor) in kept {
            if let Some(note) = self
                .session
                .annotations
                .iter()
                .find(|note| note.id == id && drawn(note))
            {
                anchor.geometry = geometry(note);
                self.anchors.insert(id, anchor);
            }
        }
    }

    /// Whether the note is drawn away from its stored position, which happens
    /// while scrolling holds it partly outside the window. Moving or resizing
    /// it then would mix two coordinate frames, so those wait until it fits.
    pub(super) fn displaced(&self, id: &str, window: &Window, cx: &App) -> bool {
        self.session
            .annotations
            .iter()
            .find(|note| note.id == id && drawn(note))
            .is_some_and(|note| self.scroll_shift(note, window, cx) != Some((0.0, 0.0)))
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
            let after = moved(&before, shift, size);
            // Earlier versions of this note, kept for undo and redo, live in
            // the old frame: move them too so every version agrees.
            let snapshots: Vec<_> = self
                .history
                .iter()
                .chain(self.future.iter())
                .flat_map(|(from, to)| [from, to])
                .flatten()
                .filter(|snapshot| snapshot.id == before.id)
                .map(|snapshot| moved(snapshot, shift, size))
                .collect();
            // A shape pushed partly out of the window cannot be stored without
            // clamping, which would flatten it for good. Leave such a note
            // anchored and displaced: scrolling back restores it exactly.
            if !std::iter::once(&after).chain(snapshots.iter()).all(|note| {
                note.feedback
                    .shape
                    .as_ref()
                    .is_some_and(|shape| shape.valid())
            }) {
                continue;
            }
            match self.store.replace_note(Some(&before), Some(&after)) {
                Ok(()) => {
                    for snapshot in self
                        .history
                        .iter_mut()
                        .chain(self.future.iter_mut())
                        .flat_map(|(from, to)| [from, to])
                        .flatten()
                        .filter(|snapshot| snapshot.id == before.id)
                    {
                        *snapshot = moved(snapshot, shift, size);
                    }
                    if let Some(anchor) = self.anchors.get_mut(&before.id) {
                        anchor.geometry = geometry(&after);
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
fn moved(note: &Annotation, shift: (f32, f32), size: (f32, f32)) -> Annotation {
    let (w, h) = (size.0.max(1.0), size.1.max(1.0));
    let mut moved = note.clone();
    if let Some(shape) = &note.feedback.shape {
        moved.feedback.shape = Some(shape.translated(shift.0 / w, shift.1 / h));
    }
    moved.feedback.x += shift.0 / w * 100.0;
    moved.feedback.y += shift.1;
    moved.feedback.bounding_box.x += shift.0;
    moved.feedback.bounding_box.y += shift.1;
    moved
}
