use super::*;
use crate::ely::{
    lists::ListItem,
    motion,
    primitives::{Measure, raise},
    theme::{Elevation, Radius},
};
use gpui::{AnyElement, Bounds, KeystrokeEvent, MouseButton, Pixels, Point, anchored, px};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Query {
    pub(super) start: usize,
    end: usize,
    pub(super) search: String,
    pub(super) dropdown: bool,
}

pub(super) fn query(text: &str, caret: usize) -> Option<Query> {
    let before = text.get(..caret)?;
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let word = &before[start..];
    let (search, dropdown) = if let Some(rest) = word.strip_prefix('@') {
        (rest.strip_prefix('#').unwrap_or(rest), true)
    } else {
        (word.strip_prefix('#')?, false)
    };
    if search.chars().any(|c| !c.is_alphanumeric()) {
        return None;
    }
    Some(Query {
        start,
        end: caret,
        search: search.into(),
        dropdown,
    })
}

pub(super) struct Motion {
    pub(super) from: f32,
    pub(super) to: f32,
    pub(super) since: Instant,
}
impl Motion {
    pub(super) fn sample(&self, duration: Duration) -> (f32, bool) {
        let t = if duration.is_zero() {
            1.0
        } else {
            (self.since.elapsed().as_secs_f32() / duration.as_secs_f32()).min(1.0)
        };
        (
            motion::lerp(self.from, self.to, motion::ease_out_cubic(t)),
            t < 1.0,
        )
    }
}

#[derive(Default)]
pub(super) struct Mentions {
    query: Option<Query>,
    selected: Option<usize>,
    hovered: Option<String>,
    dismissed: Option<usize>,
    escape_consumed: bool,
    pub(super) preview: Option<String>,
    motions: std::collections::HashMap<String, Motion>,
    pub bounds: Option<Bounds<Pixels>>,
}

impl AnnotationOverlay {
    pub(super) fn mention_candidates(&self) -> Vec<crate::Annotation> {
        let Some(query) = &self.mentions.query else {
            return vec![];
        };
        let current = self.draft.as_ref().and_then(|d| d.id.as_deref());
        let search = query.search.to_lowercase();
        let mut notes: Vec<_> = self
            .session
            .annotations
            .iter()
            .filter(|note| Some(note.id.as_str()) != current)
            .filter(|note| {
                matches!(
                    note.status,
                    AnnotationStatus::Pending | AnnotationStatus::Acknowledged
                )
            })
            .filter(|note| {
                note.index.to_string().starts_with(&search)
                    || note.feedback.comment.to_lowercase().contains(&search)
            })
            .cloned()
            .collect();
        // An explicit number must stay reachable even when earlier comments
        // contain the same digits and fill the six-row popup.
        notes.sort_by_key(|note| note.index.to_string() != search);
        notes.truncate(6);
        notes
    }

    fn preview_pin(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.mentions.preview == id {
            return;
        }
        let duration = motion::duration(motion::FAST, cx);
        if let Some(old) = self.mentions.preview.take() {
            let from = self
                .mentions
                .motions
                .get(&old)
                .map_or(1.35, |m| m.sample(duration).0);
            self.mentions.motions.insert(
                old,
                Motion {
                    from,
                    to: 1.0,
                    since: Instant::now(),
                },
            );
        }
        if let Some(id) = &id {
            let from = self
                .mentions
                .motions
                .get(id)
                .map_or(1.0, |m| m.sample(duration).0);
            self.mentions.motions.insert(
                id.clone(),
                Motion {
                    from,
                    to: 1.35,
                    since: Instant::now(),
                },
            );
        }
        self.mentions.preview = id;
        cx.notify();
    }

    pub(super) fn sync_mentions(&mut self, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        let next = self
            .draft
            .as_ref()
            .filter(|d| d.editing)
            .and_then(|_| query(input.text(), input.cursor()));
        if self.mentions.query != next {
            if next.as_ref().map(|q| q.start) != self.mentions.query.as_ref().map(|q| q.start) {
                self.mentions.dismissed = None;
            }
            self.mentions.query = next;
            self.mentions.selected = None;
            self.mentions.hovered = None;
            self.mentions.bounds = None;
        }
        if self
            .mentions
            .query
            .as_ref()
            .is_some_and(|q| self.mentions.dismissed == Some(q.start))
        {
            self.preview_pin(None, cx);
            return;
        }
        let exact = self
            .mentions
            .query
            .as_ref()
            .filter(|q| self.mentions.dismissed != Some(q.start))
            .and_then(|q| q.search.parse::<u64>().ok())
            .and_then(|index| {
                self.mention_candidates()
                    .into_iter()
                    .find(|n| n.index == index)
            })
            .map(|n| n.id);
        let selected = self.mentions.selected.and_then(|index| {
            self.mention_candidates()
                .get(index)
                .map(|note| note.id.clone())
        });
        self.preview_pin(self.mentions.hovered.clone().or(selected).or(exact), cx);
    }

