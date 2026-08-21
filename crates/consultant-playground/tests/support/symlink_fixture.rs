//! Fixture package whose `src/` root is a symlink, for exercising the
//! symlink rejection path in `source_root_boundary` and `manifest_boundary`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::ManifestFixture;

#[cfg(any(unix, windows))]
pub(crate) struct SymlinkedSourceFixture {
    pub(crate) base: ManifestFixture,
    pub(crate) source_link: PathBuf,
}

#[cfg(any(unix, windows))]
impl SymlinkedSourceFixture {
    pub(crate) fn new() -> std::io::Result<Self> {
        let base = ManifestFixture::new("");
        let real_source = base.root.join("real-source");
        let source_link = base.root.join("src");
        fs::create_dir(&real_source)?;
        fs::write(real_source.join("lib.rs"), "mod domain;\n")?;
        fs::write(real_source.join("domain.rs"), "")?;
        fs::remove_dir_all(&source_link)?;
        create_directory_symlink(&real_source, &source_link)?;
        Ok(Self { base, source_link })
    }
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}
