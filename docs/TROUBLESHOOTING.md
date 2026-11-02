# Troubleshooting

## Windows warns that the program is unsigned

CalcDex has no code-signing certificate, so SmartScreen may
warn on first run. Verify the download's SHA-256 against
`SHA256SUMS.txt` first (see [INSTALL.md](INSTALL.md)); if it
matches, choose **More info** → **Run anyway**.

## "CalcDex is already running"

Only one window can be open at a time (a named mutex guards
the data files). A second launch prints that note and
exits — switch to the open window. If no window is visible,
check the taskbar and Task Manager for a running
`CalcDex.exe` before assuming a stale lock: the name is
released automatically when the process ends.

## `--clean` / `--uninstall` refuse to run

They refuse while a CalcDex window is open, because the
running program holds the files. Close the window and run
the command again.

## My settings, log or examples seem missing after upgrading

From v0.9.0 the files live in the `data` folder beside
`CalcDex.exe`, copied from `%APPDATA%\CalcDex` on first run
(see [MIGRATION.md](MIGRATION.md)). If the program folder is
read-only, CalcDex keeps using the old location instead —
check both places. `CalcDex --log` shows the log the program
currently sees.

## A warning about settings from a newer version

If `settings.txt` declares a `schema_version` newer than
the running build understands, the build loads defaults and
warns instead of touching the file. Run the newer build, or
move the file aside to start fresh — the file itself is
never overwritten in that state.

## I cleared the log by accident

`CalcDex --restore-log` undoes the last clear (a backup is
written before every clear and before every log edit).

## Something else

The built-in help (F1 in the window, `CalcDex --help-guide`
on the command line) covers everyday use. For the rest, see
[SUPPORT.md](SUPPORT.md).
