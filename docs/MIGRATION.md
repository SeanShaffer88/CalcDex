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
2. Otherwise, any of the files found in `%APPDATA%\CalcDex`
   are **copied** into `data\`. The originals are left
   untouched — the copy is one-way, and going back to an
   older version simply uses the old files again (it will
   not see changes made since, so treat the move as
   one-directional in practice).
3. If `data\` cannot be created or the copy fails (for
   example the program sits in a read-only folder), the old
   `%APPDATA%\CalcDex` location is used for that run —
   an existing user's files are never lost to a failed move.

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
