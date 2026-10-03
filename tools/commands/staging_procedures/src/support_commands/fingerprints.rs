//! `cargo xtask staging fingerprints`: the source and configuration digests a recording started
//! now would bind to.
//!
//! **Role:** prints both fingerprints, the values the run day compares before each step.
//!
//! **Position:** called by `staging_dispatch.rs`; the digests come from the acceptance verifier's own
//! fingerprints through
//! `api_readiness_checks::operational_recording::current_fingerprints`, so they are
//! exactly what `RecordingSession::begin` snapshots and `verify api-readiness` recomputes.
//!
//! **Signals & state:** none; reads the tree and this process's environment.
//!
//! **Invariants:** only reads; each digest is 64 lowercase hexadecimal characters, or the command
//! fails rather than printing something else.

use std::io::Write;
use std::path::Path;

use crate::error::{Result, ensure};

use api_readiness_checks::operational_recording::current_fingerprints;

/// Prints the fingerprints of the checkout at `root`.
pub(crate) fn run(root: &Path, output: &mut dyn Write) -> Result<u8> {
    let (source, configuration) = current_fingerprints(root)?;
    output.write_all(render(&source, &configuration)?.as_bytes())?;
    Ok(0)
}

/// `source_sha256=<hex>` and `configuration_sha256=<hex>`, one per line.
pub(crate) fn render(source: &str, configuration: &str) -> Result<String> {
    for (name, digest) in [("source", source), ("configuration", configuration)] {
        ensure!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "the {name} fingerprint is not a lowercase SHA-256 digest"
        );
    }
    Ok(format!(
        "source_sha256={source}\nconfiguration_sha256={configuration}\n"
    ))
}
