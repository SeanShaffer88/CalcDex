# Security

## Design posture

- **Offline by design.** CalcDex has no accounts, no
  telemetry, no analytics and no crash reporting. The single
  network feature is the update check: off by default, and
  even when on (or run by hand with `--check-update`) it
  only reads the latest release version from GitHub and
  downloads nothing — anything it offers is fetched by you,
  from the release page.
- **Your data stays yours.** Settings, the calculation log
  and saved examples are plain-text files in one place (the
  `data` folder beside the executable, see
  [MIGRATION.md](MIGRATION.md)). Nothing is uploaded or
  synced anywhere.
- **No privileges.** The program runs without administrator
  rights and the installer is per-user.
- **Input is untrusted.** Calculator inputs and the contents
  of the data files are validated as they are parsed; bad
  input produces a plain-words error, not a crash. Data-file
  saves are atomic (write to a temporary file, flush, rename
  over), so an interrupted save cannot corrupt the previous
  file.
- **Unsigned builds.** Release binaries are not
  code-signed. Verify downloads against `SHA256SUMS.txt` on
  the release page (see [INSTALL.md](INSTALL.md)).

## Reporting a problem

Please report suspected security problems privately through
GitHub's security advisory flow on the repository
(<https://github.com/SeanShaffer88/CalcDex/security/advisories>)
rather than as a public issue.
