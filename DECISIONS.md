# Decisions — CalcDex

## 2026-10-25 — Renamed CalcCodex → CalcDex
SEAN's order: "rename repo project and program to CalcDex". The rename
applies everywhere the program/project is named: the directory
(~/workspace/calcdex), the Cargo package (calcdex), the binary
(CalcDex.exe), the window title/header (CALCDEX), the CLI name and help
text, the installer, and all docs. Calculator ids (e.g.
compound-interest) and formulas do not change. v0.1.0 was built and
field-tested locally under the old name CalcCodex and renamed before
any GitHub publication; the first published release is CalcDex
v0.1.1. Historical mentions of the old name survive only in this note
and the CHANGELOG rename note.

## 2026-10-25 — Icon: option 1 (calculator outline)
SEAN was shown a six-option icon contact sheet for CalcDex. He first
tapped Icon 2, then immediately sent "1" as his final word — the FINAL
pick is **Icon 1**, superseding 2. Icon 1 is the calculator-outline
design: phosphor green (#4ade80) calculator outline with a dark-green
display screen and a 3x3 button grid plus a wide bottom bar, on
near-black (#0c0c0c), sharp corners. The production artwork
(assets/icon.ico, 16–256 px, plus assets/icon-64.rgba for the window)
was redrawn cleanly from the concept — the concept sheet's overlap
glitch, where the wide bottom row crossed the third button row, was
fixed with even spacing. The icon is wired into the window/taskbar
(eframe with_icon), the .exe resource itself (build.rs + windres,
.rsrc section), and the NSIS installer/uninstaller.

## 2026-10-25 — Smarter dates + calculation log (v0.1.1)
From SEAN's field test of v0.1.0: date entry as separate year/month/
day boxes with "On year" wording was odd. Decided: one MM/DD/YYYY box
per date (explicit date-group metadata on the registry, internal
numeric triples unchanged), a plain-words format hint, a "Today?"
button beside every date field (Age End Date and Countdown From Date
pre-filled), the CLI accepts a date as one MM/DD/YYYY argument (the
six-number form still works identically), and a persistent
calculation log shared by GUI and CLI (%APPDATA%\CalcDex\history.txt,
500-entry cap, --log / --clear-log, click-to-restore in the GUI).


## 2026-10-25 — Audience: academic + professional first
SEAN: "keep in mind the kind of people who will be using this program
— students, teachers, professors, scientists, engineers, etc."
Applied across v0.1: precise (not cutesy) wording; every result shows
formula + substituted values + result; full precision kept (general
number formatting carries 10 decimal places, nothing rounded away
silently); scientific-notation inputs accepted everywhere (Rust f64
parsing, documented in README + `--help`); physical constants named
with values in descriptions/outputs (g = 9.80665, R in both unit
systems, Avogadro); estimates labelled ESTIMATE; error messages teach
the reason (domain errors stated mathematically). Category and
calculator names use the vocabulary of the fields (Statistics, Gas
Laws, Thermodynamics-adjacent naming where honest).

## 2026-10-25 — Scope: calculators only
SEAN was offered three scopes (phased / calculators-only / one giant
build) and picked **calculators only**: skip Nutilz's PDF, image, text
and developer tools. Sources: Nutilz calculator subset + all of Omni
Calculator. SEAN added: "should be the ultimate tool for any and all
kinds of calculations" — maximise calculator breadth honestly.

## 2026-10-25 — Stack: Rust + egui/eframe 0.33
Same stack and versions as DevTextTools (SEAN's proven pattern, crates
already cached): pure functions in one registry dispatched identically
by GUI and CLI; fully offline; no extra dependencies at all in v0.1
(no network, no date crate — calendar maths is implemented from
Howard Hinnant's published days-from-civil algorithm and tested).

## 2026-10-25 — Parametric collapsing (counting honesty)
Omni lists a separate page per unit pair and per ingredient; Nutilz
lists a separate tool per image/PDF format pair (out of scope here).
CalcDex registers ONE converter per dimension with the units as
numbered inputs, and one grams↔cups calculator per ingredient with
its published density. The registry count is the count of these real
registered calculators — no inflated per-pair counting in either
direction. Documented in docs/COVERAGE.md.

## 2026-10-25 — Registry signature
`CalcFn = Fn(&[f64]) -> Result<String, String>`. Fixed calculators
validate argument count in `run_calc`; list calculators take any
count; pair calculators take repeating pairs. Output text always
includes the result AND the formula/steps in plain words, so GUI,
CLI and tests see identical output.

## 2026-10-25 — Theme
Terminal theme with Purgeon's exact `:root` values (fetched from
SeanShaffer88/Purgeon src/styles.css in earlier work, recorded in
memory): bg #0c0c0c, panel #121212, raised #181818, border #262626,
green #4ade80, selection #163322, text #d0d0d0, muted #7a7a7a, amber
#e5c07b, cyan #56b6c2, violet #b294f0, red #f26d6d; radius 0; UI font
forced to the Monospace family and Cascadia Mono/Code font *files*
are loaded when present on the machine, matching DevTextTools'
Segoe-font loading pattern.

## 2026-10-25 — Compound interest signature
`compound-interest` is (principal, rate %, years) with YEARLY
compounding, so the headline example `compound-interest 1000 5 10`
= 1628.89 matches Omni's common default. Monthly compounding lives
in `compound-interest-monthly-deposit`; no silent assumption.

## 2026-10-25 — Future layout: Design 3 picked
SEAN picked **Design 3** from the 8 research mockups as CalcDex's
future layout (2026-10-25). The redesign is its own slice AFTER
v0.1.2 — v0.1.2 deliberately keeps the current layout and only
carries the readability / contrast / responsive work, which is
written generally so it carries forward into Design 3.

## 2026-10-25 — v0.2.0: Design 3 built (layout decisions)

Design acceptance criteria for this build: instant visible
response to input, fewest-clicks flows, a stable predictable
layout, WCAG AA contrast with a full keyboard path, no-panic
teaching errors, an uncluttered window, and a dedicated polish
pass (aligned keypad grid, 4/8 pt spacing rhythm, designed empty
states). The on-Windows result is Unverified until a field run.

Layout decisions taken while building Design 3:

- **Modes are the categories.** The mockup's rail lists a consumer
  calculator's six modes (Standard, Scientific, Graphing,
  Programmer, Date Calculation, Converters). CalcDex has no such
  modes — its organisation IS its 14 categories — so the rail lists
  All + the 14 categories as modes in a curated order (not
  alphabetical), with short labels (CHEM, STATS, CONVERT, BUILD,
  DATE, DAILY…) and the full name + count in the row tooltip.
- **Shortcut set:** Alt+1..Alt+9, Alt+0 for the first ten modes,
  Alt+Q/W/E/R/T for the remaining five, in rail order. Only these
  are printed (on the rail rows and in the README), and only these
  are implemented. Mode rows are click targets; the keyboard path
  to a mode is its Alt+ key (everything after that — chips, fields,
  CALCULATE, RE-OPEN — is Tab-reachable egui widgets).
- **The mockup's memory row (MC/MR/M+/M-/MS) is omitted.**
  CalcDex's memory is the persistent calculation log / history
  rail: results are saved automatically and re-open with inputs
  restored, which does the job a register memory does on the
  reference app, in the way this program already worked.
- **Operator keys × ÷ are omitted from the keypad on purpose.**
  CalcDex fields take single values, not expressions; a × or ÷
  keypress would put a character in the field that cannot parse
  and would only produce an error. Sign is the +/− key; `/` appears
  for date fields and `,` for list fields (the separator each
  field actually uses); `=` is Calculate.
- **The mockup shows a category list under the modes in the rail.**
  Ours are the same list (modes ARE categories), so there is no
  second list — one navigation system, nothing duplicated.
- **Click counts did not grow vs v0.1.2.** Pick a result:
  mode click (or Alt+key, or none — All is default) → calculator
  click → type/pad the fields → CALCULATE. Identical count to
  v0.1.2's category → calculator → CALCULATE, and the keypad adds
  a mouse-only path that did not exist before. Search is the same
  box, still filtering as you type.
- **History collapse threshold: 1100 pt window width** (content
  rect). At/above it, LOG docks/undocks the right rail; below it,
  LOG switches the centre to the log view (the v0.1.x behaviour),
  so history is reachable at every size including the 760 pt
  minimum. LOG starts shown, as in the mockup.
- **Keypad target safety:** the pad edits only the string of the
  field last clicked/focused on the currently selected calculator
  (falling back to that calculator's first field). Fields are
  per-selection state and are reset when a calculator is picked,
  so pad input can never land in another calculator's saved or
  logged state; the log only ever records Calculate runs.
- **Display headline:** the display shows the result's first line
  at 24 pt; the complete output (formula, substituted values,
  result) is duplicated verbatim into the STEPS / FULL RESULT
  footer — the design's "steps footer" — so nothing is truncated.

## 2026-10-25 — v0.3.0: Batch 1 format choices

- **Sig-figs display (#1):** rounded showing is ADDED, never
  substituted; where plain notation would make the count
  ambiguous (integer part longer than the digit count, or
  exponent < -4) the rounded value is shown in scientific form
  (1628.89 at 3 figures = `1.63e3`). Dates inside a headline are
  never rewritten.
- **Vary CLI order (#16):** `CalcDex <id> --vary <field> <start>
  <end> <step> <other numbers...>` — field numbered from 0 (the
  `--describe` numbering), then the sweep, then the remaining
  inputs in their normal order. Output is CSV (`value,result`).
- **Batch CSV (#31):** header row = the display field labels in
  order (date groups = their one merged label; cells may be
  MM/DD/YYYY). List calculators: header is the single list label
  and each row's cells are the list values. Output = input cells
  + headline result (first line), or `ERROR: <message>`. A small
  hand-written CSV parser (quotes/commas/CRLF) instead of a new
  dependency.
- **Regression inputs (#34):** the registry's pair convention —
  (x, y) pairs in one list — plus ONE optional trailing value, a
  measured y for calibration (a new `pair_tail_mode` registry
  flag; other pair calculators are unaffected).
- **Recent placement (#46):** Recent chips live in the FAVOURITES
  mode above the chip strip, not above every mode's strip — the
  centre strip stays about the current mode (uncluttered), and
  Favourites is one Alt+F away.
- **Settings file (#46/#48/#1):** one line-format text file,
  `%APPDATA%\CalcDex\settings.txt`, shared by favourites,
  recent, remembered inputs and sig-figs. The file is the shared
  source of truth for GUI and CLI (see the CLI-parity decision
  below); corrupt content falls back to defaults.
- **Defaults policy (#50):** a field's opening default is its
  teaching example unless `meta::DEFAULT_OVERRIDES` says
  otherwise — so defaults and examples can never drift apart, and
  every default is a value the audit has run. Gas-law defaults
  are STP (273.15 K, 1 atm, 22.414 L molar volume); gravity
  stays the named constant g = 9.80665 in the physics formulas
  (it is not an input anywhere).

## 2026-10-25 — Standing rule: tooltips for everything (applied)

Rule (applied from v0.3.0): every button, menu item and
interactive control must have a tooltip, no exceptions.
Applied to CalcDex from v0.3.0: all
buttons and selectable labels are constructed ONLY through the
`tip_btn` / `tip_btn_sized` / `tip_sel` / `tip_sel_value` /
`tip_chip` helpers in main.rs, each of which REQUIRES a tooltip
argument;
text edits and the combo box carry tooltips on their responses;
mode rows and history entries already had them. tests/tooltips.rs
scans src/main.rs and fails if a bare widget constructor appears
outside the helpers, so the rule is enforced by the gate, not by
memory.

## 2026-10-25 — Standing rule: full command-line support (applied)

Rule (applied from v0.3.0): every capability available in the
window must also be usable from the command line. CalcDex's
one-registry design already gave this for
calculations; v0.3.0 closes the state-feature gaps: `--favourite
<id>` / `--unfavourite <id>` / `--favourites` operate on the same
settings file the window uses, CLI runs write remembered inputs
and Recent into that file too, and `--describe` / `--example` /
`--vary` / `--batch` / `--sig-figs` mirror the window's panels.
docs/PARITY.md is maintained as the GUI-vs-CLI parity table;
there are no window-only capabilities — every capability in the
window has a CLI counterpart.

## 2026-10-25 — v0.3.0: on-screen keypad removed

The on-screen keypad (added in v0.2.0) is removed in v0.3.0: it
made the window contents uneven. The keypad UI, its focus
tracking, and the library pad code (`pad_apply` and friends, with
their tests) are deleted — no dead code. The centre column now
spans its full width uniformly and form fields widened (cap
380 → 480 pt). Keyboard flow is unchanged: typing in the fields,
Enter = Calculate, Alt+ mode keys, Ctrl+K search.

## 2026-10-25 — v0.4.0: one display-format setting (items 1/2/3/37)

The sig-figs selector (#1), the decimals setting (#2), the
engineering notation mode (#3) and Auto sig-figs inference
(#37) are ONE setting — `DisplayFormat { mode, n, rounding }` —
not four competing ones. Precedence/semantics, chosen for
cleanliness and recorded here:

- Exactly ONE annotation line is ever added to a result, in the
  active mode. There is no "sig figs wins over decimals"
  conflict because the two are modes of the same setting, never
  both active.
- The per-result VIEW row in the display area overrides the
  mode for the CURRENT result only (re-render, no setting
  change). On the CLI, `--format` / `--decimals` / `--sig-figs`
  are the same one-run override; `--as <fmt>` (copy format)
  replaces the annotated print entirely for that run.
- Rounding mode (#9) is a separate axis of the same struct: it
  says HOW the active formatter rounds, not WHICH formatter
  runs. Half up = halves away from zero (school convention);
  Banker's = half to even; Truncate = toward zero.
- Annotations stay display-only: the stored result, the log and
  batch output always keep full precision.
- Settings file: `display_format=<mode[:n]>` + `rounding=<mode>`
  lines replace `sig_figs=` (a legacy `sig_figs=` line still
  loads as the sig-figs mode, so v0.3.0 settings files keep
  working).

## 2026-10-25 — v0.4.0: typed quantities + dimensional check (#6/#7)

- One pure parser, `parse_quantity(text, family)`, in the
  library (src/rigour.rs): number+unit segments that SUM
  ("5 ft 3 in", "2 h 30 min"), over a unit table covering
  length, mass, time, volume, speed, force, area, energy,
  power, voltage, current, resistance, frequency, pressure,
  angle, acceleration and density. Temperature is affine
  (single segment only: C/F/K). Canonical unit per family =
  the factor-1 unit; `parse_quantity_to` converts into the
  unit a field actually expects.
- Wiring is metadata-driven, never guessed: a field accepts
  typed quantities only when the registry carries unit info
  for it (new optional Calc fields `unit_families` /
  `quantity_units` / `dimensions`, attached in src/meta.rs and
  in the converter constructors). Coverage at v0.4.0: every
  converter calculator, the physics set (kinematics, force,
  energy, power, pressure, Ohm's law family, projectile,
  pendulum, wave, torque, momentum, friction, Hooke, gravity,
  centripetal, free fall) and BMI/BMR/water/protein in Health.
  List calculators and all other fields keep plain-number
  entry — an unparsable entry falls back to the plain-number
  path and its teaching error.
- Dimensional check (#7): the CALCULATE path compares the
  typed quantity's detected dimension against the field's
  declared dimension and refuses with a plain-words warning
  on mismatch. Plain numbers never warn (they carry no
  dimension), and fields without metadata never warn.
- Converter value fields convert the typed quantity into the
  chosen FROM unit, so `convert-length "5 ft 3 in" 6 3` works
  (yards → kilometres) with the unit codes unchanged.

## 2026-10-25 — v0.4.0: text calculators in the registry (#5)

The Expression Calculator is a registry citizen with a
different input shape: `Calc` gains an optional `text_run`
function (`Fn(&str, bool) -> Result<String, String>`) plus
`example_text`. Numeric dispatch refuses text calculators
with a pointer to the text path; `--describe` prints the
text example; batch/vary refuse them with teaching messages.
The parser is hand-written recursive descent in the library —
no new dependency (the expression grammar is small and the
teaching error messages need token positions anyway). Runs
log through the normal history with the expression as the
restorable input, so the log is the paper tape and RE-OPEN
restores an expression.

## 2026-10-25 — v0.4.0: update check via the system curl

The offer-only update check shells out to the system `curl`
(ships with Windows 10/11; `-fsS --max-time 6`) against the
GitHub releases endpoint instead of adding an HTTP dependency
— the program's dependency set stays exactly as it was, and
offline behaviour is unchanged. Version comparison and the
release-tag extraction are pure library functions with tests;
the fetch itself is network-dependent and labelled
Unverified. The check never downloads or installs: it prints
the release page URL. Default is fully offline: the check
runs only from the HELP panel button, the CLI flag, or the
opt-in startup toggle (default OFF), all recorded in the
settings file.

## 2026-10-26 — v0.5.0: formula + glossary as data (items #19/#12)

Every calculator's formula and symbol glossary live as data in
src/meta.rs (FORMULAS / GLOSSARY statics beside EXAMPLES/HINTS),
not inside each calculator's code, so the registry-wide test can
forbid a missing row and `--describe`, the GUI section and the
deeper-steps builder all read the same source. Glossary pairs are
written in INPUT ORDER: the deeper-steps substitution pairs
glossary entry i with input i (longest symbol first, whole-token
matching). Where that pairing cannot apply (date, list and text
calculators, whose "inputs" are not the symbols), the substituted
stage falls back to naming each field and its value — it never
invents a substitution. Deeper stage 3 is the calculator's own
output byte-for-byte: rearrangement working a calculator already
prints sits there unchanged, and no stage ever fabricates
working that the calculator did not produce.

## 2026-10-26 — v0.5.0: check-my-answer honesty (#13)

The check compares against the FIRST result value a calculator
prints (first assignment in the headline; documented in the
panel text and here), because multi-result calculators (circle:
area, circumference, diameter) have no single "the answer".
Fraction-family answers are compared as exact rationals;
everything else uses a stated 0.5% relative tolerance. The slip
detectors are generic pure functions (x100, /100, x4, /4, x2,
/2, square, square-root, sign flip) matched within the same
0.5% band — a student's rounded wrong answer (314.16 for a true
314.159...) still matches its slip — and every slip sentence is
phrased "a common cause is...", naming a possibility, never a
diagnosis. A merely-close wrong answer names no slip.

## 2026-10-26 — v0.5.0: exact mode scope (#14)

Exact is a display-format mode, joining the v0.4.0 setting, with
a deliberately narrow, documented scope: (a) fraction-family
results are labelled exact; (b) the pi geometry family shows an
exact form computed from exact rational coefficients of the
printed inputs (the ellipse perimeter is labelled an
approximation, as it is); (c) the Expression Calculator gains a
separate AST evaluator over an i128 rational type for +, −, ×, ÷
and whole-number powers of integer/fraction values. Everything
else — functions, pi/e/tau constants, x, non-whole powers,
overflow — falls back to the decimal result with an annotation
saying exactly that. The decimal engine (rigour's evaluator) is
untouched; the AST evaluator mirrors its grammar, and a test
keeps their decimal results in agreement on the exact cases.

## 2026-10-26 — v0.5.0: practice generation (#15)

Problems are the calculator's example values varied (0.5x–2x;
0.8x–1.25x for values of 50 or more, so heights/prices/weights
stay plausible), retried until the varied inputs run cleanly.
A pure xorshift64* generator in the library (no dependency)
drives both surfaces; the CLI form is stateless by design —
`--practice --seed N --answer X` re-makes the same problem from
the seed to check it. Excluded, and stated in the view and the
CLI error: date-group, list/pair-input and text calculators (by
shape) plus the unit-code, lookup-table and fill-one-of-two
families (explicit list in students.rs) — random values would
not be sensible questions for them. The session tally is memory
only and labelled so.

## 2026-10-26 — v0.5.0: one sampler for both plot surfaces (#17)

The GUI plot and the CLI `--plot` CSV use the same library
sampler, so a gap (undefined/non-finite point) is a gap in both.
The GUI draws with the egui painter (no plot dependency): axes,
gridlines at 1/2/5×10^k round values, the curve as segments
between consecutive defined samples, breaking also when a
segment jumps more than 90% of the plot height (tan/log poles
must not streak). The plot exists for the Expression Calculator
and the quadratic solver — the registry's only single-variable
function calculators.

## 2026-10-26 — v0.6.0: the problem-set file format (items #22/#23/#27/#30)

One plain-text format serves the builder, seeded generation,
worksheets, keys, marking and assignment export/import:

```
# CalcDex problem set
# Name: <set name>
# Seed: <n>            (only on generated sets)
calc-id | input | input ...
```

Inputs are the calculator's display values, `|`-separated — the
same tokens the form shows (dates as one merged MM/DD/YYYY
token, unit codes as their numbers). Differences from the batch
CSV conventions: the set is line-oriented text, not a table (a
set mixes calculators with different field counts, so a shared
header row cannot work), and every entry is validated on load by
actually running it — a set file can never name a problem
CalcDex cannot pose. The file contains problems ONLY: no
answers, no results. That is what makes the same file the
student assignment (#30) — a test asserts no result text appears
in it. Worksheets (problems + working space) and answer keys
(answers + one-line substituted-formula working) are separate
renderings of the same in-memory set, never stored back into it.

## 2026-10-26 — v0.6.0: demonstration mode stores the REAL result in the log (#25)

DEMO masking is display-only: the centre display, the steps
footer and solution exports hide result values, but the
calculation log stores the run exactly as it happened. Reason:
the log is the teacher's own record, and RE-OPEN exists so the
teacher can pull a past run back up — a masked log entry would
make that record useless and would silently falsify it. The
risk accepted: a student looking over the teacher's shoulder at
the LOG view could see past results — the same exposure as the
teacher's own mark book, and the DEMO state line in the header
says plainly that the log stores real results. The masking
itself replaces every number in the result text that is not one
of the run's own input values with a bullet run, so substituted
working ("A = pi x 5^2") stays readable while answers
("= 78.539816") do not leak.

## 2026-10-26 — v0.6.0: course-pack tagging rule (#24)

A calculator is tagged for a course level only where it
genuinely belongs in that curriculum: GCSE = the standard
14–16 secondary topics (number, basic algebra, shapes,
right-triangle trig, percentages, simple/compound interest
basics, core statistics, basic physics and chemistry
quantities); A-Level adds logs, series, hypothesis testing,
normal/binomial/Poisson work, mechanics and the full science
categories; AP mirrors the US AP course families (Statistics,
Calculus-adjacent, Physics 1/2 + C, Chemistry); Undergrad takes
the whole Math/Statistics/Physics/Chemistry categories plus the
engineering-flavoured Everyday/Sports sets. When a fit was
unsure, the calculator was left untagged — a pack that is
missing a borderline tool is a smaller fault than a pack padded
with tools that do not belong. Counts at v0.6.0: GCSE 115,
A-Level 161, AP 121, Undergrad 210.

## 2026-10-26 — v0.6.0: examples library format (#28)

%APPDATA%\CalcDex\examples.txt mirrors the settings-file
discipline: a `# CalcDex saved examples` header, then one line
per example — `title | calc id | note | input;;input...` — with
`\`, `|`, `;` and newlines backslash-escaped inside fields.
Titles are unique (saving an existing title replaces it), and
parsing drops lines whose calculator id no longer exists rather
than failing the whole library.

## 2026-10-26 — v0.6.0: projection mode approach (#21)

Projection is a view concern, not a data change: the window
sets egui's zoom factor to 1.5 and re-runs the style pass with
the hint/muted colours brightened (#d8d8d8 / #b8b8b8) and the
panel backgrounds darkened, so every existing panel pairs at
higher contrast without a second palette. The toggle persists
as `projection=on|off` in the shared settings file, which is
also what the CLI writes — the CLI has no screen to zoom, so
parity for this item is the shared setting itself.

## 2026-10-26 — v0.7.0: paste-a-column shares one parser (#32)

List values are parsed by exactly one function
(`calcdex::parse_value_list`): split on runs of comma,
semicolon, tab, newline or space, parse each token, and name
the offending token in the error. The CLI's list expansion and
the window's list field both call it, so a pasted spreadsheet
column behaves identically in both shells. Before v0.7.0 the
two shells split slightly differently (the window did not
accept semicolons as its own rule, only by accident of
whitespace splitting); now there is one rule to document and
one place to fix.

## 2026-10-26 — v0.7.0: the 95% interval uses Student's t (#33)

Replicates Summary embeds the standard two-sided 95% t
critical values for df 1–30 (three decimals, the values printed
in textbook tables). Beyond df 30 it switches to the normal
value 1.96 rather than extrapolating the table, and the
calculator's description says so. The interval is therefore
exact-table for the small samples the calculator is for, and
the switch point is visible to the user instead of silent.

## 2026-10-26 — v0.7.0: quartiles by the Tukey method (#36)

Outlier Check computes Q1 and Q3 as the medians of the lower
and upper halves of the sorted data (Tukey hinges; the overall
median is excluded from both halves for odd counts). Percentile
interpolation methods give slightly different quartiles on
small samples, so the method is named in the calculator's
description and stated here: results are reproducible by hand
from the printed sorted list.

## 2026-10-26 — v0.7.0: serial dilution takes a mode code (#35)

The registry's numeric-only inputs cannot express "either
volumes or a factor" as a switch, so Serial Dilution follows
the established mode-code pattern used by the other
multi-method calculators (a Mode input: 1 = transfer +
diluent volumes, factor = (transfer + diluent) / transfer;
2 = a plain per-step factor). The output's first sentence names
the method and factor actually used, so a printed result is
self-describing. Units are not parsed: concentrations carry
the starting concentration's unit and volumes one shared unit,
stated in the output.

## 2026-10-26 — v0.7.0: constants — `e_charge`, settings storage (#38)

A bare `e` in an expression has meant Euler's number since
v0.4.0, and silently redefining it would change existing
expressions, so the elementary charge is `e_charge` (the
listing annotates it). CODATA constants live in a compiled-in
table (`rigour::CONSTANTS`) with name, symbol, value, unit and
aliases; the parser resolves built-ins first, then user
constants. User constants are `constant.<name>=<value>` lines
in the shared settings file. Because the expression parser is
in the library and both shells need the same table, the shells
install the settings' constants into a process-wide table at
startup (and re-install after any define/delete); validation
refuses any name colliding with a built-in, a function, or
pi/e/tau/x at define time, and loading skips stored lines that
fail validation. Extreme-scale values in listings and
expression results print via a scientific-notation fallback
(`n_full`), so a photon energy reads 3.972891714298e-19 rather
than rounding to 0 at ten decimal places.

## 2026-10-26 — v0.7.0: uncertainty grammar + worst-case rules (#40)

Grammar: `±` (U+00B1, or the spelling `+/-`) binds to the atom
directly before it — a number, a constant, or a bracketed
expression — tighter than any operator, at most once per atom:
`2 * 10 ± 0.2` is 2 × (10 ± 0.2), never (2 × 10) ± 0.2. The
uncertainty itself is an unsigned number literal. Propagation
is the first-order maximum-error convention taught in
undergraduate labs: absolute uncertainties add for + and −;
relative uncertainties add for × and ÷; a power n scales the
base's relative uncertainty by |n|; when both parts of a power
are uncertain, the base's scaled relative term and the
exponent's |ln(base)|-scaled relative term combine in
quadrature (the standard combined formula for that case);
functions use the slope rule u_out = |f′(x)| · u_in. Steps name
the rule behind every operation. Note on the worked example in
the enhancement list: it quotes (10 ± 0.2) * (5 ± 0.1) =
50 ± 2.236…, which follows from neither the addition of
absolute uncertainties (50 ± 2.86…), the stated relative-addition
rule (50 ± 2 exactly: 2% + 2% = 4%), nor root-sum-of-squares
(50 ± 1.414…). The stated rules were implemented; the
addition example (15 ± 0.3) matches the brief exactly, and the
multiplication answer under the stated rules is 50 ± 2,
covered by known-answer tests.

## 2026-10-26 — v0.7.0: lab notebook storage + edit backup (#39)

History lines gain two trailing fields (tag, then note), for
eight pipe-separated escaped fields total. The reader accepts
both six-field lines (everything written by v0.1.1–v0.6.0) and
eight-field lines, so existing history files keep working with
no migration; the next append simply writes the new shape.
Tag/note edits address entries by their --log number (#1 =
newest) and, before writing, save the current log to
`history-edit-backup.txt` — deliberately a different file from
the clear-log backup (`history-backup.txt`), so undoing a clear
and undoing an edit can never overwrite each other's safety
copy. The lab record export is chronological (oldest first),
the reverse of the log view, because a lab record is read
forwards in time; its header says so.

## 2026-10-26 — v0.8.0: category placement for the new packs (#41/#45)

The electronics pack (resistor colour code, series/parallel
resistors, voltage divider) sits in Physics: Ohm's law and the
rest of the electricity set already live there, and a user
looking for a divider looks next to them. The mechanical pack
(tolerance stack-up, safety factor, bolt torque, beam bending)
sits in Construction, the registry's home for structural and
load-bearing work. No new categories: the mode rail is a
settled 14, and both packs have a natural home.

## 2026-10-26 — v0.8.0: resistor colour code is a text calculator (#41)

Band colours are words, not numbers, so the colour code is a
text-input calculator like the Expression Calculator: one field,
both directions. The direction is chosen by the first token — a
number (4700, 4.7k, 1M) means "give me this value's bands";
colour words mean "read these bands". Encoding prefers the
4-band form (exactly two significant digits times a power of
ten), falls back to 5-band (three digits), and otherwise
refuses with a teaching message naming the nearest 4-band
value — a silently rounded band readout would be worse than no
answer. When no tolerance colour is given for an encode, gold
(±5%, the commonest) is used and the result says so.

Series & parallel resistors are ONE calculator that reports
both totals for the same list, rather than two registry
entries: the two figures are the pair every user wants, and
one entry keeps the list shorter. A 0 Ω entry in parallel is
reported as the short circuit it is, not an error.

## 2026-10-26 — v0.8.0: safety factor modes and wording (#43)

The stress source is a mode code (1 = stress given, 2 = load
and area), the same pattern the Serial Dilution planner set
for mode-switched calculators. Whether the strength is a yield
or an ultimate value is the user's fact, not the program's:
the input is labelled "yield or ultimate" and results speak of
"the strength you entered". An FoS below 1 is stated as a
predicted failure in plain words — softening that sentence
would be a disservice. The reading bands (below 1 / below 1.5
/ 1.5 and above) report context; the calculator explicitly
does not approve designs.

## 2026-10-26 — v0.8.0: materials values are typical, insulators stay blank (#44)

The materials table holds typical published textbook values
and is labelled "typical values" on every surface (calculator
descriptions, `--materials`, the MATERIALS view, this file's
companion docs) because real alloys and batches vary.
Resistivity is given for metals only: glass, wood and the
plastics are insulators whose resistivities are
moisture-and-grade dependent across many orders of magnitude,
so the table says "not applicable (electrical insulator)"
instead of printing a plausible-looking invented figure.

The table is data, not a calculator (the constants library is
the precedent): it is served by `CalcDex --materials`, the
MATERIALS view, and the beam calculator. The beam takes a
material by numeric code — its table position — because the
registry's inputs are numbers; the window adds a USE E IN BEAM
button that fills the row's modulus into the E field as a
plain number, so both routes produce exactly the same run.
E = 0 is the "use the code" sentinel, following the mode-code
pattern. Wiring material names into the older density-input
calculators was considered and not done: their inputs are
densities in the user's own units, and `--materials` plus the
view is the lookup that feeds them; the beam is the one
calculator whose modulus input the table owns outright.

Beam output prints bending stress in MPa (the unit engineers
read; the Pa figure at the registry's 10-decimal formatting
showed raw floating-point tails) and the section inertia in
scientific notation when it is below 1e-3 m^4.

## 2026-10-26 — v0.8.0: pin storage and the cap rule (#49)

A pinned history entry's line carries one extra trailing
field, `|1`; unpinned entries serialize byte-identically to
the v0.7.0 eight-field shape, and the reader accepts six,
eight and nine fields, so a v0.1.1 or v0.7.0 history file reads
unchanged and only pinned lines use the new shape. The
accepted trade-off: an older version reading a file that
contains a pinned line will skip that one line (its field
count check fails), in the same way it would skip any line it
cannot parse. Pin edits take the same backup-first path as
tag/note edits (history-edit-backup.txt).

The 500-entry cap now counts UNPINNED entries: appends trim
the oldest unpinned entries until at most 500 unpinned remain,
and pinned entries are never trimmed, so the file can hold
500 + (number of pins) lines. CLEAR LOG still clears
everything, pins included — it is an explicit action with its
own backup, and RESTORE CLEARED LOG undoes it wholesale.

Filtered log views (window and CLI) display each entry with
its GLOBAL number — its number in the whole log — never a
number within the filtered subset, so --log-pin, --log-tag
and --log-note addressing is the same from any view. The
filtered export is chronological, like the lab record,
because an exported record is read forwards in time.

## 2026-11-07 — RPN / programmable mode (Free42 + Plus42 functionality)
SEAN asked for the functionality of Free42 and Plus42 and said
"proceed as you suggest". Chosen scope: (b) an HP-42S-compatible
RPN/programmable mode inside CalcDex first, then the Plus42
extras (equations, units, TVM, directories, plotting). Clean-room
implementation (Free42/Plus42 are GPLv2; CalcDex stays MIT): no
upstream source read or copied. Engine lives in a new `src/rpn/`
beside the registry, because the registry signature is stateless,
f64-only and scalar. Numbers: f64 first, 34-digit decimal in P2.
Own versioned state format; program listings import/export. No UI
code until the wireframe is approved. Plan: docs/RPN_PLAN.md.
