//! Inventory of the crate's production `.rs` files, used to pin the exact
//! source closure the physical boundary tests validate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::source_root::{source_root_boundary, SourceRootError};

pub(crate) fn production_sources(source_root: &Path) -> Result<BTreeSet<PathBuf>, SourceRootError> {
    source_root_boundary(source_root)?;
    let mut sources = BTreeSet::new();
    collect_production_sources(source_root, &mut sources);
    Ok(sources)
}

fn collect_production_sources(directory: &Path, sources: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory must be readable") {
        let entry = entry.expect("source entry must be readable");
        let file_type = entry.file_type().expect("source type must be readable");
        assert!(
            !file_type.is_symlink(),
            "production sources must not be symlinks"
        );
        let path = entry.path();
        if file_type.is_dir() {
            collect_production_sources(&path, sources);
            continue;
        }
        assert!(file_type.is_file(), "unexpected production source entry");
        assert_eq!(
            path.extension().and_then(|value| value.to_str()),
            Some("rs")
        );
        sources.insert(path);
    }
}
