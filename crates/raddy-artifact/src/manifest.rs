use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ArtifactError;
use crate::hash::sha256_hex;

/// Parsed `raddy.artifact.toml` (§C6). Precompiled cwasm is Stage 5.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub artifact: ArtifactMeta,
    pub module: ModuleMeta,
    pub app: AppMeta,
    #[serde(default)]
    pub capabilities: CapabilityMeta,
    pub limits: LimitsMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMeta {
    pub name: String,
    pub version: String,
    pub abi: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleMeta {
    pub wasm: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppMeta {
    pub fs: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityMeta {
    #[serde(default)]
    pub grants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsMeta {
    pub memory_max_mib: u32,
    pub deadline_ms: u64,
}

/// Parse a manifest document. Unknown keys are hard errors.
pub fn parse_manifest(toml_text: &str) -> Result<ArtifactManifest, ArtifactError> {
    let parsed: ArtifactManifest =
        toml::from_str(toml_text).map_err(|err| ArtifactError::Manifest(err.to_string()))?;
    if parsed.artifact.abi != raddy_abi_version() {
        return Err(ArtifactError::Abi(parsed.artifact.abi));
    }
    if parsed.module.sha256.len() != 64
        || !parsed.module.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ArtifactError::Manifest(
            "module.sha256 must be 64 hex digits".into(),
        ));
    }
    Ok(parsed)
}

/// Refuse the module when the bytes do not match the pinned hash.
pub fn validate_module(manifest: &ArtifactManifest, wasm: &[u8]) -> Result<(), ArtifactError> {
    let got = sha256_hex(wasm);
    if !got.eq_ignore_ascii_case(&manifest.module.sha256) {
        return Err(ArtifactError::HashMismatch);
    }
    let _ = Path::new(&manifest.module.wasm);
    Ok(())
}

fn raddy_abi_version() -> u32 {
    raddy_abi::ABI_VERSION
}
