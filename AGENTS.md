# CalcDex — Project Rules (condensed from Adaptive Engineering Contract V4.0)

## Project Manifest
- tier: 3 (rich Windows GUI)
- tier rationale: egui/eframe multi-pane calculator suite (mode rail, calculator strip, form + steps pane, history rail) with layout well beyond stock controls. Note: egui is not named in the contract's §6 renderer list (Slint, Direct2D, Godot, raylib); the t3 placement follows the adopted classification companion (`engineering-contract-v4.0-classification.md`), which lists CalcDex under t3.
- targets: windows-x64
- toolchain: Rust 2021, eframe/egui 0.33 (glow renderer), crt-static
- renderer: egui immediate-mode GUI over glow; static runtime linkage
- release assets: CalcDex-v{VERSION}-x64-Setup.exe + CalcDex-v{VERSION}-x64-Portable.zip + SHA256SUMS.txt (the exe installer ships with every desktop release; no MSI, ever — SEAN's amendment)
- tier overrides: idle RAM cap 40 MB (t3 rule); LICENSE kept under that exact name at the repo root for GitHub license detection — a documented deviation from the standard docs set's LICENSE.md
- project records: PROJECT_RULES.md (Elite rule sheet), DECISIONS.md and QUALITY.md remain the project's own records and are not replaced by this file
- dropped: Sections 3, 4, 5, 7, 8 (other tiers); non-Windows CI steps
- condensed from: Adaptive Engineering Contract V4.0 (2026-11-01, with SEAN's five amendments)

## Universal mandates (§2 — Class C, never violated silently)
- Zero cloud / zero telemetry: 100% functional air-gapped; no telemetry endpoints, license servers, crash reporters, or phone-home. The one network path is the offer-only update check, off by default, user-triggered or explicitly switched on, which downloads nothing.
- Respect the user: no unprompted toasts, promotional links, browser-default changes, or unapproved startup entries. Offline help in the window (F1) and via `--help`. Everything the program writes lives in one documented location (`data/` beside the executable; see the manifest note in docs/MIGRATION.md).
- Config schema versioning: persistent files carry a `schema_version`; loaders migrate upward explicitly and never silently overwrite or reset user data; every schema bump ships a tested migration fixture.
- Token economy: complete files when new; surgical diffs for edits; status in concise bullets.

## Tier 3 rules (§6, as they apply here)
- Static runtime linkage mandatory (crt-static); zero Electron/web runtimes.
- Idle RAM < 40 MB; UI thread < 16 ms; no blocking first frame.
- Mockup/wireframe confirmation before new top-level windows or primary layout changes.
- Every capability in the window is also usable from the command line (the registry in `src/lib.rs` is the single dispatch; `src/main.rs` is GUI + CLI shell only). CLI maintenance commands: `--help`, `--version`, `--clean`, `--uninstall`, with `--dry-run`/`--silent`/`--json` modifiers.
- Single-instance: GUID-prefixed named mutex `Global\CalcDex-{11A5D674-3C31-43A1-A932-6D514EF598BE}` for window launches.
- Disk mutations to the app's own files are atomic: temp file → flush (`sync_all`) → rename over the target.
- `unwrap()`/`expect()` banned in hot paths and untrusted-input code (calculator inputs and file contents are untrusted); errors surface as plain-words messages, never panics.
- Releases (tag + GitHub Release) require explicit human confirmation; branches/PRs/commits do not. CI runs the full gate (fmt check, clippy `-D warnings`, full tests) and the tier lint before any release; asset names match the matrix exactly; checksums in one pass. No winget, ever.
