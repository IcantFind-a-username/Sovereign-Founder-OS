//! Shared fixture-root helper for v01-D03 tests.

use std::path::{Path, PathBuf};

use sovereign_authority::broker::fixture_root::SyntheticFixtureRootV1;

pub fn marked_root(parent: &Path) -> PathBuf {
    let path = parent.join("fixture-root");
    SyntheticFixtureRootV1::create(&path, 1).expect("a fresh fixture root must create");
    path
}
