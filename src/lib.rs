//! FourDeers - Stereoscopic 4D Visualization

pub mod app;
pub mod camera;
pub mod geometry;
pub mod input;
pub mod state;
pub mod ui;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use app::FourDeersApp;
pub use input::{CameraAction, TapAnalysis, Zone};
pub use state::AppState;
