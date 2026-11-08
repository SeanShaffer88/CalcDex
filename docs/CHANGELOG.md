# Changelog — CalcDex

## Unreleased

### Added
- **RPN mode, phase P1 (command line).** `CalcDex rpn "2 3 + 4 *"`
  runs HP-42S-style tokens on a 4-level stack with LASTX, stack
  lift/ENTER, arithmetic, powers, roots, logs, trig (DEG/RAD/GRAD,
  exact at quarter turns), hyperbolics, FACT/COMB/PERM, %, %CH,
  MOD, named variables (STO/RCL and STO+ - * /, RCL+ - * /, CLV)
  and FIX/SCI/ENG/ALL display. `--stack` shows T/Z/Y/X, `--json`
  prints one JSON object, `--state NAME` keeps the stack and
  variables between runs in `data/rpn-NAME.txt` (versioned,
  atomic; a corrupt file is kept as `.bad`, a newer-version file
  is never overwritten). `--clean`/`--uninstall` remove these
  files. Numbers are f64 for now; 34-digit decimal is phase P2.
  No window yet (phase P9). Plan: [RPN_PLAN.md](RPN_PLAN.md).

### Fixed
- Legacy storage migration stages and flushes the complete data set before
  publication, preserving legacy storage on failure and existing foreign entries.
- **Projection mode keeps the History rail.** Turning on classroom
  projection enlarges the whole window (1.5x), which used to make
  the History rail drop away and the top row run off the right edge
  on ordinary screens. The rail's fit is now judged on the physical
  window width, both side rails start narrower while projection is
  on so all three panes stay on screen together, and the top row
  wraps instead of clipping.

## 0.9.0 — 2026-11-02

**Engineering-contract conformance release.** No calculator
behaviour changes; the work is where the program keeps its
files, how it is built, and how it is released.

- **Your files moved next to the program.** Settings, the
  calculation log, saved class examples and their backups now
  live in a `data` folder beside `CalcDex.exe` instead of
  `%APPDATA%\CalcDex`. On first run the old files are copied
  across (the originals are left untouched); if the `data`
  folder cannot be written, the old location is used instead,
  so nothing is ever lost to a failed move. See
  [MIGRATION.md](MIGRATION.md). `CALCDEX_DATA_DIR` can point
  the data folder elsewhere.
- **Settings carry a format version.** `settings.txt` now
  starts with `schema_version=1`. Older files load unchanged
  and are upgraded on the next save; a file written by a newer
  CalcDex loads as defaults with a warning instead of being
  overwritten.
- **Saves are crash-safe.** Settings, log and examples are
  written to a temporary file, flushed, and renamed over the
  old file — a crash or power cut mid-save can no longer leave
  a half-written file.
- **The window remembers its place.** The last window
  position and size are stored in the settings and restored
  on the next launch.
- **New maintenance commands.** `CalcDex --version` prints
  the version. `CalcDex --clean` removes CalcDex's own files
  from the data folder (with `--dry-run` to preview and
  `--silent` to skip the confirmation); `--uninstall` does
  the same and explains how the program files are removed.
  `--json` prints a run's result, or a clean/uninstall
  report, as one JSON object. Both refuse while a CalcDex
  window is open.
- **One window at a time.** A second launch now prints
  "already running" and exits instead of opening a second
  window over the same files.
- **Bug fix.** The median calculator could crash on an
  empty list; it now reports that the list is empty.
- **Releases are x64 only, in one shape.** Each release now
  ships `CalcDex-v{version}-x64-Setup.exe`,
  `CalcDex-v{version}-x64-Portable.zip` and
  `SHA256SUMS.txt`. The x86 and ARM64 installers and the MSI
  are gone. Builds are made by the new CI/release workflows,
  which run the full check (formatting, lints, all tests)
  before packaging.
- **Static runtime.** The Windows build now links the C
  runtime statically, so no separate runtime install is
  needed.
- **Documentation.** The changelog moved to `docs/` and the
  standard document set was adopted: FEATURES,
  COMPATIBILITY, ARCHITECTURE, INSTALL, MIGRATION,
  TROUBLESHOOTING, SUPPORT, SECURITY, CONTRIBUTING,
  CODE_OF_CONDUCT, MAINTAINERS, GOVERNANCE and CREDITS now
  live beside it.

## 0.8.0 — 2026-10-26

**Engineers & professional polish batch of the 50-enhancement
program — items 41, 42, 43, 44, 45, 49: DONE.** With this batch
the planned enhancement set is complete (all 50 items shipped
across v0.3.0–v0.8.0). Registry 309 → **316 calculators**.
Everything below works in the window AND on the command line
(docs/PARITY.md updated; zero window-only capabilities).

- **#41 Electronics pack (new Physics calculators).**
  Resistor Colour Code: type the band colours in order (4 or 5
  words, e.g. `red red brown gold`) to decode a resistor to its
  value and tolerance range, or type a resistance (`4700`,
  `4.7k`, `1M`, optionally with a tolerance colour) to get its
  bands — 4-band form when the value is exact in two significant
  digits, 5-band for three, and a teaching refusal with the
  nearest band value when no exact bands exist.
  Resistors in Series & Parallel: one list of resistances gives
  both totals, with a 0 Ω entry reported as the short circuit it
  is in parallel. Voltage Divider: Vin, R1, R2 → Vout, the shared
  current, and each resistor's power.
