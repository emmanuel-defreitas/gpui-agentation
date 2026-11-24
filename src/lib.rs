//! macOS-native element feedback and a loopback API, compiled only with debug assertions.
#![cfg(debug_assertions)]
#![cfg(feature = "native")]
#![deny(missing_docs)]

#[cfg(not(target_os = "macos"))]
compile_error!("gpui-agentation's native library supports macOS only");

mod drawing;
mod http;
mod inspection;
mod integration;
mod mcp;
mod model;
mod overlay;
mod store;
mod target;

mod ui;

pub use drawing::AnnotationShape;
/// The exact GPUI Kit dependency shared by Agentation and its host.
pub use gpui as kit;
pub use http::Server;
pub use inspection::kit::RendererPolicy;
#[cfg(feature = "test-support")]
pub use integration::init_for_tests;
pub use integration::{Agentation, Assets, HostMode, InitOptions, init, init_with_options};
pub use model::{
    Annotation, AnnotationInput, AnnotationPatch, AnnotationStatus, Event, GpuiConfiguration,
    GpuiStyleChange, InspectionSnapshot, Rect, Session, ThreadMessage,
};
pub use overlay::AnnotationOverlay;
pub use overlay::commands::{AnnotationCommand, debug_menu};
pub use store::{Error, Store, ToolbarPosition};
pub use target::{begin_frame, target};

#[cfg(test)]
fn init_inspection_tests(cx: &mut gpui::App) {
    gpui::init(cx);
    overlay::inspector::install(cx);
}
