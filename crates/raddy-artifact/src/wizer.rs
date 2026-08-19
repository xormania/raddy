use std::path::Path;
use std::process::Command;

use crate::error::ArtifactError;

/// Resolve the Wizer CLI. `WIZER` wins, then `wizer` on `PATH`.
#[must_use]
pub fn wizer_bin() -> String {
    std::env::var("WIZER").unwrap_or_else(|_| "wizer".into())
}

/// Snapshot `input` by running `wizer.initialize`. Writes `output`.
pub fn wizer_file(input: &Path, output: &Path) -> Result<(), ArtifactError> {
    let bin = wizer_bin();
    let status = Command::new(&bin)
        .arg("-f")
        .arg("wizer.initialize")
        .arg("-o")
        .arg(output)
        .arg(input)
        .status()
        .map_err(|err| ArtifactError::Wizer(format!("spawn {bin}: {err}")))?;
    if !status.success() {
        return Err(ArtifactError::Wizer(format!(
            "{bin} exited {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}
