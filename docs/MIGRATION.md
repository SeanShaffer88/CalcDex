# Migration

## v0.9.0 — files moved beside the program

Before 0.9.0, CalcDex kept its files in
`%APPDATA%\CalcDex\`. From 0.9.0 they live in a **`data`
folder beside `CalcDex.exe`**, so a portable install is
fully self-contained and an installed build keeps everything
in one known place.

What moved:

| File | Contents |
| --- | --- |
| `settings.txt` | Favourites, recent calculators, remembered inputs, display choices, your constants, projection/demo preferences, saved window position and size. |
| `history.txt` | The calculation log. |
| `history-backup.txt` | The backup written before the log is cleared. |
| `history-edit-backup.txt` | The backup written before a log entry is edited. |
| `examples.txt` | Saved class examples. |

How the move works:

1. On first run, if the `data` folder already holds any of
   these files, it is used as-is.
2. Otherwise, existing known files in `%APPDATA%\CalcDex`
   are **copied** into an exclusively created sibling staging directory.
   Each copy and a completion marker are flushed with `sync_all` before
   the whole directory is published as `data\`. Windows publication
   refuses any existing destination, including one created concurrently.
   Missing optional files are valid. The originals are left
   untouched — the copy is one-way, and going back to an
   older version simply uses the old files again (it will
   not see changes made since, so treat the move as
   one-directional in practice).
3. If `data\` cannot be created or the copy fails (for
   example the program sits in a read-only folder), the old
   `%APPDATA%\CalcDex` location is used for that run —
   an existing user's files are never lost to a failed move.

An existing empty or foreign-only `data\` is preserved; migration falls
back to legacy storage with a warning rather than merging or deleting entries.
Reparse points in migration paths and nonregular legacy entries are refused.
Source files are held against writes/deletion while copying on Windows.
Failed copies, flushes, completion recording or publication leave legacy
storage selected. A new process retries from the current legacy files;
sibling staging remnants are never selected. Staging is created atomically with its owned Windows handle. Outputs are
created relative to that handle and held against replacement until flushed.
Publication uses the same staging handle with a fully qualified target under
the pinned parent, without replacing an existing destination. Child handles
close before publication; a subsequent failure retains staging and warns.
Earlier failures delete only still-owned child handles and the empty staging
handle. Foreign entries are retained with a warning; cleanup never recursively
deletes or reopens paths to infer ownership. The retained staging path is
reported through the warning. This assumes trusted stable ancestors; handle
pinning does not prove containment against malicious same-token attribute
rewriting.

`paths::data_resolution()` exposes the cached directory and optional warning;
`paths::resolve_locations()` permits independent resolution with explicit paths.
The current GUI does not display this additive warning seam. These operations
do not change calculator behavior or file schemas. File flushes and atomic
publication do not constitute a tested guarantee against sudden power loss.

**Manual recovery for older partial migrations:** an unmarked `data\` holding
any known file remains authoritative for compatibility with portable installs.
It cannot reliably be distinguished from a partial copy left by an older
version. CalcDex never automatically merges or deletes it. Close CalcDex,
back up both directories, compare their known files, and select the desired
complete data set manually (or use the override). Preserve foreign files.

The `CALCDEX_DATA_DIR` environment variable, when set,
points the data folder at an exact location instead (used
by scripts and the test suite).

Old log entries pinned in the log or backups are unaffected:
the file formats did not change.

## Settings format version 1

`settings.txt` now begins with a `# CalcDex settings`
header and a `schema_version=1` line.

- A settings file **without** the line (everything written
  before 0.9.0) is version 0: it loads exactly as before —
  including the legacy `sig_figs=` line — and is rewritten
  as version 1 on the next save.
- A file declaring a **newer** version than the running
  program understands loads as defaults with a warning on
  the console, and the program refuses to overwrite it, so
  a newer CalcDex's settings survive an older build.
