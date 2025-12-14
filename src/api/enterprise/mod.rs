// Enterprise Integration Module
//
// Central nervous system coordinating all enterprise modules

pub mod api_facade;
pub mod lifecycle;
pub mod registry;
pub mod resources;
pub mod tracing;

// Re-export main types and functions
pub use api_facade::*;
pub use lifecycle::*;
pub use registry::*;
pub use resources::*;
pub use tracing::*;
