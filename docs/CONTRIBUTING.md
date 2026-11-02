# Contributing

Contributions are welcome — bug reports, calculator
corrections, and new calculators included. Acceptance is at
the maintainer's discretion (see
[GOVERNANCE.md](GOVERNANCE.md)).

## Ground rules

- **Everything is an operation.** A calculator is one entry
  in the registry in `src/lib.rs` (calculators themselves
  live in `src/calcs/`), which makes it available in the
  window and on the command line at once. Window-only
  features are not accepted; see
  [ARCHITECTURE.md](ARCHITECTURE.md) and
  [../PROJECT_RULES.md](../PROJECT_RULES.md).
- **Errors are text.** A calculator returns a plain-words
  error string for bad input — never a panic. No
  `unwrap()`/`expect()` on untrusted input.
- **Plain text stays plain text.** Data files keep their
  documented formats; a format change needs a
  `schema_version` bump, a migration, and a fixture test
  (the pattern is in `src/settings.rs` and
  `tests/v090.rs`).

## The gate

Run the full check before proposing a change — the same one
CI runs:

```text
cargo fmt --all
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

New behaviour ships with tests in `tests/` (one file per
release batch is the house pattern), and user-visible
changes get a [CHANGELOG.md](CHANGELOG.md) entry.

## Building

```text
cargo build --release
```

needs Rust 1.80+ and produces `target/release/CalcDex.exe`
on Windows. See [COMPATIBILITY.md](COMPATIBILITY.md).