- **#42 Tolerance Stack-Up (new Construction calculator).**
  Dimensions entered as (nominal, tolerance) pairs give the
  total, the worst-case range (every tolerance at its extreme)
  and the RSS (root-sum-square) statistical range.
- **#43 Safety Factor & Margin of Safety (new Construction
  calculator).** Strength (yield or ultimate, labelled) against
  an applied stress entered directly (mode 1) or as a load over
  an area (mode 2) → FoS and MoS, with a plain-language reading;
  an FoS below 1 is stated as a predicted failure, not softened.
- **#44 Materials table.** A built-in table of 12 common
  engineering materials (aluminium 6061 and pure, copper, brass,
  mild steel, stainless 304, titanium Ti-6Al-4V, cast iron,
  glass, oak, ABS, nylon 6/6) with typical published density,
  electrical resistivity and Young's modulus values — labelled
  "typical values" wherever shown, and with resistivity marked
  not-applicable for insulators instead of inventing a figure.
  Served by `CalcDex --materials [name|code]`, the header's
  MATERIALS view, and the beam calculator (below), which takes
  a material code when its E field is 0; the MATERIALS view's
  USE E IN BEAM button fills a row's modulus into the beam
  calculator directly.
- **#45 Mechanical pack (new Construction calculators).**
  Bolt Torque: T = K × D × F with the nut factor's standard
  guidance values (about 0.20 dry steel, 0.15 lubricated) in
  the field hint; torque in N·m and ft·lbf.
  Beam — Stress & Deflection: a simply supported beam with a
  central point load; rectangle or circle section; moment of
  inertia, maximum bending stress and midspan deflection, with
  E entered directly or taken from the materials table.
