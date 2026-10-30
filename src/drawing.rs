use crate::Rect;
use crate::ely::media::{Mark, Tool};
use gpui::{PathBuilder, Window, point, px};
use serde::{Deserialize, Serialize};

/// A serializable live-surface mark in Ely ImageAnnotator's normalized coordinate model.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnnotationShape {
    /// Rectangle, including a drag in reverse direction.
    Box {
        /// First normalized corner.
        from: (f32, f32),
        /// Opposite normalized corner.
        to: (f32, f32),
        /// Ely chart color, zero through three.
        color: usize,
    },
    /// Directed arrow.
    Arrow {
        /// Normalized tail.
        from: (f32, f32),
        /// Normalized tip.
        to: (f32, f32),
        /// Ely chart color, zero through three.
        color: usize,
    },
    /// Freehand polyline.
    Pen {
        /// Normalized points, at most 4,096.
        points: Vec<(f32, f32)>,
        /// Ely chart color, zero through three.
        color: usize,
    },
    /// Plain text at a normalized position, rendered at one fixed size.
    Text {
        /// Normalized top-left corner.
        at: (f32, f32),
        /// Plain text, at most 8,000 characters.
        text: String,
        /// Ely chart color, zero through three.
        color: usize,
        /// Normalized wrap width. `None` sizes the box to its content.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f32>,
    },
    /// Point marker.
    Pin {
        /// Normalized position.
        at: (f32, f32),
        /// Ely chart color, zero through three.
        color: usize,
    },
}

impl AnnotationShape {
    pub(crate) fn from_stroke(tool: Tool, points: &[(f32, f32)], color: usize) -> Option<Self> {
        let (&first, &last) = (points.first()?, points.last()?);
        match tool {
            Tool::Select => None,
            Tool::Pin => Some(Self::Pin { at: last, color }),
            Tool::Box if first != last => Some(Self::Box {
                from: first,
                to: last,
                color,
            }),
            Tool::Arrow if first != last => Some(Self::Arrow {
                from: first,
                to: last,
                color,
            }),
            Tool::Pen if points.iter().any(|p| *p != first) => Some(Self::Pen {
                points: points.to_vec(),
                color,
            }),
            _ => None,
        }
    }

    pub(crate) fn mark(&self) -> Mark {
        match self {
            Self::Box { from, to, color } => Mark::Box {
                from: *from,
                to: *to,
                color: *color,
            },
            Self::Arrow { from, to, color } => Mark::Arrow {
                from: *from,
                to: *to,
                color: *color,
            },
            Self::Pen { points, color } => Mark::Pen {
                points: points.clone(),
                color: *color,
            },
            Self::Text { at, color, .. } | Self::Pin { at, color } => Mark::Pin {
                at: *at,
                color: *color,
            },
        }
    }

    pub(crate) fn color(&self) -> usize {
        match self {
            Self::Box { color, .. }
            | Self::Arrow { color, .. }
            | Self::Pen { color, .. }
            | Self::Pin { color, .. }
            | Self::Text { color, .. } => *color,
        }
    }

    /// Offset every normalized point by `delta`, clamped to the surface, so a
    /// duplicated shape lands just beside its original.
    pub(crate) fn nudge(&mut self, delta: f32) {
        let shift = |p: &mut (f32, f32)| {
            p.0 = (p.0 + delta).clamp(0.0, 1.0);
            p.1 = (p.1 + delta).clamp(0.0, 1.0);
        };
        match self {
            Self::Box { from, to, .. } | Self::Arrow { from, to, .. } => {
                shift(from);
                shift(to);
            }
            Self::Pen { points, .. } => points.iter_mut().for_each(shift),
            Self::Pin { at, .. } | Self::Text { at, .. } => shift(at),
        }
    }

    fn points(&self) -> Vec<(f32, f32)> {
        match self {
            Self::Box { from, to, .. } | Self::Arrow { from, to, .. } => vec![*from, *to],
            Self::Pen { points, .. } => points.clone(),
            Self::Pin { at, .. } | Self::Text { at, .. } => vec![*at],
        }
    }

    pub(crate) fn valid(&self) -> bool {
        let points = self.points();
        !matches!(self, Self::Text { text, .. } if text.trim().is_empty() || text.chars().count() > 8000)
            && self.color() < 4
            && !points.is_empty()
            && points.len() <= 4096
            && points.iter().all(|p| {
                [p.0, p.1]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            })
    }

    pub(crate) fn bounds(&self, width: f32, height: f32) -> Rect {
        let points = self.points();
        let min_x = points.iter().map(|p| p.0).fold(1.0, f32::min);
        let min_y = points.iter().map(|p| p.1).fold(1.0, f32::min);
        let max_x = points.iter().map(|p| p.0).fold(0.0, f32::max);
        let max_y = points.iter().map(|p| p.1).fold(0.0, f32::max);
        Rect {
            x: min_x * width,
            y: min_y * height,
            width: (max_x - min_x) * width,
            height: (max_y - min_y) * height,
        }
    }

