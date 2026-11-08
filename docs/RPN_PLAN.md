# RPN / Programmable Mode — Architecture & UX Plan

Status: P1 done (RPN core + `rpn` CLI, f64 numbers). P2–P13 not
started. Progress: 1 of 13 phases (~8%, rough, phases differ a lot
in size). No ETA: P4–P6 and P10–P12 are the big ones.
Decision: scope (b) — an HP-42S-compatible RPN/programmable mode
inside CalcDex, then the Plus42-style extras. Recorded in DECISIONS.md.

## Goal
Offer the functionality of Free42 (HP-42S simulator) and Plus42
(Free42 + algebraic equations, units, directories, TVM, plotting)
as a new **RPN** mode of CalcDex, alongside the 316 registry
calculators. Targets: HP-42S program compatibility and Plus42
feature coverage, claimed only as far as tests prove.

## Clean-room rule
Free42/Plus42 are GPLv2. CalcDex is MIT. No Free42/Plus42 source is
read, copied or translated. Behaviour comes from the published
HP-42S Owner's Manual, HP-42S Programming Examples, the Free42 /
Plus42 public documentation pages, and black-box comparison of
program results. Test programs must be public-domain or written
here. Any contributor who has read Free42 source is asked to say so
in the PR.

## Why a new engine
The registry signature `Fn(&[f64]) -> Result<String,String>` is
stateless, scalar and f64-only. The RPN machine is stateful (stack,
registers, programs, flags) and typed (real, complex, string,
matrix, list, later unit/equation). It lives beside the registry:

```
src/rpn/
  mod.rs       public API: Session, Key/Op dispatch, text front end
  num.rs       Num (real) — f64-backed first, 34-digit decimal later
  value.rs     enum Value { Real, Complex, Str, Matrix, List, ... }
  stack.rs     4-level stack + LASTX, optional growing "big stack"
  ops/         one file per family: arith, trig, stats, base, time,
               matrix, complex, string, list, flags, control
  display.rs   FIX/SCI/ENG/ALL, RDX./RDX, separators, base views
  prog.rs      program memory, labels, listing parse/print
  run.rs       interpreter: GTO/XEQ/RTN, flags, local vars, SST/BST
  solve.rs     SOLVE, integrator (programmable + equation based)
  state.rs     versioned state file, atomic save, import/export
  eqn.rs       (Plus42 phase) equation parser/evaluator
  units.rs     (Plus42 phase) attached units
```

`src/main.rs` stays shell-only. The registry is untouched except for
one new text entry point, `run_rpn(session, line)`, so the window
and CLI share one dispatch (AGENTS.md invariant).

## Invariants (inherit AGENTS.md)
- No `unwrap()`/`expect()` on user input or file content; every
  error is a plain-words message with a stable code.
- State, programs and settings live under `data/`, versioned
  (`schema_version`), written atomically, with a migration fixture
  for every bump. Unparseable state is quarantined, never reset.
- Interpreter is bounded: step/time limit with a cancel check
  (abort < 50 ms), recursion depth cap, memory cap.
- Offline, zero telemetry, idle RAM < 40 MB, UI thread < 16 ms
  (long programs run on a worker thread).
- Every capability reachable from the CLI.

## CLI surface
```
CalcDex rpn "2 3 + 4 *"            one-shot, prints X (and stack with --stack)
CalcDex rpn --state NAME ...       use/save a named state
CalcDex rpn --run prog.txt         load a listing and XEQ it
CalcDex rpn --listing prog.txt     print/validate a listing
CalcDex rpn --json ...             machine-readable result
```
Input tokens: numbers (incl. `1.5e-3`), HP-42S mnemonics (`ENTER`,
`STO 01`, `RCL "A"`, `XEQ "NAME"`), and `PRGM` lines.

## Numbers
- Phase 1: `Num` wraps f64 (Free42 "Binary" equivalent). Documented
  difference: 0.1 inexact.
- Phase 2: `Num` becomes a 34-digit decimal (IEEE 754-2008
  decimal128 semantics: 34 digits, exponent ±6144), pure Rust,
  no C dependency. Transcendentals are implemented with guard
  digits and verified against high-precision reference vectors.
  A runtime/setting switch keeps both modes, as Free42 does.
