# Architecture

CalcDex is a single Rust crate: a library that does all the
work, plus a thin binary shell. The design rule is
**everything is an operation**: every calculator is one entry
in the library's registry, and the window and the command line
dispatch through that same registry, so there are no
window-only calculators (the capability matrix is
[PARITY.md](PARITY.md)).

## Layout

| Part | What it holds |
| --- | --- |
| `src/lib.rs` | The calculator registry (`all_calcs`, `run_calc`, `run_text_calc`), the calculation log (load/save, backup/restore, search/filter/pin) and shared formatting. |
| `src/calcs/` | The calculators themselves, one module per area. Each entry is a pure function from parsed numbers (or text) to a result string — errors are returned as text, never panics. |
| `src/meta.rs` | Central metadata: field labels, hints, examples, glossaries, formulas, course packs. |
| `src/settings.rs` | The settings file: parsing, the `schema_version` handling and migration notes, favourites/recent/remembered inputs, user constants, saved window geometry. |
| `src/paths.rs` | Where the program's files live: the `data` folder beside the executable, the one-way migration from the pre-0.9.0 location, the `CALCDEX_DATA_DIR` override, and the atomic-write helper every save uses. |
| `src/teachers.rs` | Problem sets, answer keys, marking, formula sheets, saved class examples. |
| `src/students.rs` | Practice problems, answer checking, deeper steps, worked-solution text. |
| `src/rigour.rs` | Display/copy formats, spec checks, the constants table, the built-in help text, the offer-only update check. |
| `src/engineers.rs`, `src/features.rs` | The materials table and shared run helpers (batch CSV, vary/sweep, search ranking). |
| `src/main.rs` | The shell only: the egui window (views over library state) and the CLI front end (argument parsing that ends in the same registry calls). |
| `src/win32.rs` | The single-instance mutex (raw `CreateMutexW` FFI, RAII guard) for window launches. |

## Invariants

- **One dispatch.** GUI and CLI call the same registry
  functions; a capability added to the registry exists in
  both surfaces at once.
- **Files in one place.** Everything the program writes
  lives in the data folder resolved by `src/paths.rs`;
  saves are atomic (temporary file → flush → rename over).
- **Errors as text.** Calculator input is untrusted: bad
  input produces a plain-words error message, not a crash.
- **Offline.** The library has no networking except the
  opt-in update check in `src/rigour.rs`, which downloads
  nothing.

## Project records

- [../PROJECT_RULES.md](../PROJECT_RULES.md) — the project's
  rule sheet (frozen foundation, coverage floors).
- [../DECISIONS.md](../DECISIONS.md) — the decision log.
- [../QUALITY.md](../QUALITY.md) — the quality scorecard.
- [PARITY.md](PARITY.md) — GUI/CLI capability parity.
- [COVERAGE.md](COVERAGE.md) — test-corpus coverage notes.
- [AUDIT.md](AUDIT.md) — the standing audit record.
- [../AGENTS.md](../AGENTS.md) — the condensed engineering
  contract for this repo (tier t3).
