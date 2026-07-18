//! Ground Truth physical-access and generic extraction boundary.
//!
//! Types in this crate are authoritative acquisition/extractor records. They
//! must be mapped to subjective identities before agent cognition consumes
//! them.

pub mod access;
pub mod extraction;

pub use access::*;
pub use extraction::*;
