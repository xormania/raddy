use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use crate::{ArtifactError, ArtifactManifest, parse_manifest, sha256_hex, validate_module};

#[derive(Debug)]
pub struct LoadedArtifact {
    manifest: ArtifactManifest,
    precompiled: File,
}

impl LoadedArtifact {
    #[must_use]
    pub fn manifest(&self) -> &ArtifactManifest {
        &self.manifest
    }

    #[must_use]
    pub fn into_precompiled(self) -> File {
        self.precompiled
    }
}

/// Load and verify the module selected for this host and Wasmtime version.
pub fn load_artifact(
    root: &Path,
    target: &str,
    wasmtime_version: &str,
) -> Result<LoadedArtifact, ArtifactError> {
    let manifest_text = std::fs::read_to_string(root.join("raddy.artifact.toml"))?;
    let manifest = parse_manifest(&manifest_text)?;
    let wasm = std::fs::read(resolve_artifact_path(root, &manifest.module.wasm)?)?;
    validate_module(&manifest, &wasm)?;

    let precompiled = manifest
        .precompiled
        .get(target)
        .ok_or_else(|| ArtifactError::MissingPrecompiled(target.into()))?;
    if precompiled.wasmtime != wasmtime_version {
        return Err(ArtifactError::WasmtimeVersion {
            got: precompiled.wasmtime.clone(),
            want: wasmtime_version.into(),
        });
    }
    let path = resolve_artifact_path(root, &precompiled.cwasm)?;
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if sha256_hex(&bytes) != precompiled.sha256 {
        return Err(ArtifactError::HashMismatch);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(LoadedArtifact {
        manifest,
        precompiled: file,
    })
}

fn resolve_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, ArtifactError> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ArtifactError::Manifest(
            "artifact paths must be non-empty relative paths without traversal".into(),
        ));
    }
    Ok(root.join(relative))
}
