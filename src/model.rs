use serde::{Deserialize, Serialize};

/// Current properties captured from an element anchored to an inspection note.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectionSnapshot {
    /// All properties exposed by the inspector, including unchanged values.
    pub properties: std::collections::BTreeMap<String, String>,
}

/// Window coordinates in logical pixels, independent of display scaling.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    pub(crate) fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.width >= 0.0
            && self.height >= 0.0
    }

    pub(crate) fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.width && y <= self.y + self.height
    }
}

/// Agentation's feedback lifecycle.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    /// Waiting for an agent to read it.
    #[default]
    Pending,
    /// An agent has read the feedback.
    Acknowledged,
    /// The requested change has been verified.
    Resolved,
    /// The feedback was declined.
    Dismissed,
}

/// One temporary GPUI style override made in the native inspector.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GpuiStyleChange {
    /// Styled property, such as width or padding.
    pub property: String,
    /// Value before the inspector override.
    pub before: String,
    /// Value after the inspector override.
    pub after: String,
}

/// Inspector configuration attached to a feedback note for an agent to implement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GpuiConfiguration {
    /// Source location of the GPUI element selected in the inspector.
    pub source: String,
    /// GPUI global element path selected in the inspector.
    pub element_path: String,
    /// Only the properties changed through the visual inspector.
    pub changes: Vec<GpuiStyleChange>,
    /// Suggested GPUI builder calls representing the overrides.
    pub rust: String,
}

/// Feedback supplied by the UI or an API client. IDs and timestamps are server-owned.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationInput {
    /// Human feedback, at most 8,000 characters.
    pub comment: String,
    /// Stable native target identifier; this is not a CSS selector.
    pub element_path: String,
    /// Human-readable component name.
    pub element: String,
    /// Horizontal position as a percentage of the window width.
    pub x: f32,
    /// Vertical position in window logical pixels.
    pub y: f32,
    /// Visible target bounds at creation time.
    pub bounding_box: Rect,
    /// Rust call-site location, when registered by the application.
    pub source_file: Option<String>,
    /// Full property snapshot for an element-anchored inspection annotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspection: Option<InspectionSnapshot>,
    /// Temporary visual GPUI overrides active when the note was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpui_configuration: Option<GpuiConfiguration>,
    /// Optional live-window drawing, using normalized viewport coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<crate::AnnotationShape>,
}

/// A persisted note attached to a native element and route.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// Stable, one-based number within the route; deleted numbers are not reused.
    #[serde(default)]
    pub index: u64,
    /// Unique note ID.
    pub id: String,
    /// Owning session ID.
    pub session_id: String,
    /// Route URL.
    pub url: String,
    /// Creation time in Unix milliseconds.
    pub timestamp: u64,
    /// Current lifecycle state.
    pub status: AnnotationStatus,
    /// This implementation emits feedback notes.
    pub kind: String,
    /// Optional Ely annotation color, zero through three; older notes use their shape color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<usize>,
    /// Element context and human comment.
    #[serde(flatten)]
    pub feedback: AnnotationInput,
    /// Conversation visible to both the human and agent.
    pub thread: Vec<ThreadMessage>,
}

impl Annotation {
    pub(crate) fn annotation_color(&self) -> usize {
        self.color.unwrap_or_else(|| {
            self.feedback
                .shape
                .as_ref()
                .map_or(0, |shape| shape.color())
        })
    }
}

/// Mutable annotation fields. Omitted fields are preserved.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnotationPatch {
    /// Replacement comment.
    pub comment: Option<String>,
    /// New lifecycle state.
    pub status: Option<AnnotationStatus>,
}

/// A human or agent response to a note.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMessage {
    /// Unique message ID.
    pub id: String,
    /// Author label, normally human or agent.
    pub role: String,
    /// Response text.
    pub message: String,
    /// Creation time in Unix milliseconds.
    pub timestamp: u64,
}

/// A window route's feedback collection, reused after a development restart.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Unique session ID.
    pub id: String,
    /// Application name.
    pub app_name: String,
    /// Route URL supplied by the host app.
    pub url: String,
    /// All notes, including resolved notes.
    pub annotations: Vec<Annotation>,
}

/// A sequenced event delivered through the REST/SSE API.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Monotonic event cursor within this server run.
    pub id: u64,
    /// Event name such as annotation.created.
    #[serde(rename = "type")]
    pub kind: String,
    /// Owning session.
    pub session_id: String,
    /// Updated object, or the deleted annotation's ID.
    pub data: serde_json::Value,
}
