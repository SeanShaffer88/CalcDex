//! Where CalcDex's own files live (v0.9.0): a portable `data/`
//! folder beside the executable — settings, the calculation log
//! (and its two backups) and the class examples library.
//!
//! Resolution order (first call wins; the result is cached for
//! the process):
//!   0. The `CALCDEX_DATA_DIR` environment variable, when set to
//!      a non-empty path — an explicit override for tests,
//!      wrappers and managed deployments. Nothing is migrated
//!      into or out of an override folder.
//!   1. `<exe folder>\data\` when it already holds any of the
//!      app's files, or when the pre-v0.9.0 files in the legacy
//!      location (step 2) have just been copied into it, or when
//!      it can simply be created (a fresh portable install).
//!   2. The legacy `%APPDATA%\CalcDex\` folder — used when `data\`
//!      cannot be created or the migration copy fails (e.g. the
//!      program sits in a read-only folder), so an existing user
//!      never loses their files to a failed move.
//!   3. Beside the executable, then the current folder (the
//!      pre-v0.9.0 fallbacks, kept for exotic setups).
//!
//! The migration is one-way and copy-only: the legacy files are
//! left exactly as they were, as the backup of the move.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Every file the program keeps in its data folder. The purge
/// commands (`--clean` / `--uninstall`) remove exactly these.
pub const KNOWN_FILES: [&str; 5] = [
    "settings.txt",
    "history.txt",
    "history-backup.txt",
    "history-edit-backup.txt",
    "examples.txt",
];

/// The pre-v0.9.0 location: `%APPDATA%\CalcDex` (None when APPDATA
/// is unset or empty).
pub fn legacy_dir() -> Option<PathBuf> {
    match std::env::var("APPDATA") {
        Ok(appdata) if !appdata.is_empty() => Some(Path::new(&appdata).join("CalcDex")),
        _ => None,
    }
}

/// Cached storage selection and a migration warning, if any.
#[derive(Clone, Debug)]
pub struct DataResolution {
    pub directory: PathBuf,
    pub warning: Option<String>,
}

fn known_entries(dir: &Path) -> bool {
    KNOWN_FILES
        .iter()
        .any(|name| std::fs::symlink_metadata(dir.join(name)).is_ok())
}

fn selected(directory: PathBuf) -> DataResolution {
    DataResolution {
        directory,
        warning: None,
    }
}

/// Resolve explicit locations independently of the process cache.
/// Overrides bypass all migration and filesystem inspection.
pub fn resolve_locations(
    override_dir: Option<PathBuf>,
    exe: Option<PathBuf>,
    legacy: Option<PathBuf>,
) -> DataResolution {
    resolve_with(override_dir, exe, legacy, &mut |_| Ok(()))
}

fn resolve_with(
    override_dir: Option<PathBuf>,
    exe: Option<PathBuf>,
    legacy: Option<PathBuf>,
    boundary: &mut impl FnMut(&str) -> std::io::Result<()>,
) -> DataResolution {
    if let Some(dir) = override_dir {
        return selected(dir);
    }
    if let Some(exe) = &exe {
        let data = exe.join("data");
        let portable = pin_directories(&data).is_ok() && known_entries(&data);
        if portable {
            return selected(data);
        }
        if let Some(old) = legacy.as_ref().filter(|old| known_entries(old)) {
            return match migrate(exe, old, &data, boundary) {
                Ok(()) => selected(data),
                Err(error) => DataResolution {
                    directory: old.clone(),
                    warning: Some(format!(
                        "Storage migration failed; using legacy storage {}: {error}",
                        old.display()
                    )),
                },
            };
        }
        if std::fs::create_dir(&data).is_ok() || pin_directories(&data).is_ok() {
            return selected(data);
        }
    }
    selected(legacy.or(exe).unwrap_or_else(|| PathBuf::from(".")))
}

