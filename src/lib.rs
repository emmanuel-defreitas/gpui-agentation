//! macOS-native element feedback and a loopback API, compiled only with debug assertions.
#![cfg(debug_assertions)]
#![cfg(feature = "native")]
#![deny(missing_docs)]

#[cfg(not(target_os = "macos"))]
compile_error!("gpui-agentation's native library supports macOS only");

mod drawing;
mod http;
mod integration;
mod mcp;
mod model;
mod overlay;
mod store;
mod target;

/// Bundled components based on the pinned Ely revision and recorded local patches.
///
/// These types and globals belong to Agentation, independently of any host Ely crate.
#[allow(missing_docs, deprecated, unexpected_cfgs, clippy::all)]
pub mod ely {
    include!(concat!(env!("OUT_DIR"), "/ely/lib.rs"));
}

#[cfg(test)]
#[path = "tests/ely.rs"]
mod ely_tests;

/// Runs the unit tests of the build-script helper that bundles Ely.
#[cfg(test)]
#[path = "../build/ely.rs"]
#[allow(dead_code)]
mod ely_build;

pub use drawing::AnnotationShape;
pub use http::Server;
#[cfg(feature = "test-support")]
pub use integration::init_for_tests;
pub use integration::{Agentation, Assets, init};
pub use model::{
    Annotation, AnnotationInput, AnnotationPatch, AnnotationStatus, Event, GpuiConfiguration,
    GpuiStyleChange, InspectionSnapshot, Rect, Session, ThreadMessage,
};
pub use overlay::AnnotationOverlay;
pub use overlay::commands::{AnnotationCommand, debug_menu};
pub use store::{Error, Store, ToolbarPosition};
pub use target::{begin_frame, target};
