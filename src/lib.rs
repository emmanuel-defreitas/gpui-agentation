//! Native element feedback and a loopback API, compiled only with debug assertions.
#![cfg(debug_assertions)]
#![cfg(feature = "native")]
#![deny(missing_docs)]

mod drawing;
mod http;
mod mcp;
mod model;
mod overlay;
mod store;
mod target;

/// The exact Ely distribution used by Agentation, for shared theme and focus state.
/// Existing Ely hosts must use this distribution too; do not mix it with Git Ely.
pub use ely_gpui_component as ely;

pub use drawing::AnnotationShape;
pub use http::Server;
pub use model::{
    Annotation, AnnotationInput, AnnotationPatch, AnnotationStatus, Event, Rect, Session,
    ThreadMessage,
};
pub use overlay::AnnotationOverlay;
pub use store::{Error, Store};
pub use target::{begin_frame, target};