- Risk: correct rounding of sin/exp/ln at 34 digits is the hardest
  single item; if it slips, decimal mode ships with arithmetic,
  sqrt and powers exact and transcendentals documented as
  ≤ 1 ulp-ish with measured error, never claimed correctly rounded.

## Phases (each = one PR, full gate green before merge)
| # | Scope | Done when |
|---|---|---|
| P1 | RPN core: stack, LASTX, entry, arithmetic and scientific functions, STO/RCL + named variables, FIX/SCI/ENG/ALL, `rpn` CLI | HP-42S manual chapter 2–3 examples pass |
| P2 | 34-digit decimal `Num`, Binary/Decimal switch | reference-vector suite; 0.1+0.2 exact |
| P3 | Complex (rect/polar), strings/ALPHA, lists | manual complex examples pass |
| P4 | Matrices: edit, inverse, det, transpose, row ops, linear solve, complex matrices | known-answer matrix suite |
| P5 | Programming: memory, labels, GTO/XEQ/RTN, tests, flags, loops (ISG/DSE), local vars (LSTO), SST/BST, listing import/export + clipboard | public-domain 42S programs run to expected output |
| P6 | Numerical SOLVE and integrator (programmable) | manual solver/integrator examples |
| P7 | Statistics (Σ+, regressions, forecasts), BASE (word size, bit ops), time/date functions | known-answer suites; dates 1582-10-15..4320-09-10 |
| P8 | State files, states manager, full copy/paste formats (scalars, matrices tab-delimited, programs) | round-trip + migration fixtures |
| P9 | GUI: keypad + display + program view + variables, as a mode on the mode rail | wireframe approved (below); screenshots from real run |
| P10 | Plus42: EQN parser/evaluator, direct+numeric solver, equation integration | equation suite |
| P11 | Plus42: attached units + catalog, TVM/amortization, directories | unit and TVM known answers |
| P12 | Plus42: XY plotting + plot viewer, big display options | plot export + visual check on Windows |
| P13 | Compatibility pass: corpus of 42S programs, verification matrix, docs | claims in README match tests |

Out of scope unless asked: HP-82240 printer emulation, skin files,
accelerometer/GPS/compass, Android/iOS builds (CalcDex is
windows-x64).

## UX
- New **RPN** mode on the mode rail (same Terminal theme, radius 0,
  Cascadia Mono). Centre: display (stack levels, X highlighted,
  annunciators: RAD/DEG, FIX/SCI, shift, PRGM, flags), keypad,
  menu row (soft keys), optional program/variables panes. History
  rail shows the tape.
- Keyboard first: physical keys map to RPN keys; Enter = ENTER;
  Esc = cancel/abort program; Tab order logical; every key has a
  tooltip (action + side effects).
- Window geometry and splitters persist (already supported).
- Accessibility: focus visuals, accessible names on every key,
  contrast measured to WCAG AA, no colour-only annunciators.
- Wireframe sign-off is required before P9 (AGENTS.md gate); P1–P8
  have no UI work.

## Testing / verification
- Known-answer vectors per op; property tests for stack invariants.
- Program corpus with expected outputs (written here or
  public-domain).
- Failure drills: bad state file, truncated listing, runaway loop,
  deep recursion, huge matrix, cancel mid-run.
- Evidence labels (Verified / Partially verified / Unverified) in
  QUALITY.md; on-Windows GUI checks stay Unverified until run.

## Open questions
1. Wireframe approval (P9).
2. Decimal precision fallback acceptable per the risk note above?
3. Keep Free42-compatible `.f42` state files? Default: no — own
   versioned format, with listing import/export for programs.

## Known limitations after P1 (Unverified against real HP-42S)
- SQRT of a negative real, LN of 0 or less, etc. give Invalid Data;
  real hardware/Free42 may return complex results. Revisit in P3.
- `0 0 Y^X` returns 1 (IEEE); hardware behaviour not checked.
- `MOD` with divisor 0 is an error here; hardware not checked.
- ENG n shows n+1 significant digits and no trailing point
  (`12E-3`); the HP-42S look may differ.
- No registers 00-99 limit, no SIZE, no complex/strings/matrices yet.
- Behaviour checked only by this repo's tests, not by running an
  HP-42S or Free42.
