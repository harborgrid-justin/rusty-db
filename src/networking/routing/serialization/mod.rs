// Serialization module for cluster messages
//
// This module provides efficient serialization and deserialization of cluster messages
// using binary encoding with optional compression and integrity checks.

pub mod binary;
pub mod messages;

pub use binary::BinaryCodec;
pub use messages::*;
