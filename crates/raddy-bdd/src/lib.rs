//! Cucumber harness. Scenarios bind at crate ports, not at a live process.

mod steps;
mod world;

use std::path::PathBuf;

use cucumber::World as _;

pub use world::BddWorld;

/// Run every feature, or only `@stage-N` when `BDD_STAGE` is set.
pub async fn run() {
    let features = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../features");
    let features = features
        .canonicalize()
        .expect("features/ must exist at the repository root");
    let stage = std::env::var("BDD_STAGE").ok().filter(|s| !s.is_empty());
    BddWorld::cucumber()
        .with_default_cli()
        .filter_run_and_exit(features, move |_, _, scenario| match &stage {
            Some(n) => {
                let tag = format!("stage-{n}");
                scenario.tags.iter().any(|t| t == &tag)
            }
            None => true,
        })
        .await;
}
