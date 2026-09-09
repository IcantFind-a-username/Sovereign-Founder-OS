use std::collections::BTreeMap;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub(crate) struct FakeRoot {
    root: tempfile::TempDir,
    home: PathBuf,
    xdg: PathBuf,
    work: PathBuf,
}

impl FakeRoot {
    pub(crate) fn new(canary: &'static [u8]) -> io::Result<Self> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        let xdg = root.path().join("xdg-data");
        let work = root.path().join("work");
        for directory in [&home, &xdg, &work] {
            fs::create_dir(directory)?;
        }
        for base in [&xdg, &home.join(".local/share")] {
            fs::create_dir_all(base.join("sovereign-founder-os/vault"))?;
            fs::write(base.join("sovereign-founder-os/device.json"), canary)?;
            fs::write(base.join("sovereign-founder-os/ledger.json"), canary)?;
            fs::write(base.join("sovereign-founder-os/vault/secret"), canary)?;
            fs::create_dir_all(base.join("sovereign-founder-os/unrelated/nested"))?;
            fs::write(
                base.join("sovereign-founder-os/unrelated/nested/sentinel"),
                canary,
            )?;
        }
        #[cfg(target_os = "macos")]
        {
            let base = home.join("Library/Application Support");
            fs::create_dir_all(base.join("sovereign-founder-os/vault"))?;
            fs::write(base.join("sovereign-founder-os/device.json"), canary)?;
            fs::write(base.join("sovereign-founder-os/ledger.json"), canary)?;
            fs::write(base.join("sovereign-founder-os/vault/secret"), canary)?;
            fs::create_dir_all(base.join("sovereign-founder-os/unrelated/nested"))?;
            fs::write(
                base.join("sovereign-founder-os/unrelated/nested/sentinel"),
                canary,
            )?;
        }
        Ok(Self {
            root,
            home,
            xdg,
            work,
        })
    }

    pub(crate) fn configure(&self, command: &mut Command) {
        command
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.xdg)
            .current_dir(&self.work);
    }

    pub(crate) fn snapshot(&self) -> io::Result<TreeSnapshot> {
        snapshot_tree(self.root.path())
    }
}

pub(crate) type TreeSnapshot = BTreeMap<PathBuf, TreeEntry>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TreeEntry {
    mode: u32,
    modified: SystemTime,
    contents: Option<Vec<u8>>,
}

pub(crate) fn mutation_probe() {
    for mutation in [
        "added",
        "removed",
        "content",
        "mode",
        "empty-dir",
        "symlink",
        "unexpected",
        "io-error",
    ] {
        let root = FakeRoot::new(b"mutation-canary").unwrap();
        let path = root.work.join("fixture");
        match mutation {
            "added" => {
                let before = root.snapshot().unwrap();
                let file = Path::new("work/fixture/nested/file");
                assert!(!before.contains_key(file));
                fs::create_dir_all(path.join("nested")).unwrap();
                fs::write(path.join("nested/file"), b"new").unwrap();
                let after = root.snapshot().unwrap();
                assert_eq!(after[file].contents.as_deref(), Some(b"new".as_slice()));
                for directory in ["work/fixture", "work/fixture/nested"] {
                    assert!(!before.contains_key(Path::new(directory)));
                    assert_eq!(after[Path::new(directory)].contents, None);
                }
            }
            "removed" => {
                fs::create_dir_all(path.join("nested")).unwrap();
                fs::write(path.join("nested/file"), b"x").unwrap();
                let before = root.snapshot().unwrap();
                let file = Path::new("work/fixture/nested/file");
                let directory = Path::new("work/fixture/nested");
                assert_eq!(before[file].contents.as_deref(), Some(b"x".as_slice()));
                assert_eq!(before[directory].contents, None);
                fs::remove_file(path.join("nested/file")).unwrap();
                fs::remove_dir(path.join("nested")).unwrap();
                let after = root.snapshot().unwrap();
                assert!(!after.contains_key(file));
                assert!(!after.contains_key(directory));
            }
            "content" => {
                fs::write(&path, b"before").unwrap();
                let before = root.snapshot().unwrap();
                fs::write(&path, b"after").unwrap();
                let after = root.snapshot().unwrap();
                let file = Path::new("work/fixture");
                assert_eq!(before[file].contents.as_deref(), Some(b"before".as_slice()));
                assert_eq!(after[file].contents.as_deref(), Some(b"after".as_slice()));
            }
            "mode" => {
                fs::write(&path, b"mode").unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
                let before = root.snapshot().unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
                let after = root.snapshot().unwrap();
                let file = Path::new("work/fixture");
                assert_eq!(before[file].mode & 0o7777, 0o640);
                assert_eq!(after[file].mode & 0o7777, 0o600);
                assert_ne!(before[file].mode, after[file].mode);
            }
            "empty-dir" => {
                let before = root.snapshot().unwrap();
                fs::create_dir_all(path.join("nested/empty")).unwrap();
                let after = root.snapshot().unwrap();
                for directory in [
                    "work/fixture",
                    "work/fixture/nested",
                    "work/fixture/nested/empty",
                ] {
                    assert!(!before.contains_key(Path::new(directory)));
                    assert_eq!(after[Path::new(directory)].contents, None);
                }
            }
            "symlink" => {
                #[cfg(unix)]
                std::os::unix::fs::symlink("missing", &path).unwrap();
                #[cfg(not(unix))]
                continue;
                assert!(root.snapshot().is_err());
            }
            "unexpected" => {
                #[cfg(unix)]
                {
                    let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
                    assert!(root.snapshot().is_err());
                    continue;
                }
                #[cfg(not(unix))]
                continue;
            }
            "io-error" => {
                assert!(snapshot_tree(&root.root.path().join("missing")).is_err());
                continue;
            }
            _ => unreachable!(),
        }
    }
}

fn snapshot_tree(root: &Path) -> io::Result<TreeSnapshot> {
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries)?;
    Ok(entries)
}

fn visit(root: &Path, path: &Path, entries: &mut TreeSnapshot) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let relative = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    #[cfg(unix)]
    let mode = std::os::unix::fs::MetadataExt::mode(&metadata);
    #[cfg(not(unix))]
    let mode = 0;
    if metadata.is_dir() {
        entries.insert(
            relative,
            TreeEntry {
                mode,
                modified: metadata.modified()?,
                contents: None,
            },
        );
        for child in fs::read_dir(path)? {
            visit(root, &child?.path(), entries)?;
        }
    } else if metadata.is_file() {
        entries.insert(
            relative,
            TreeEntry {
                mode,
                modified: metadata.modified()?,
                contents: Some(fs::read(path)?),
            },
        );
    } else {
        return Err(io::Error::other(
            "fixture contains symlink or unexpected entry type",
        ));
    }
    Ok(())
}
