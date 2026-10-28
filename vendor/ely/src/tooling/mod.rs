mod a11y;
mod events;
#[cfg(debug_assertions)]
pub(crate) mod inspector;
mod perf;
mod playground;
mod renders;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod tokens;

pub use a11y::A11yChecker;
pub use events::EventLogger;
#[cfg(debug_assertions)]
pub use inspector::{
    InspectorSelection, InspectorStyleChange, InspectorStyleConfiguration,
    active_inspector_configuration, clear_inspector_selection, inspector_panel,
    inspector_selection, inspector_target, install_inspector,
};
#[cfg(all(debug_assertions, agentation_external_inspector))]
pub use inspector::set_inspector_target;
pub use perf::FpsMeter;
pub use playground::{Knob, Playground, Setting, Settings};
pub use renders::RenderCounter;
pub use tokens::DesignTokenViewer;
