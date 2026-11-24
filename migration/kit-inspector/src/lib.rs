//! Kit inspector cutover consumer. Compiles the actual owned services and models.
// Shared modules contain operations used by other migration lanes.
#![allow(dead_code)]
#[path = "../../../src/drawing.rs"]
mod drawing;
#[path = "../../../src/model.rs"]
mod model;
pub use drawing::AnnotationShape;
pub use model::*;
#[path = "../../../src/inspection/mod.rs"]
mod inspection;
#[path = "../../../src/inspection/kit/mod.rs"]
pub mod presentation;

#[cfg(test)]
fn init_inspection_tests(cx: &mut gpui::App) {
    gpui::init(cx);
    presentation::install(presentation::RendererPolicy::Replace, cx);
}
