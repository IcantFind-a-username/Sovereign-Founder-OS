//! Guard against a symlinked or non-directory `src/` root before any
//! traversal or manifest canonicalization happens against it.

use std::fs;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SourceRootError {
    MetadataUnreadable,
    RootSymlink,
    RootNotDirectory,
}

pub(crate) fn source_root_boundary(source_root: &Path) -> Result<(), SourceRootError> {
    let metadata =
        fs::symlink_metadata(source_root).map_err(|_| SourceRootError::MetadataUnreadable)?;
    if metadata.file_type().is_symlink() {
        return Err(SourceRootError::RootSymlink);
    }
    if !metadata.is_dir() {
        return Err(SourceRootError::RootNotDirectory);
    }
    Ok(())
}
