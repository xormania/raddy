//! Artifact manifest parse, hash, and load-time validation.

mod error;
mod hash;
mod loader;
mod manifest;
mod wizer;

pub use error::ArtifactError;
pub use hash::sha256_hex;
pub use loader::{LoadedArtifact, load_artifact};
pub use manifest::{ArtifactManifest, PrecompiledMeta, parse_manifest, validate_module};
pub use wizer::{wizer_bin, wizer_file};
