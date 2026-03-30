//! FourDeers - Stereoscopic 4D Visualization

pub mod app;
pub mod camera;
pub mod colors;
pub mod input;
pub mod polytopes;
pub mod render;
pub mod rotation4d;
pub mod tetrahedron;
pub mod toy;
pub mod toys;

#[cfg(test)]
mod test_utils;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use app::FourDeersApp;
pub use camera::Camera;
pub use input::{DragView, TapAnalysis, TetraId, Zone};
pub use polytopes::{PolytopeType, Vertex4D};
pub use toy::Toy;
