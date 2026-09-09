//! Root classification: deciding whether a directory may be used as the
//! fixture's store at all.
//!
//! The broker performs this itself rather than trusting the caller, and it
//! does so before it binds, locks, or opens anything. That ordering is the
//! whole point. A same-account process can hand the hidden mode a
//! syntactically valid bootstrap frame — RFC 0006 states plainly that this is
//! unqualified fixture control and never product admission — so the one thing
//! that must not follow is that such a caller can aim the broker at the
//! owner's real Vault or workspace.
//!
//! The rule is allow-list shaped: a root must positively carry the fixture
//! marker. Absence of known product files is not enough, because the next
//! product file to be invented would not be on any deny-list.

use super::protocol::Diagnostic;
use std::path::{Path, PathBuf};

/// The file a directory must contain to be a fixture root. Its presence is
/// the claim "this holds synthetic data and nothing else".
pub const FIXTURE_MARKER: &str = "synthetic-owner-effect-fixture-v1";

/// Files that mark a real product, legacy, or Vault root. A root carrying any
/// of these is refused even if someone also dropped the fixture marker in it.
const PRODUCT_MARKERS: &[&str] = &[
    "device.json",
    "ledger.json",
    "vault",
    "workspace_graph.enc",
    "manifest.json",
    "authority",
    "executions",
    "outbox",
    "workflows",
    "artifacts",
];

/// A classified, canonical fixture root.
pub struct FixtureRoot(PathBuf);

impl FixtureRoot {
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl std::fmt::Debug for FixtureRoot {
    /// Value-free: a path is not a secret, but it is a detail of the owner's
    /// machine and nothing here needs to print it.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FixtureRoot(<redacted>)")
    }
}

/// Decide whether `candidate` may be used. Every rejection is the same
/// value-free `RootRejected`: telling a caller *which* rule it tripped would
/// let it probe the filesystem through this interface.
pub fn classify(candidate: &Path) -> Result<FixtureRoot, Diagnostic> {
    // A symlink is refused before it is followed: canonicalizing first would
    // resolve it and then approve whatever it pointed at.
    let metadata = std::fs::symlink_metadata(candidate).map_err(|_| Diagnostic::RootRejected)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Diagnostic::RootRejected);
    }
    let root = candidate
        .canonicalize()
        .map_err(|_| Diagnostic::RootRejected)?;

    // Positive claim first.
    let marker = root.join(FIXTURE_MARKER);
    let marker_metadata =
        std::fs::symlink_metadata(&marker).map_err(|_| Diagnostic::RootRejected)?;
    if marker_metadata.file_type().is_symlink() || !marker_metadata.is_file() {
        return Err(Diagnostic::RootRejected);
    }

    // Then: nothing in it may look like product state.
    if contains_product_marker(&root) {
        return Err(Diagnostic::RootRejected);
    }
    // And it must not be nested inside one. A fixture directory created
    // *within* the owner's data root would otherwise pass on its own merits.
    let mut ancestor = root.parent();
    while let Some(directory) = ancestor {
        if contains_product_marker(directory) {
            return Err(Diagnostic::RootRejected);
        }
        ancestor = directory.parent();
    }

    Ok(FixtureRoot(root))
}

fn contains_product_marker(directory: &Path) -> bool {
    PRODUCT_MARKERS
        .iter()
        .any(|name| directory.join(name).exists())
}
