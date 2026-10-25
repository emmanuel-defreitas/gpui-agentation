//! Native element feedback and a loopback API, compiled only with debug assertions.
#![cfg(debug_assertions)]
#![cfg(feature = "native")]
#![deny(missing_docs)]

mod drawing;
mod hot_reload;
mod http;
mod integration;
mod mcp;
mod model;
mod overlay;
mod store;
mod target;

/// Bundled components from the exact pinned Ely revision.
///
/// These types and globals belong to Agentation, independently of any host Ely crate.
#[allow(missing_docs, deprecated, unexpected_cfgs, clippy::all)]
pub mod ely {
    include!(concat!(env!("OUT_DIR"), "/ely/lib.rs"));
}

pub use drawing::AnnotationShape;
pub use http::Server;
#[cfg(feature = "test-support")]
pub use integration::init_for_tests;
pub use integration::{Agentation, Assets, init};
pub use model::{
    Annotation, AnnotationInput, AnnotationPatch, AnnotationStatus, Event, Rect, Session,
    ThreadMessage,
};
pub use overlay::AnnotationOverlay;
pub use overlay::commands::{AnnotationCommand, debug_menu};
pub use store::{Error, Store, ToolbarPosition};
pub use target::{begin_frame, target};
