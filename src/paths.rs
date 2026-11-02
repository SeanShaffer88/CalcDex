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

fn dir_holds_known_files(dir: &Path) -> bool {
    KNOWN_FILES.iter().any(|f| dir.join(f).exists())
}

/// Resolve the data folder (see the module docs for the order).
/// Performs the one-way legacy migration copy on the way through
/// when that is what puts `data/` into use.
fn resolve_data_dir() -> PathBuf {
    // Step 0: the explicit override wins outright.
    if let Ok(dir) = std::env::var("CALCDEX_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let legacy = legacy_dir();
    if let Some(exe_dir) = &exe_dir {
        let data = exe_dir.join("data");
        if dir_holds_known_files(&data) {
            return data;
        }
        if let Some(legacy) = &legacy {
            if dir_holds_known_files(legacy) {
                // One-way copy migration: new folder absent, old
                // folder in use. Copy what exists; the originals
                // stay untouched as the backup of the move.
                if std::fs::create_dir_all(&data).is_ok() {
                    let mut copied = false;
                    let mut failed = false;
                    for name in KNOWN_FILES {
                        let from = legacy.join(name);
                        let to = data.join(name);
                        if from.exists() && !to.exists() {
                            match std::fs::copy(&from, &to) {
                                Ok(_) => copied = true,
                                Err(_) => failed = true,
                            }
                        }
                    }
                    if copied && !failed {
                        return data;
                    }
                }
                // data/ unusable: stay on the legacy folder rather
                // than strand the user's files.
                return legacy.clone();
            }
        }
        // Fresh install: prefer data/ beside the executable.
        if std::fs::create_dir_all(&data).is_ok() {
            return data;
        }
    }
    if let Some(legacy) = legacy {
        return legacy;
    }
    if let Some(exe_dir) = exe_dir {
        return exe_dir;
    }
    PathBuf::from(".")
}

/// The folder holding the app's files (see module docs). Resolved
/// once per process, migration included.
pub fn data_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(resolve_data_dir).clone()
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