// Keep every existing ancestor pinned against replacement on Windows. Opening
// the reparse point itself lets us reject it before traversing through it.
fn pin_directories(path: &Path) -> std::io::Result<Vec<std::fs::File>> {
    let mut pins = Vec::new();
    let mut ancestors: Vec<_> = path
        .ancestors()
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    ancestors.reverse();
    for ancestor in ancestors {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options
                .access_mode(0)
                .custom_flags(0x02200000)
                .share_mode(3); // BACKUP_SEMANTICS | OPEN_REPARSE_POINT, no DELETE sharing
        }
        let file = options.open(ancestor).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("cannot pin {}: {error}", ancestor.display()),
            )
        })?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() || is_reparse(&metadata) {
            return Err(std::io::Error::other(
                "storage path contains a reparse point or non-directory",
            ));
        }
        pins.push(file);
    }
    Ok(pins)
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(windows)]
#[path = "migration_native.rs"]
mod native;

#[cfg(not(windows))]
fn migrate(
    _exe: &Path,
    _legacy: &Path,
    _data: &Path,
    _boundary: &mut impl FnMut(&str) -> std::io::Result<()>,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "migration requires Windows",
    ))
}

#[cfg(windows)]
fn migrate(
    exe: &Path,
    legacy: &Path,
    data: &Path,
    boundary: &mut impl FnMut(&str) -> std::io::Result<()>,
) -> std::io::Result<()> {
    // Fully qualified target, pinned parent and trusted stable ancestors remain
    // required; pins do not defeat malicious same-token attribute rewriting.
    let exe = std::path::absolute(exe)?;
    let data = std::path::absolute(data)?;
    if data.parent() != Some(exe.as_path())
        || data.file_name() != Some(std::ffi::OsStr::new("data"))
    {
        return Err(std::io::Error::other(
            "migration target is not the pinned parent's data directory",
        ));
    }
    let parents = pin_directories(&exe)?;
    let parent = parents
        .last()
        .ok_or_else(|| std::io::Error::other("missing migration parent"))?;
    let _source_dirs = pin_directories(legacy)?;
    if std::fs::symlink_metadata(&data).is_ok() {
        return Err(std::io::Error::other(
            "data destination already exists; preserved without merging",
        ));
    }
    // Hold all source handles with no write/delete sharing throughout copying.
    let mut sources = Vec::new();
    for name in KNOWN_FILES {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000).share_mode(1);
        }
        match options.open(legacy.join(name)) {
            Ok(file) => {
                let metadata = file.metadata()?;
                if !metadata.is_file() || is_reparse(&metadata) {
                    return Err(std::io::Error::other("legacy entry is not a regular file"));
                }
                sources.push((name, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    if sources.is_empty() {
        return Err(std::io::Error::other("legacy files disappeared"));
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut owned = None;
    for _ in 0..32 {
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let candidate = exe.join(format!(".calcdex-migration-{}-{id}", std::process::id()));
        let name = candidate
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| std::io::Error::other("invalid generated staging name"))?;
        match native::create(parent, name, true) {
            Ok(handle) => {
                owned = Some((candidate, handle));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error),
        }
    }
    let (stage, guard) = owned
        .ok_or_else(|| std::io::Error::other("cannot allocate migration staging directory"))?;
    let mut created = Vec::new();
    let mut children_closed = false;
    let result = (|| {
        for (name, source) in &mut sources {
            boundary(&format!("copy:{name}"))?;
            created.push(native::create(&guard, name, false)?);
            let index = created.len() - 1;
            let output = &mut created[index];
            std::io::copy(source, output)?;
            boundary(&format!("flush:{name}"))?;
            output.sync_all()?;
        }
        for name in KNOWN_FILES {
            if std::fs::symlink_metadata(legacy.join(name)).is_ok()
                != sources.iter().any(|(existing, _)| *existing == name)
            {
                return Err(std::io::Error::other(
                    "legacy file set changed during migration",
                ));
            }
        }
        boundary("completion:create")?;
        created.push(native::create(&guard, ".migration-complete", false)?);
        let index = created.len() - 1;
        let complete = &mut created[index];
        complete.write_all(b"CalcDex migration complete v1\n")?;
        boundary("completion:flush")?;
        complete.sync_all()?;
        // Windows directory publication requires closed children. Their identity
        // must never be reacquired by path if publication subsequently fails.
        children_closed = true;
        created.clear();
        boundary("publish")?;
        native::publish(&guard, &data)
    })();
    if let Err(error) = result {
        let cleanup = (|| {
            boundary("cleanup")?;
            if children_closed {
                return Err(std::io::Error::other(
                    "child ownership handles closed before publication",
                ));
            }
            let mut failure = None;
            for file in created.drain(..) {
                if let Err(error) = native::delete(&file) {
                    failure = Some(error);
                }
            }
            if let Some(error) = failure {
                return Err(error);
            }
            native::delete(&guard)
        })();
        return match cleanup {
            Ok(()) => Err(error),
            Err(cleanup) => Err(std::io::Error::other(format!(
                "{error}; owned staging retained at {}: {cleanup}",
                stage.display()
            ))),
        };
    }
    Ok(())
}

/// The process-wide selection, including a warning available to host adapters.
pub fn data_resolution() -> DataResolution {
    static RESOLUTION: OnceLock<DataResolution> = OnceLock::new();
    RESOLUTION
        .get_or_init(|| {
            resolve_locations(
                std::env::var_os("CALCDEX_DATA_DIR")
                    .filter(|p| !p.is_empty())
                    .map(PathBuf::from),
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(Path::to_path_buf)),
                legacy_dir(),
            )
        })
        .clone()
}

/// The folder holding the app's files, resolved once per process.
pub fn data_dir() -> PathBuf {
    data_resolution().directory
}

/// One of the app's files inside the data folder.
pub fn data_file(name: &str) -> PathBuf {
    data_dir().join(name)
}

/// Write `contents` to `path` atomically: a temp file in the same
/// folder is written, flushed to disk (`sync_all`), then renamed
/// over the target — a crash mid-save can never leave a truncated
/// file behind. Creates the parent folder when needed.
pub fn atomic_write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = parent {
        std::fs::create_dir_all(dir)?;
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "calcdex".to_string());
    let tmp_name = format!(".{file_name}.tmp-{}", std::process::id());
    let tmp = match parent {
        Some(dir) => dir.join(tmp_name),
        None => PathBuf::from(tmp_name),
    };
    let write = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if write.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write
}

/// The files a purge (`--clean` / `--uninstall`) would remove from
/// `dir`: exactly the app's known files that exist there. Pure —
/// touches nothing.
pub fn purge_candidates(dir: &Path) -> Vec<PathBuf> {
    KNOWN_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|p| p.exists())
        .collect()
}

