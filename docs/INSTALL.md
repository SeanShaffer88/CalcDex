# Install

Every release ships the same three files (Windows 10/11,
64-bit):

- `CalcDex-v{version}-x64-Setup.exe` — the installer.
- `CalcDex-v{version}-x64-Portable.zip` — the portable build.
- `SHA256SUMS.txt` — the SHA-256 checksum of both downloads.

## Check the download

Compare the SHA-256 of your downloaded file with the line
for it in `SHA256SUMS.txt` (for example with
`Get-FileHash <file>` in PowerShell). The program is
unsigned — there is no code-signing certificate — so Windows
may warn on first run; choose **More info**, then **Run
anyway**, only if the checksum matches and you trust the
source.

## Installer

Run `CalcDex-v{version}-x64-Setup.exe`. It installs per-user
(no administrator rights) to
`%LOCALAPPDATA%\Programs\CalcDex`, adds Start Menu and
desktop shortcuts, and registers an uninstaller. Upgrades
install over the top; your files are kept (they live in the
`data` folder, see [MIGRATION.md](MIGRATION.md)).

**Uninstall:** Settings → Apps, or run the installer's
`Uninstall.exe`. To also delete your settings, log and
examples, run `CalcDex.exe --uninstall` before removing the
program (it removes the data files and explains the rest),
or delete the `data` folder afterwards.

## Portable

Unzip `CalcDex-v{version}-x64-Portable.zip` anywhere and run
`CalcDex.exe`. Nothing is installed; the program keeps its
files in a `data` folder beside the executable. To remove
it, run `CalcDex.exe --clean` if you want the data gone
first, then delete the folder.