- **#49 Log polish.** The calculation log gains search
  (`--log-search`, and the log view's search box), filters by
  calculator, tag and date (`--log-filter calc=… tag=… date=…`,
  mirrored in the log view), pinning (`--log-pin <n>` / the
  PIN button: a pinned entry is never dropped by the 500-entry
  trim — the cap counts unpinned entries, and pinned entries
  are marked in `--log` and the window), and export of the
  currently filtered set to a plain-text file (`--log-export
  <file>` with `--search/--calc/--tag/--date/--pinned` flags,
  and the log view's EXPORT FILTERED button, which backs an
  existing target up to a `.bak` file first). Filtered displays
  keep each entry's global `--log` number, so pin/tag/note
  numbering stays unambiguous. History files keep their format
  discipline: a pinned entry's line carries one extra field;
  unpinned lines are byte-identical to the v0.7.0 shape, and the
  reader still accepts the v0.1.1 six-field and v0.7.0
  eight-field lines.

## 0.7.0 — 2026-10-26

**Scientists & researchers batch of the 50-enhancement program —
items 32, 33, 35, 36, 38, 39, 40: DONE** (items 31, 34 and 37
shipped in earlier batches). Registry 306 → **309 calculators**.
Everything below works in the window AND on the command line
(docs/PARITY.md updated; zero window-only capabilities).

- **#32 Paste-a-column data entry.** One shared value-list
  parser in the library now feeds every list calculator in both
  shells: values may be separated by commas, semicolons, tabs,
  spaces or new lines, so a copied spreadsheet column pastes
  straight into the window field or a command-line argument.
  A token that is not a number produces a teaching error naming
  the token.
- **#33 Replicates Summary (new Statistics calculator).** A
  list of repeated measurements → count, mean, sample standard
  deviation, standard error of the mean, and the 95% confidence
  interval, with the working shown. The interval uses Student's
  t from an embedded critical-value table (df 1–30); beyond 30
  degrees of freedom it uses the normal value 1.96, stated in
  the calculator's description.
- **#36 Outlier Check (new Statistics calculator).** Quartiles
  by the Tukey median-of-halves method, interquartile range,
  and the 1.5×IQR fences; values outside the fences are listed
  explicitly, and a clean dataset is reported as having no
  outliers in plain words.
- **#35 Serial Dilution (new Chemistry calculator).** Starting
  concentration plus either transfer and diluent volumes
  (mode 1) or a plain per-step factor (mode 2), and a step
  count, → the concentration after every step. Units are
  carried as the user's own text: concentrations are in the
  starting concentration's unit, volumes in one shared unit.
- **#38 Constants library.** Fifteen CODATA 2018 constants —
  c, h, ħ, e (elementary charge, as `e_charge`), k_B, N_A, R,
  G, ε0, μ0, m_e, m_p, u, F, σ — each with name, symbol, value
  and unit, usable by name or alias in the Expression
  Calculator (a bare `e` remains Euler's number). `CalcDex
  --constants` lists them; the window's CONSTANTS view lists
  them with INSERT into the expression field. User-defined
  constants (`CalcDex --define name=value`, or the CONSTANTS
  view) are stored as `constant.<name>=<value>` lines in the
  shared settings file; a name that collides with a built-in
  constant, a function, or pi/e/tau/x is refused with a
  teaching error, and a colliding line found in the settings
  file is skipped when it loads.
- **#40 Uncertainty propagation (Expression Calculator).** A
  value written `10 ± 0.2` (or `10 +/- 0.2`) carries its
  uncertainty through + − × ÷ ^, brackets and functions by the
  first-order (maximum-error) lab rules: absolute uncertainties
  add for + and −; relative uncertainties add for × and ÷; a
  power n scales the relative uncertainty by |n| (when both
  parts of a power are uncertain, their relative contributions
  combine in quadrature); functions use the slope rule
  u = |f′(x)| · u_in, with degrees converted where the angle
  mode needs it. The result prints as `value ± uncertainty`
  with the uncertainty's percentage, and each step names the
  rule used. Results at extreme scales print in scientific
  notation instead of 0 — the Expression Calculator's result
  line now uses that fuller format throughout.
- **#39 Lab notebook.** Log entries carry a tag and a note:
  the LOG view shows them and edits them per entry (TAG /
  NOTE), and the command line has `--log-tag <n> <tag>` and
  `--log-note <n> <note>` using the `--log` numbering (#1 =
  newest). `--lab-export <file>` writes the whole log as a
  plain-text lab record (oldest first, tags and notes included,
  timestamps stated as UTC). History lines now store eight
  fields; the reader still accepts the six-field lines written
  by v0.1.1–v0.6.0, so existing history files read unchanged.
  Any tag/note edit writes the pre-edit log to
  `history-edit-backup.txt` first (a separate file from the
  clear-log backup, so the two undos never overwrite each
  other).

## 0.6.0 — 2026-10-26

**Teachers & professors batch of the 50-enhancement program —
items 21, 22, 23, 24, 25, 26, 27, 28, 29, 30: DONE.** Registry
unchanged: 306 calculators. Everything below works in the window
AND on the command line (docs/PARITY.md updated; zero
window-only capabilities).

The shared foundation is the **problem set**: an ordered list of
problems (a calculator + the values to use), stored as a plain
text file — a `# CalcDex problem set` header, a `# Name:` line,
an optional `# Seed:` line, then one problem per line as
`calc-id | input | input ...` (date values as one MM/DD/YYYY
token). A set file holds problems only, never answers, so the
same file is the student assignment (#30).

- **#21 Classroom projection mode.** A PROJECTION toggle in the
  header renders the whole window at about 1.5x with brighter
  secondary text and darker panels, and says so on screen. The
  choice is saved in the shared settings file
  (`CalcDex --projection on|off`).
- **#22 Problem-set builder.** A SETS view: name a set, add the
  currently open calculator with its current values, remove
  entries, load an entry back into its calculator form, save the
  set to a file, and export a teacher answer key (answers plus a
  one-line substituted-formula working, headed ANSWER KEY —
  TEACHER COPY). CLI: `CalcDex set new <file> [name...]`,
  `CalcDex set add <file> <id> <inputs...>`,
  `CalcDex set key <file> <key-file>`,
  `CalcDex set open <file>` (lists the problems, never answers).
- **#23 Seeded sets.** `CalcDex set generate <file> --seed N
  --count K <id> [more ids...]` (and the same controls in the
  SETS view) generates problems with the seeded practice
  generator, round-robin over the given calculators. The same
  seed always produces the identical set file, byte for byte —
  a whole class can work the same problems.
- **#24 Course packs.** Calculators are tagged by real
  curriculum fit: GCSE (115), A-Level (161), AP (121),
  Undergrad (210); a calculator may carry several tags or none.
  COURSE PACKS in the rail filters the calculator strip to one
  pack; `CalcDex --pack GCSE --list` prints a pack, and
  `--describe` names a calculator's packs.
- **#25 Hide-answer demonstration mode.** A DEMO toggle: a run
  shows inputs and working structure with result values replaced
  by bullets until REVEAL is pressed for that result (HIDE AGAIN
  re-masks). The display, the steps footer and solution exports
  are masked; the calculation log stores the real results (it is
  the teacher's record — RE-OPEN is a teacher action). CLI:
  `--demo` masks a run's printed values, `--demo --reveal`
  prints them.
- **#26 Bulk marking.** `CalcDex --mark <set-file>
  <answers.csv>` marks a class CSV (a header row, then one row
  per student: name, then answers in problem order) with the
  #13 checker — fraction answers compared exactly, others
  within 0.5%. It writes a marked CSV (right/wrong per problem
  plus a score column) beside the answers file and prints one
  summary line per student. The SETS view has the same marking
  with path fields and a summary display.
- **#27 Printable worksheets.** EXPORT WORKSHEET in the SETS
  view (or `CalcDex set worksheet <file> <out>`) writes a
  plain-text worksheet: name/date/class header lines, the
  numbered problems, ruled working space and an answer line per
  problem — no answers anywhere in the file. Print it from any
  text editor; CalcDex has no print dialog.
- **#28 Saved class examples library.** Named worked examples
  (calculator + values + title/note) are stored in
  %APPDATA%\CalcDex\examples.txt (documented plain-text format).
  The SETS view saves the current calculator as an example and
  lists, loads and deletes saved ones (delete asks for one
  confirmation). CLI: `--save-example "<title>"` with a run,
  `--examples`, `--open-example "<title>"`.
- **#29 Formula-sheet generator.** `CalcDex --formula-sheet
  <Category|All> <file>` (and the SETS view) writes a printable
  text sheet: every calculator in the category with its formula
  and symbol meanings, from the #19 metadata.
- **Settings:** one new key, `projection=on|off`. Older settings
  files load unchanged (the key defaults off). Set, worksheet,
  key, marked-CSV and formula-sheet exports never overwrite an
  existing file unless --force is given (CLI) — and then the old
  file is kept as a .bak copy first, same as the window.

## 0.5.0 — 2026-10-26

**Students batch of the 50-enhancement program — items 12, 13,
14, 15, 17, 18, 19: DONE.** Registry unchanged: 306 calculators.
Everything below works in the window AND on the command line
(docs/PARITY.md updated; zero window-only capabilities).

- **#19 Formula & symbols for every calculator.** Every one of
  the 306 calculators now carries a formula in plain text symbols
  (e.g. `A = pi x r^2; C = 2 x pi x r; d = 2r`) and a symbol
  glossary (symbol → plain-words meaning). A FORMULA & SYMBOLS
  section sits under each calculator's form in the window;
  `CalcDex --describe <id>` prints the formula and glossary too.
  A registry-wide test fails if any calculator ever lacks either.
- **#12 Deeper steps.** A STEPS selector in the header (and the
  `steps=` settings key): Standard shows the usual full result;
  Deeper shows four labelled stages — (1) the formula, (2) the
  formula with your values substituted in, (3) the calculator's
  normal working, verbatim, (4) the result line. CLI:
  `--steps deeper`.
- **#13 Check my answer.** A CHECK MY ANSWER panel under the
  form: type your own answer (a number, or a fraction like 3/4),
  and CalcDex runs the calculation and answers in plain words —
  correct, or not yet, showing your value, the correct value, and
  the advice to compare your working with the steps. Fraction
  calculators are checked exactly (2/4 counts as 1/2); everything
  else allows 0.5% for rounding in your working, and the verdict
  says which was used. The check compares against the FIRST
  result value the calculator prints. When a wrong answer
  matches a common slip — 100x or /100 (percent handling), 4x or
  /4 (diameter vs radius), 2x or /2, the square or the square
  root of the correct value, or a flipped sign — the verdict
  names the likely slip, phrased as "a common cause is...".
  CLI: `CalcDex <id> <args> --check <answer>`.
- **#14 Exact mode.** A sixth display format, Exact, in the
  FORMAT selector, the per-result VIEW row, and `--format
  exact`. Scope: (a) the fraction calculators' results are
  labelled exact; (b) the pi geometry family (circle,
  circumference, circle-from-area, cylinder, sphere, cone,
  sector, ellipse, frustum, square-footage-circle) also shows
  its exact form, e.g. `[Exact form: Area = 25*pi; Circumference
  = 10*pi]` (the ellipse perimeter line is labelled an
  approximation, as it is); (c) the Expression Calculator
  evaluates +, −, ×, ÷ and whole-number powers exactly with a
  rational-number engine when every value is an integer or a
  fraction (1/3 + 1/6 = 1/2 exactly). Anything outside that
  scope keeps the decimal result and the annotation says so.
- **#15 Practice mode.** A PRACTICE view (header toggle): pick
  any calculator in the strip or press RANDOM PROBLEM; CalcDex
  generates a problem by varying the calculator's example values
  (large values stay within 0.8x–1.25x of the example, others
  0.5x–2x), shows the question with the answer hidden, checks
  your typed answer with the same checker as #13, and REVEAL
  shows the worked solution. A SEED field restarts the problem
  sequence — the same seed always makes the same problems (a
  small seeded generator in the library, no new dependency).
  The session tally (attempted / correct) is kept in memory
  only and is labelled as such. Calculators whose inputs are
  unit codes, lookup tables, fill-one-of-two pairs, dates or
  lists are excluded, and the exclusion is stated in the view.
  CLI: `CalcDex <id> --practice --seed N` prints the problem;
  adding `--answer X` checks it (the problem is re-made from
  the seed for the check).
- **#17 Graph plot.** The Expression Calculator gains a PLOT
  section: the expression in x, x-from / x-to fields, and a
  hand-drawn curve (axes, gridlines at round values, sampled at
  pixel resolution; undefined points and steep jumps break the
  line instead of streaking). The quadratic solver plots its
  a*x^2 + b*x + c from the form the same way. CLI parity:
  `CalcDex expr "x^2" --plot -5 5` prints the plot as CSV x,y
  sample points (a blank y means the function is not defined
  there); trig respects --degrees/--radians.
- **#18 Export worked solution.** An EXPORT SOLUTION button
  under the steps writes a plain-text file containing the
  calculator name, formula, symbols, inputs, full steps and
  result, to a path in an editable field prefilled with
  %USERPROFILE%\Documents\CalcDex-<id>-solution.txt; success or
  failure is shown in words. CLI: `CalcDex <id> <args> --export
  <path>` writes the same text.
- Built-in help text covers the new tools. Settings file gains
  only the `steps=` key; v0.4.0 settings files load unchanged.

## 0.4.0 — 2026-10-25

**Rigour batch of the 50-enhancement program — items 2, 3, 4, 5,
6, 7, 8, 9, 10, 37: DONE.** Plus four standing-rule features:
built-in help, undo for destructive actions, drag-and-drop batch,
and an offer-only update check. Registry: 300 → 306 calculators.
Everything below works in the window AND on the command line
(docs/PARITY.md updated; zero window-only capabilities).

- **One display-format setting (#1+#2+#3+#37 unified).** The
  header FORMAT selector replaces the old SIG FIGS selector:
  Off / Sig figs N / Decimals N / Engineering / Auto (from
  inputs). Exactly one annotation line is ever added, and it is
  display only — the stored result and the log keep full
  precision. Sig figs counts significant figures; Decimals fixes
  decimal places (0–10); Engineering shows the headline in
  engineering notation with SI prefixes (1.63 k, 4.2 M, 350 m,
  down to pico); Auto counts the significant figures of the
  numbers as typed (standard convention: "100" = 1, "100." = 3,
  "1.00" = 3, "0.004560" = 4), takes the minimum, and says so on
  the annotation line. A small VIEW row under the display
  re-renders the current result in another format without
  changing the saved setting. CLI: `--format <mode[:n]>`,
  `--decimals N`, `--sig-figs N` (kept), settings file gains
  `display_format=` (the old `sig_figs=` line still loads).
- **#9 Rounding modes.** Half up (halves away from zero) /
  Banker's (half to even) / Truncate, applied inside all display
  formatters. Chosen in the display area; CLI `--rounding`.
  Tested at 2.5 / 3.5 / −2.5 in all three modes.
- **#4 Copy result.** COPY row under the display: Plain /
  LaTeX / Scientific / CSV. LaTeX escapes the printed text and
  wraps the numbers in $...$ — an export of the values and lines
  CalcDex printed, not a re-derived formula (documented limit).
  Scientific rewrites numbers in e-notation (exponents in the
  printed working, like the 2 in 1.75^2, stay plain). CSV emits
  label,value rows for lines that read as `label: value` or
  `label = value`; other lines become a single text row. CLI:
  `CalcDex <id> args --as latex|csv|sci|plain`.
- **#5 Expression Calculator with paper tape.** New Math
  calculator: type `2*(3+4)^2`, `sin(30)`, `sqrt(16)+abs(-3)` —
  operators + − * / ^ (right-associative), parentheses, unary
  minus, functions sin cos tan asin acos atan sqrt ln log10 exp
  abs, constants pi e tau, and a degrees/radians switch. Parse
  errors teach (character position + what was expected). Every
  run is saved to the calculation log — the log is the paper
  tape. CLI: `CalcDex expr "2*(3+4)^2"`. Hand-written parser,
  no new dependency.
- **#6 Unit-aware typed inputs.** Fields that carry a unit
  family accept typed quantities: `5 ft 3 in`, `1.2 kg`, `70 F`,
  `2 h 30 min`, `3 lb 4 oz`. Compound amounts add up; the value
  is converted into the unit the field expects (for converters,
  into the chosen FROM unit). Works in the window and on the
  CLI (`CalcDex bmi "154 lb" "5 ft 9 in"`). Coverage: all
  converter calculators plus the physics set (speed, force,
  energy, power, pressure, Ohm's law, projectile, pendulum,
  wave, torque, momentum and more) and the BMI/BMR/water/protein
  health set. Any other field keeps plain-number entry, and
  anything unparsable gets the usual teaching error.
- **#7 Dimensional check.** When a typed quantity's dimension
  does not match the field's, the run is refused with a plain
  warning ("This field expects a mass, but '175 cm' looks like
  a length value…"). Plain numbers never warn; fields without
  dimension metadata never warn.
- **#8 Compound-unit converters (+5).** Density (kg/m³, g/cm³,
  g/mL, kg/L, g/L, lb/ft³, lb/gal US), Fuel Consumption
  (L/100km ↔ mpg US / mpg UK / km/L — the one non-linear
  converter), Data Rate (bit/s … GiB/s), Concentration (mg/mL,
  g/L, kg/m³, % w/v, ppm, µg/mL, g/mL), Energy Density (J/m³ …
  kWh/m³, BTU/ft³). Each has an example, hints and known-answer
  vectors (1000 kg/m³ = 1 g/cm³; 8 L/100km = 29.401823 mpg US;
  100 Mbps = 12,500 KB/s; 1 % w/v = 10 mg/mL; 3.6 MJ/m³ =
  1 kWh/m³).
- **#10 Spec check.** SPEC CHECK row under the display: enter
  Min / Max and every numeric result line is judged PASS
  (inside, limits inclusive) or FAIL (which limit, by how
  much) — shown as a ✓/✗ symbol, a word, and colour, never
  colour alone. CLI: `CalcDex <id> args --spec MIN MAX`.
- **Built-in help (standing rule).** HELP button in the header
  and F1 open a plain-words guide inside the program: what
  CalcDex is, the five things people do most, keyboard
  shortcuts, where the files live, the command line in 3
  lines. Compiled in — works fully offline. CLI:
  `CalcDex --help-guide` prints the same text.
- **Undo for destructive actions (standing rule).** CLEAR LOG
  now saves the log to `history-backup.txt` first, and the
  history view gains RESTORE CLEARED LOG to undo the clear.
  CLI `--clear-log` / `--restore-log` do the same. Batch runs
  refuse to overwrite an existing output file unless the
  OVERWRITE toggle (window) or `--force` (CLI) is set — and
  with force, the old file is kept as `<name>.bak` first.
- **Drag and drop (standing rule).** Dropping a .csv file on
  the window opens the BATCH panel with the input path filled.
- **Update check, offer only (standing rule).** The HELP panel
  has CHECK FOR UPDATES: it asks github.com for the newest
  CalcDex release and, if one exists, shows the version and
  the release page — it never downloads or installs anything.
  A "Check for updates at startup" toggle (default OFF)
  controls the only automatic check. Otherwise CalcDex makes
  no network connection at all. CLI: `CalcDex --check-update`.
  A failed check is a plain message, never a crash or a hang
  (6-second timeout, via the system curl).
- **Fix found while building:** on startup the window loaded
  the settings file validated against the category names
  instead of the calculator ids, so window-side remembered
  inputs/favourites were re-checked against the wrong list.
  The CLI path was unaffected. Fixed; the shared settings file
  now loads identically in both.
- Tests: 82 green (67 at v0.3.0 + 15 new in tests/v040.rs),
  including registry-wide coverage: all 306 calculators carry
  an example, hints and a known-answer vector.

## 0.3.0 — 2026-10-25

**Batch 1 of the 50-enhancement program — items 1, 11, 16, 20,
31, 34, 46, 47, 48, 50: DONE.** (Program tracker: Batch 1 complete;
next is the rigour batch, items 2–10 + 37.) Also the first release
This release
also applies two standing rules in full — **tooltips for
everything** and **full command-line support** — both recorded in
DECISIONS.md.

- **#1 Significant-figures mode.** Header SIG FIGS selector
  (Off/2/3/4/5/6) and CLI `--sig-figs N`: the headline is ALSO
  shown rounded to N significant figures (pure `format_sig_figs`
  in the library, classic traps tested: 0, negatives, powers of
  ten, 999.9, 0.0000999). Display only — full precision is kept
  everywhere, including the log.
- **#11 Try an example.** Every one of the 300 calculators carries
  hand-written teaching example inputs (registry-wide test: each
  has one, and it runs). TRY AN EXAMPLE button fills + runs in one
  click; `CalcDex <id> --example` does the same on the CLI.
- **#16 Vary-one-input table.** GUI VARY section + CLI
  `<id> --vary <field> <start> <end> <step> <other numbers...>`
  (CSV-friendly output). Pure sweep engine in the library; 200-row
  cap with the cap stated; list calculators and bad sweeps get
  teaching messages.
- **#20 Hints under every input.** Format + unit + code meanings
  (hand-written where the label is not enough — sex/size/unit
  codes, activity factors); hints live in the registry metadata,
  so `--describe <id>` prints them too.
- **#31 Batch from CSV.** GUI BATCH panel (typed paths — no
  file-picker in this toolkit, stated honestly) + CLI `--batch`.
  Hand-written CSV parser (quoted commas supported; no new
  dependency). Bad rows are captured per-row; a header mismatch
  is refused with the expected header named.
- **#34 Linear Regression & Calibration.** New Statistics
  calculator (registry 299 → 300): slope, intercept, r, R², means,
  residuals, substituted sums in the steps, and calibration —
  append one measured y and it predicts x = (y − b)/m. Known
  answers from a textbook dataset (m = 0.6, b = 2.2, R² = 0.6).
- **#46 Favourites + Recent.** ★ toggle per calculator, FAVOURITES
  mode at the top of the rail (Alt+F) with the last 8 run as
  Recent chips.
- **#47 Ctrl+K quick search with synonyms.** Ctrl+K (or `/`)
  focuses search; 80+-entry synonym table (40+ pairs) with
  ranking exact name > synonym > description.
- **#48 Remembered inputs.** Reopening a calculator restores its
  last-used values; RESET restores the smart defaults.
- **#50 Smart defaults everywhere.** Fields open on visible,
  editable defaults (teaching examples; gas laws at STP —
  273.15 K / 1 atm / 22.414 L; dates default to today where they
  did before). AUDIT.md addendum lists the notable ones.
- **Settings file** (shared by #1/#46/#48):
  `%APPDATA%\CalcDex\settings.txt`, a documented line format.
  A corrupt or missing file can never break startup.
- **Tooltips for everything (standing rule).** Every button,
  toggle, field, chip and history control carries a
  plain-words tooltip. All buttons/selectables are built ONLY
  through helper constructors that require a tooltip argument,
  and tests/tooltips.rs scans the GUI source to keep it so.
- **Full command-line support (standing rule).** CLI gains
  `--describe`, `--example`, `--vary`, `--batch`, `--sig-figs`,
  `--favourite` / `--unfavourite` / `--favourites`. CLI runs now
  also write remembered inputs + recent to the shared settings
  file. `docs/PARITY.md` is now a full GUI-vs-CLI parity table.
  One CLI behaviour change: `--search` is now the ranked synonym
  search (same engine as the window), so result order/content for
  a query follows the ranking instead of registry order.
- All v0.2.0 behaviours carry over (Design 3 layout, history
  rail, 299 known-answer vectors + the new regression vectors),
  except the keypad (next item).
- **Removed: the on-screen keypad.** It made the window contents
  uneven. The keypad UI, its focus tracking and its library code
  are gone; the centre column now spans its full width evenly
  (form fields widened). Keyboard flow is unchanged: type in the
  fields, Enter calculates, Alt+ switches modes, Ctrl+K searches. The new GUI panels are **Unverified** on a real
  Windows machine until a field run — fresh v0.3.0 captures are on
  the README list.


## 0.2.0 — 2026-10-25

The **Design 3** release — the layout SEAN picked from the eight
research mockups (Microsoft-Calculator-inspired), replacing the
v0.1.x two-column window. No calculator changed: the registry, the
CLI and every result are exactly as audited at 0.1.2.

- **Mode rail (far left).** The categories are modes — ALL, MATH,
  FINANCE, HEALTH, PHYSICS, CHEM, STATS, CONVERT, BUILD, DATE,
  DAILY, SPORTS, FOOD, ECOLOGY, OTHER — one click each, with the
  v0.1.2 readability styling (bright 13 pt text, green selection
  background + text + marker bar, ellipsis + full-name tooltip).
  Each row prints its Alt+ shortcut; the working set is Alt+1..9,
  Alt+0 for the first ten modes and Alt+Q/W/E/R/T for the last
  five, in rail order. Search stays at the top of the rail.
- **Centre work area.** A chip strip lists the active mode's
  calculators (search still filters across all); below it the
  selected calculator's form is unchanged — labelled inputs with
  units, merged MM/DD/YYYY date fields with format hints and
  Today? buttons, list-input format, teaching errors, pre-filled
  defaults (Age End Date / Countdown From Date = today).
- **Display + steps footer.** Results land in a prominent Display
  (large green type) at the top of the centre; the full result
  text — formula, substituted values, result — sits in the
  STEPS / FULL RESULT footer underneath, complete and scrollable.
- **On-screen keypad** (removed in v0.3.0 — it made the window
  contents uneven).
- **History rail (right).** The calculation log is docked as a
  right-hand rail — newest first, RE-OPEN restores calculator +
  inputs, CLEAR LOG, count in the header. Below a 1100 pt window
  width it collapses and the header LOG button opens the same
  history in the centre (the v0.1.x behaviour). LOG now starts
  shown, matching the mockup.
- Responsive rules kept: resizable rails, fluid detail rows,
  vertical scrolling, the 760 × 520 minimum and zoom reflow from
  0.1.2. The new layout's on-Windows look is **Unverified** until
  SEAN's run — fresh v0.2.0 captures are on the README list.
- Deviations from the mockup are recorded in DECISIONS.md (the
  mockup's consumer mode names and memory row do not exist in
  CalcDex; × ÷ operator keys are omitted on purpose).

## 0.1.2 — 2026-10-25

The readability + responsive + full-audit release, from SEAN's field
test of 0.1.1. (0.1.1 itself is FIELD-VERIFIED: SEAN's screenshots
show the merged date fields, format hints, Today? buttons, the
pre-filled End Date, the calculation log view with its LOG (1)
count, RE-OPEN and CLEAR LOG, and Icon 1 in the title bar; his Age
run 06/04/1977 → 10/26/2026 = 49 years / 18,041 days / 2,577 weeks
was independently re-checked and is exactly correct. His two
captures are now in `docs/screenshots/`.)

- **Left column readability:** list text is now the main text
  colour at 13 pt (measured 12.15:1 contrast on the panel — WCAG AA
  asks 4.5:1; the old muted-grey labels measured 4.36:1), category
  headers are green and bold, rows are taller, the selected row has
  a green background, green text AND a marker bar on its left edge
  (selection is never colour-alone), hover is clearly visible, and
  long names stay on one line with an ellipsis + full-name tooltip
  instead of wrapping mid-name.
- **Responsive window:** the left panel is user-resizable
  (190–400 pt); the detail pane scrolls vertically and its input
  rows reflow from the available width (labels 38%, fields take the
  rest, capped) so nothing clips off the right edge at the new
  760 × 520 minimum size or at 150–200% text zoom; the search box
  stretches with the panel. On-Windows resize behaviour is
  Partially Verified (by construction + code review) until SEAN's
  check — see `docs/AUDIT.md` for the layout mechanisms.
- **Full audit of all 299 calculators** (usability, accuracy,
  context, accessibility) — full report in `docs/AUDIT.md`. Known-
  answer test coverage went from 67 of 299 calculators to **299 of
  299** (expected values computed independently). Formula bugs
  fixed, each pinned by a regression test:
  - Fraction Multiply printed the numerator as a bogus "= value"
    (2/3 × 3/4 showed "= 1"; now "Result: 1/2 (decimal 0.5)").
  - Average Velocity's divide-by-zero guard tested the wrong
    inputs: equal times printed an infinite velocity, and some
    valid problems got a false error. Guard now tests t2 − t1.
  - Ideal Weight (Devine) clamped heights under 5 ft to the 5 ft
    weight (a 4′10″ man got 50.0 kg; the signed formula gives
    45.4 kg).
  - Ratio truncated decimal parts before simplifying (1.5 : 2.5
    became 1 : 2; now correctly 3 : 5).
  - Fraction to Decimal mishandled negative mixed numbers
    (−1 and 1/2 gave −0.5; now −1.5).
  - Nth Root rejected odd roots of negatives (cube root of −27
    now = −3, with a teaching error for even/non-whole roots).
  - Silent-garbage edge cases now teach instead: Double Discount
    at price 0 (was "NaN%"), Effective Rate at n = 0, Rule of 72
    at a negative rate, Sample Size at p = 0 or 1 (was "0 people
    needed"), Dog Age with an invalid size code, and fractional
    converter unit numbers (1.9 was silently read as unit 1).
  - Proportion's second output line was self-contradictory; both
    standard forms are now computed and labelled. Fraction results
    no longer print negative denominators (3/-2 → −3/2).
  - Wording/labels: Speed, Distance and Time calculators state
    their unit assumptions on the labels; Karvonen's "Age" is now
    "Age (years)"; the ideal-gas descriptions no longer repeat the
    gas constant twice; Decimal → Fraction documents its
    9-decimal-place precision limit.

## 0.1.1 — 2026-10-25

Renamed CalcCodex → **CalcDex** (program, project, and repo) and the
"smarter dates + calculation log" release, from SEAN's field test of
0.1.0 (main window and Age Calculator field-verified on his machine;
the Age result 04/23/1988 → 10/25/2026 = 38 years / 14,064 days was
independently re-checked and was exactly right).

- **Merged date fields:** every date calculator now takes one box per
  date in MM/DD/YYYY form instead of separate year/month/day boxes,
  with a plain-words format hint under the field ("for example
  04/23/1988 means April 23, 1988") and teaching errors for dates
  that are not real (Feb 29 2025 fails, Feb 29 2024 works).
- **Clear date labels:** Birth Date / End Date (the old "On year /
  On month / On day" wording is gone everywhere, including results),
  Start Date, From Date, Target Date, Date, and First Day of Last
  Period for the due-date and ovulation calculators.
- **Today? button** beside every date field fills it with today's
  date. The Age calculator's End Date and the Countdown's From Date
  start pre-filled with today.
- **Calculation log:** every successful calculation — in the window
  or on the command line — is saved automatically to
  %APPDATA%\CalcDex\history.txt (newest 500 kept). The LOG button
  in the header opens it (newest first, count shown); RE-OPEN loads
  an entry's calculator with its inputs restored; CLEAR LOG empties
  it. Command line: `CalcDex --log` and `CalcDex --clear-log`.
- **Icon (SEAN's pick, option 1):** phosphor-green calculator
  outline on near-black, redrawn cleanly from the concept sheet
  (its bottom-row overlap glitch fixed), wired into the window,
  the .exe itself, and the installer.
- **CLI dates:** a date is one argument — `CalcDex age 04/23/1988
  10/25/2026` gives exactly the same answer as the old six-number
  form, which still works.
- Field wording in the registry now uses "end date" throughout.


## 0.1.0 — 2026-10-25

First release. Calculators only (SEAN's scope pick), covering the
Omni Calculator categories and the Nutilz calculator subset as one
offline Windows program in the dark Terminal theme.
(0.1.0 was built and field-tested locally under the name CalcCodex;
the program, project, and repo were renamed to CalcDex before any
GitHub publication, so the first published release is 0.1.1.)

- One registry of calculators (exact count in docs/COVERAGE.md and
  proven by a registry test), used identically by the window and the
  command line — no window-only features.
- Categories: Math, Finance, Health, Physics, Chemistry,
  Construction, Conversion, Statistics, Date & Time, Everyday,
  Sports, Food, Ecology, Other.
- Every result shows the standard formula and the steps in plain
  words; estimates are labelled ESTIMATE; bad input gives a clear
  error, never a crash.
- Search with Clear button, category list with counts, labelled
  inputs, CALCULATE, result panel.
- Known-answer tests across every category (tests/known_answers.rs).
- Release shape (SEAN's rule): loose NSIS installer .exe + one
  portable .zip (exe + LICENSE + PORTABLE.txt). Unsigned; no winget.