    pub(super) fn pin_scale(&self, id: &str, window: &mut Window, cx: &App) -> f32 {
        let Some(motion) = self.mentions.motions.get(id) else {
            return 1.0;
        };
        let (scale, animating) = motion.sample(motion::duration(motion::FAST, cx));
        if animating {
            window.request_animation_frame();
        }
        scale
    }

    fn insert_mention(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(query), Some(note)) = (
            self.mentions.query.clone(),
            self.mention_candidates().get(index).cloned(),
        ) else {
            return;
        };
        self.mentions.query = None;
        self.mentions.bounds = None;
        self.mentions.hovered = None;
        self.preview_pin(None, cx);
        self.input.update(cx, |input, cx| {
            input.select(query.start..query.end, cx);
            input.insert(&format!("@#{} ", note.index), cx);
        });
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn mention_key(
        &mut self,
        event: &KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.mentions.escape_consumed = false;
        if !self.input.focus_handle(cx).is_focused(window) {
            return false;
        }
        let Some(query) = &self.mentions.query else {
            return false;
        };
        if !query.dropdown || self.mentions.dismissed == Some(query.start) {
            return false;
        }
        let notes = self.mention_candidates();
        if notes.is_empty() {
            return false;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                self.mentions.escape_consumed = true;
                self.mentions.dismissed = Some(query.start);
                self.mentions.bounds = None;
                self.mentions.selected = None;
                self.mentions.hovered = None;
                self.preview_pin(None, cx);
            }
            "down" | "up" => {
                let next = match (self.mentions.selected, event.keystroke.key.as_str()) {
                    (Some(i), "down") => (i + 1) % notes.len(),
                    (Some(i), _) => (i + notes.len() - 1) % notes.len(),
                    (None, "down") => 0,
                    (None, _) => notes.len() - 1,
                };
                self.mentions.selected = Some(next);
                self.preview_pin(Some(notes[next].id.clone()), cx);
            }
            "enter" | "tab" => self.insert_mention(self.mentions.selected.unwrap_or(0), window, cx),
            _ => return false,
        }
        cx.notify();
        true
    }

    pub(super) fn take_mention_escape(&mut self) -> bool {
        std::mem::take(&mut self.mentions.escape_consumed)
    }

    pub(super) fn mention_popup(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let query = self.mentions.query.as_ref()?;
        if !query.dropdown
            || self.mentions.dismissed == Some(query.start)
            || !self.input.focus_handle(cx).is_focused(window)
        {
            return None;
        }
        let anchor = self.input.read(cx).caret_bounds()?;
        let notes = self.mention_candidates();
        if notes.is_empty() {
            return None;
        }
        let owner = cx.entity().downgrade();
        let mut list = div()
            .id("annotation-mentions")
            .debug_selector(|| "annotation-mentions".into())
            .role(gpui::Role::ListBox)
            .aria_label("Mention a comment")
            .w(gpui::rems(16.0))
            .p_1()
            .bg(cx.theme().colors.overlay)
            .text_color(cx.theme().colors.fg)
            .border_1()
            .border_color(cx.theme().colors.border)
            .rounded(cx.theme().radius(Radius::Md))
            .shadow(cx.theme().elevation(Elevation::Floating))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            });
        for (index, note) in notes.into_iter().enumerate() {
            let id = note.id.clone();
            let hovered_id = id.clone();
            list = list.child(
                div()
                    .id(format!("mention-row-{id}"))
                    .debug_selector(move || format!("mention-note-{}", note.index))
                    .on_hover(cx.listener(move |this, hovered, _, cx| {
                        this.mentions.hovered = if *hovered {
                            Some(hovered_id.clone())
                        } else {
                            None
                        };
                        this.mentions.selected = (*hovered).then_some(index);
                        this.sync_mentions(cx);
                    }))
                    .child(
                        ListItem::new(format!("mention-{id}"), format!("Note #{}", note.index))
                            .description(note.feedback.comment.chars().take(80).collect::<String>())
                            .selected(self.mentions.selected == Some(index))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.insert_mention(index, window, cx)
                            })),
                    ),
            );
        }
        let (w, h) = (
            f32::from(window.viewport_size().width),
            f32::from(window.viewport_size().height),
        );
        let width = f32::from(gpui::rems(16.0).to_pixels(window.rem_size()));
        let at = Point::new(
            px(f32::from(anchor.left()).min((w - width - 8.0).max(8.0))),
            px(f32::from(anchor.bottom()).min((h - 220.0).max(8.0))),
        );
        Some(
            raise(
                "annotation-mention-layer",
                anchored().position(at).child(
                    Measure::new("annotation-mention-bounds", move |bounds, _, cx| {
                        let _ = owner.update(cx, |this, _| this.mentions.bounds = Some(bounds));
                    })
                    .child(list),
                ),
            )
            .into_any_element(),
        )
    }
}
