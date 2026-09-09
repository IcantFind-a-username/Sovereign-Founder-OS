//! A closed lifecycle for the one kind of root the fixture may use.
//!
//! `bootstrap::classify` answers "may this directory be used at all". This
//! answers a narrower question that only has meaning once the first is
//! settled: *which* fixture root is this, and is it the one the caller
//! thinks?
//!
//! The distinction matters because fixture roots are disposable. A test
//! creates one, fills it, throws it away, and creates another at the same
//! path. Without a generation, a stale handle — a broker that was slow to
//! die, a connection credential from a previous run, a half-written record —
//! points at the new root and looks valid, because every structural check
//! still passes. The generation is what makes "this is a fixture root" and
//! "this is *my* fixture root" different statements.
//!
//! The lifecycle is closed in the sense that matters: there is exactly one
//! way to bring a root into existence and one way to open an existing one,
//! both in this module, and no variant for a product root. A caller cannot
//! construct the type, so holding one is evidence that one of those two paths
//! ran.

use super::bootstrap::{classify, FIXTURE_MARKER};
use super::protocol::Diagnostic;
use std::path::{Path, PathBuf};

/// The marker's first line. Changing it is a format break, which is the
/// point: a root written by an incompatible fixture must not open.
const MARKER_TAG: &str = "synthetic-owner-effect-fixture-v1";

/// A root this process created or opened, at a known generation.
///
/// No `ProductRoot` sibling exists, and no `From`/`Into` reaches this type
/// from anywhere else — so there is nothing to promote a product directory
/// into and nothing to widen.
pub struct SyntheticFixtureRootV1 {
    path: PathBuf,
    generation: u64,
}

impl std::fmt::Debug for SyntheticFixtureRootV1 {
    /// Value-free: a path is a detail of the owner's machine.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SyntheticFixtureRootV1")
            .field("path", &"<redacted>")
            .field("generation", &self.generation)
            .finish()
    }
}

impl SyntheticFixtureRootV1 {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Create a fixture root at `path`, at `generation`.
    ///
    /// Refuses a directory that already holds a marker. Overwriting one would
    /// silently take over a root another process might still be using, and
    /// the caller who wanted a fresh root can pick a fresh path.
    pub fn create(path: &Path, generation: u64) -> Result<Self, Diagnostic> {
        if path.join(FIXTURE_MARKER).exists() {
            return Err(Diagnostic::RootRejected);
        }
        std::fs::create_dir_all(path).map_err(|_| Diagnostic::RootRejected)?;
        let marker = format!("{MARKER_TAG}\ngeneration={generation}\n");
        std::fs::write(path.join(FIXTURE_MARKER), marker).map_err(|_| Diagnostic::RootRejected)?;
        // Classified after writing, so a root that could not pass the
        // structural checks is never reported as created.
        Self::open(path, generation)
    }

    /// Open an existing root, requiring it to be at `expected_generation`.
    ///
    /// The generation is an argument rather than an output. A caller that
    /// read whatever generation it found would accept any root at the path,
    /// which is the confusion this exists to prevent.
    pub fn open(path: &Path, expected_generation: u64) -> Result<Self, Diagnostic> {
        let classified = classify(path)?;
        let marker = std::fs::read_to_string(classified.path().join(FIXTURE_MARKER))
            .map_err(|_| Diagnostic::RootRejected)?;

        let mut lines = marker.lines();
        if lines.next() != Some(MARKER_TAG) {
            return Err(Diagnostic::RootRejected);
        }
        let generation = lines
            .next()
            .and_then(|line| line.strip_prefix("generation="))
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(Diagnostic::RootRejected)?;
        // Trailing content is refused: the marker is exact or it is nothing.
        if lines.next().is_some() {
            return Err(Diagnostic::RootRejected);
        }
        if generation != expected_generation {
            return Err(Diagnostic::RootRejected);
        }

        Ok(Self {
            path: classified.path().to_path_buf(),
            generation,
        })
    }
}
