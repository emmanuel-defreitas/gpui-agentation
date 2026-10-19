//! Native element feedback and a loopback API, compiled only with debug assertions.
#![cfg(debug_assertions)]
#![deny(missing_docs)]

mod drawing;
mod http;
mod model;
mod overlay;
mod store;
mod target;

pub use drawing::AnnotationShape;
pub use http::Server;
pub use model::{
    Annotation, AnnotationInput, AnnotationPatch, AnnotationStatus, Event, Rect, Session,
    ThreadMessage,
};
pub use overlay::AnnotationOverlay;
pub use store::{Error, Store};
pub use target::{begin_frame, target};
