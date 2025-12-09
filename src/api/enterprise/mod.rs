// Enterprise Integration Module
//
// Central nervous system coordinating all enterprise modules

pub mod registry;
pub mod tracing;
pub mod resources;
pub mod api_facade;
pub mod lifecycle;

// Re-export main types and functions
pub use registry::*;
pub use tracing::*;
pub use resources::*;
pub use api_facade::*;
pub use lifecycle::*;
