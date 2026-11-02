# Compatibility

## Program

- **Windows 10 and Windows 11, 64-bit (x64)** — the shipped
  builds. Releases ship x64 only (see
  [CHANGELOG.md](CHANGELOG.md), 0.9.0).
- The C runtime is linked statically, so no separate
  runtime or redistributable install is needed.
- No administrator rights are needed: the installer is
  per-user, and the portable build installs nothing.
- The source also builds and runs on Linux for development;
  only the Windows build is shipped.

## Building from source

- Rust **1.80 or newer** (`rust-version` in `Cargo.toml`),
  stable toolchain with `rustfmt` and `clippy`.
- Dependencies: `egui`/`eframe` 0.33 only.

## Data files

- Settings, the calculation log and saved examples are
  **plain-text files** in the `data` folder beside the
  executable (see [MIGRATION.md](MIGRATION.md)) — readable
  and editable in any text editor, and portable with the
  program folder.
- Settings carry `schema_version=1`. Files written by
  versions before 0.9.0 (no version line) load unchanged;
  files from a newer CalcDex are never overwritten by an
  older one (see [MIGRATION.md](MIGRATION.md)).
- Log files written by every version since v0.1.1 read
  unchanged.
