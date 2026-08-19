//! Artifact manifest parse, hash, and load-time validation (§C6).

mod error;
mod hash;
mod manifest;
mod wizer;

pub use error::ArtifactError;
pub use hash::sha256_hex;
pub use manifest::{ArtifactManifest, parse_manifest, validate_module};
pub use wizer::{wizer_bin, wizer_file};