#[cfg(all(test, windows))]
mod migration_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture {
        root: PathBuf,
        exe: PathBuf,
        old: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "calcdex-migration-b-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let exe = root.join("app");
            let old = root.join("legacy");
            fs::create_dir(&exe).unwrap();
            fs::create_dir(&old).unwrap();
            for (i, name) in KNOWN_FILES.iter().enumerate() {
                fs::write(old.join(name), [i as u8, 0, 255]).unwrap();
            }
            Self { root, exe, old }
        }
        fn resolve(&self, hook: &mut impl FnMut(&str) -> std::io::Result<()>) -> DataResolution {
            resolve_with(None, Some(self.exe.clone()), Some(self.old.clone()), hook)
        }
        fn restart(&self) -> DataResolution {
            resolve_locations(None, Some(self.exe.clone()), Some(self.old.clone()))
        }
        fn originals(&self) {
            for (i, name) in KNOWN_FILES.iter().enumerate() {
                assert_eq!(fs::read(self.old.join(name)).unwrap(), [i as u8, 0, 255]);
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
    fn failure() -> std::io::Error {
        std::io::Error::other("injected filesystem failure")
    }

    #[test]
    fn stage_cannot_be_renamed_or_substituted_before_copy_publish_or_cleanup() {
        for point in ["copy:settings.txt", "publish", "cleanup"] {
            let f = Fixture::new();
            let mut reached = false;
            let result = f.resolve(&mut |step| {
                if step == point {
                    reached = true;
                    let stage = fs::read_dir(&f.exe)?.next().unwrap()?.path();
                    assert!(fs::rename(&stage, f.exe.join("displaced")).is_err());
                    assert!(fs::create_dir(&stage).is_err());
                    fs::write(stage.join("foreign"), b"foreign sentinel")?;
                    return Err(failure());
                }
                if point == "cleanup" && step == "copy:settings.txt" {
                    return Err(failure());
                }
                Ok(())
            });
            assert!(reached, "{point}: {:?}", result.warning);
            assert_eq!(result.directory, f.old);
            assert!(result
                .warning
                .unwrap()
                .contains("owned staging retained at"));
            let stage = fs::read_dir(&f.exe)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(
                fs::read(stage.join("foreign")).unwrap(),
                b"foreign sentinel"
            );
            f.originals();
        }
    }

    #[test]
    fn owned_children_reject_replacement_and_cleanup_preserves_foreign_entries() {
        for point in ["flush:settings.txt", "completion:flush"] {
            let f = Fixture::new();
            let mut reached = false;
            let result = f.resolve(&mut |step| {
                if step == point {
                    reached = true;
                    let stage = fs::read_dir(&f.exe)?.next().unwrap()?.path();
                    let name = if point == "completion:flush" {
                        ".migration-complete"
                    } else {
                        "settings.txt"
                    };
                    assert!(fs::remove_file(stage.join(name)).is_err());
                    assert!(fs::rename(stage.join(name), stage.join("replaced")).is_err());
                    assert!(fs::write(stage.join(name), b"replacement").is_err());
                    fs::write(stage.join("foreign"), b"foreign sentinel")?;
                    return Err(failure());
                }
                Ok(())
            });
            assert!(reached, "{:?}", result.warning);
            assert_eq!(result.directory, f.old);
            assert!(result
                .warning
                .unwrap()
                .contains("owned staging retained at"));
            let stage = fs::read_dir(&f.exe)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(
                fs::read(stage.join("foreign")).unwrap(),
                b"foreign sentinel"
            );
            assert_eq!(fs::read_dir(stage).unwrap().count(), 1);
            f.originals();
        }
    }

    #[test]
    fn every_copy_flush_completion_and_publication_failure_restarts_from_latest_legacy() {
        let mut failures: Vec<String> = KNOWN_FILES
            .iter()
            .flat_map(|name| [format!("copy:{name}"), format!("flush:{name}")])
            .collect();
        failures.extend(["completion:create", "completion:flush", "publish"].map(String::from));
        for point in failures {
            let f = Fixture::new();
            let mut seen = false;
            let result = f.resolve(&mut |step| {
                if step == point {
                    seen = true;
                    Err(failure())
                } else {
                    Ok(())
                }
            });
            assert!(seen, "{point}");
            assert_eq!(result.directory, f.old, "{point}");
            assert!(result.warning.is_some());
            assert!(!f.exe.join("data").exists());
            assert_eq!(
                fs::read_dir(&f.exe).unwrap().count(),
                usize::from(point == "publish")
            );
            if point == "publish" {
                assert!(result
                    .warning
                    .as_ref()
                    .unwrap()
                    .contains("owned staging retained at"));
                let stage = fs::read_dir(&f.exe)
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                assert!(stage.join(".migration-complete").is_file());
                for name in KNOWN_FILES {
                    assert_eq!(
                        fs::read(stage.join(name)).unwrap(),
                        fs::read(f.old.join(name)).unwrap()
                    );
                }
            }
            f.originals();
            fs::write(f.old.join("history.txt"), b"latest legacy edit").unwrap();
            let restarted = f.restart();
            assert_eq!(restarted.directory, f.exe.join("data"));
            assert!(restarted.warning.is_none());
            assert_eq!(
                fs::read(restarted.directory.join("history.txt")).unwrap(),
                b"latest legacy edit"
            );
            assert!(restarted.directory.join(".migration-complete").is_file());
            assert_eq!(f.restart().directory, restarted.directory);
        }
    }

    #[test]
    fn successful_transaction_preserves_all_bytes_and_optional_absence() {
        let f = Fixture::new();
        f.originals();
        fs::remove_file(f.old.join("examples.txt")).unwrap();
        let result = f.restart();
        assert!(result.warning.is_none(), "{:?}", result.warning);
        for name in KNOWN_FILES
            .into_iter()
            .filter(|name| *name != "examples.txt")
        {
            assert_eq!(
                fs::read(f.old.join(name)).unwrap(),
                fs::read(result.directory.join(name)).unwrap()
            );
        }
        assert!(!result.directory.join("examples.txt").exists());
    }

    #[test]
    fn overrides_and_existing_portable_are_authoritative_without_copying() {
        let f = Fixture::new();
        let override_dir = f.root.join("override");
        let result = resolve_locations(
            Some(override_dir.clone()),
            Some(f.exe.clone()),
            Some(f.old.clone()),
        );
        assert_eq!(result.directory, override_dir);
        assert!(!override_dir.exists());
        fs::create_dir(f.exe.join("data")).unwrap();
        fs::write(f.exe.join("data/history.txt"), b"portable authority").unwrap();
        assert_eq!(f.restart().directory, f.exe.join("data"));
        assert_eq!(
            fs::read(f.exe.join("data/history.txt")).unwrap(),
            b"portable authority"
        );
        assert!(!f.exe.join("data/settings.txt").exists());
        f.originals();
    }

    #[test]
    fn existing_empty_or_foreign_destination_is_preserved() {
        for foreign in [false, true] {
            let f = Fixture::new();
            fs::create_dir(f.exe.join("data")).unwrap();
            if foreign {
                fs::write(f.exe.join("data/sentinel"), b"foreign").unwrap();
            }
            let result = f.restart();
            assert_eq!(result.directory, f.old);
            assert!(result.warning.unwrap().contains("already exists"));
            if foreign {
                assert_eq!(fs::read(f.exe.join("data/sentinel")).unwrap(), b"foreign");
            }
            f.originals();
        }
    }

    #[test]
    fn windows_publication_never_replaces_a_racing_empty_or_foreign_directory() {
        for foreign in [false, true] {
            let f = Fixture::new();
            let result = f.resolve(&mut |step| {
                if step == "publish" {
                    fs::create_dir(f.exe.join("data"))?;
                    if foreign {
                        fs::write(f.exe.join("data/sentinel"), b"race winner")?;
                    }
                }
                Ok(())
            });
            assert_eq!(result.directory, f.old);
            assert!(result.warning.is_some());
            assert!(!f.exe.join("data/history.txt").exists());
            assert_eq!(f.restart().directory, f.old);
            if foreign {
                assert_eq!(
                    fs::read(f.exe.join("data/sentinel")).unwrap(),
                    b"race winner"
                );
            }
            f.originals();
        }
    }

    #[test]
    fn retained_owned_staging_is_reported_and_ignored_on_restart() {
        let f = Fixture::new();
        let result = f.resolve(&mut |step| {
            if step == "copy:history.txt" || step == "cleanup" {
                Err(failure())
            } else {
                Ok(())
            }
        });
        assert_eq!(result.directory, f.old);
        assert!(result
            .warning
            .unwrap()
            .contains("owned staging retained at"));
        assert_eq!(fs::read_dir(&f.exe).unwrap().count(), 1);
        fs::write(f.old.join("settings.txt"), b"current").unwrap();
        let result = f.restart();
        assert_eq!(
            fs::read(result.directory.join("settings.txt")).unwrap(),
            b"current"
        );
    }

    #[test]
    fn source_writes_and_replacement_are_denied_while_copying() {
        let f = Fixture::new();
        let result = f.resolve(&mut |step| {
            if step == "copy:history.txt" {
                assert!(fs::write(f.old.join("settings.txt"), b"changed").is_err());
                assert!(fs::remove_file(f.old.join("history.txt")).is_err());
            }
            Ok(())
        });
        assert!(result.warning.is_none());
        f.originals();
    }

    #[test]
    fn legacy_junction_is_rejected_without_following_or_copying() {
        let f = Fixture::new();
        let junction = f.root.join("junction");
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&f.old)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = resolve_locations(None, Some(f.exe.clone()), Some(junction.clone()));
        assert_eq!(result.directory, junction);
        assert!(result.warning.is_some());
        assert!(!f.exe.join("data").exists());
        f.originals();
        fs::remove_dir(junction).unwrap();
    }

    #[test]
    fn nonregular_source_is_rejected_without_publication() {
        let f = Fixture::new();
        fs::remove_file(f.old.join("history.txt")).unwrap();
        fs::create_dir(f.old.join("history.txt")).unwrap();
        let result = f.restart();
        assert_eq!(result.directory, f.old);
        assert!(result.warning.is_some());
        assert!(!f.exe.join("data").exists());
    }
}