    pub(crate) fn hit(&self, at: (f32, f32), viewport: (f32, f32), reach: f32) -> bool {
        let scale = |p: (f32, f32)| (p.0 * viewport.0, p.1 * viewport.1);
        if let Self::Text {
            at: origin, text, ..
        } = self
        {
            let origin = scale(*origin);
            let width = text
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(1) as f32
                * 9.0;
            return at.0 >= origin.0 - reach
                && at.0 <= origin.0 + width + reach
                && at.1 >= origin.1 - reach
                && at.1 <= origin.1 + text.lines().count().max(1) as f32 * 22.0 + reach;
        }
        if let Self::Pin { at: pin, .. } = self {
            let pin = scale(*pin);
            return (pin.0 - at.0).hypot(pin.1 - at.1) <= reach;
        }
        let points = match self {
            Self::Box { from, to, .. } => vec![*from, (to.0, from.1), *to, (from.0, to.1), *from],
            _ => self.points(),
        };
        points.windows(2).any(|pair| {
            let (a, b) = (scale(pair[0]), scale(pair[1]));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = dx * dx + dy * dy;
            let t = if length > 0.0 {
                (((at.0 - a.0) * dx + (at.1 - a.1) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (at.0 - a.0 - t * dx).hypot(at.1 - a.1 - t * dy) <= reach
        })
    }

    pub(crate) fn paint(
        &self,
        viewport: (f32, f32),
        stroke: f32,
        head: f32,
        color: gpui::Hsla,
        window: &mut Window,
    ) {
        let place = |p: (f32, f32)| point(px(p.0 * viewport.0), px(p.1 * viewport.1));
        let mut path = PathBuilder::stroke(px(stroke));
        match self.mark() {
            Mark::Box { from, to, .. } => {
                path.move_to(place(from));
                for p in [(to.0, from.1), to, (from.0, to.1), from] {
                    path.line_to(place(p));
                }
            }
            Mark::Arrow { from, to, .. } => {
                let (tail, tip) = (place(from), place(to));
                path.move_to(tail);
                path.line_to(tip);
                let angle = f32::from(tip.y - tail.y).atan2(f32::from(tip.x - tail.x));
                for turn in [-0.45, 0.45] {
                    path.move_to(point(
                        tip.x - px(head * (angle + turn).cos()),
                        tip.y - px(head * (angle + turn).sin()),
                    ));
                    path.line_to(tip);
                }
            }
            Mark::Pen { points, .. } => {
                if let Some(first) = points.first() {
                    path.move_to(place(*first));
                }
                for p in points.iter().skip(1) {
                    path.line_to(place(*p));
                }
            }
            Mark::Pin { .. } => return,
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
}

pub(crate) fn centered_panel(
    center: (f32, f32),
    panel: (f32, f32),
    viewport: (f32, f32),
    margin: f32,
) -> (f32, f32) {
    (
        (center.0 - panel.0 / 2.0).clamp(margin, (viewport.0 - panel.0 - margin).max(margin)),
        (center.1 - panel.1 / 2.0).clamp(margin, (viewport.1 - panel.1 - margin).max(margin)),
    )
}

pub(crate) fn finish_shape(
    tool: Tool,
    mut points: Vec<(f32, f32)>,
    end: (f32, f32),
    color: usize,
) -> Option<AnnotationShape> {
    if points.is_empty() {
        return None;
    }
    if points.last() != Some(&end) {
        if points.len() >= 4096 {
            points.pop();
        }
        points.push(end);
    }
    AnnotationShape::from_stroke(tool, &points, color)
}

pub(crate) fn note_panel_position(
    bounds: Rect,
    panel: (f32, f32),
    viewport: (f32, f32),
    margin: f32,
) -> (f32, f32) {
    let bottom = bounds.y + bounds.height;
    let below = bottom + margin;
    let above = bounds.y - panel.1 - margin;
    let y = if below + panel.1 <= viewport.1 - margin {
        below
    } else if above >= margin {
        above
    } else if viewport.1 - bottom >= bounds.y {
        below
    } else {
        above
    };
    let x = bounds.x + bounds.width / 2.0 - panel.0 / 2.0;
    (
        x.clamp(margin, (viewport.0 - panel.0 - margin).max(margin)),
        y.clamp(margin, (viewport.1 - panel.1 - margin).max(margin)),
    )
}

#[cfg(test)]
#[path = "tests/drawing.rs"]
mod tests;
