# Parity — CalcDex v0.8.0

Scored ONLY from test/build evidence in this repo (Elite rule).
"Covered" means a registered calculator exists AND the category has
known-answer test coverage. Percentages are the share of the *source
sites' distinct operations* (after the parametric collapsing explained
in COVERAGE.md) judged covered by the registry — an integrator
judgement, labelled as such, not a website page count.

| Capability area | Evidence | Score |
|---|---|---|
| Math operations | Registry (Math module) + math_known_answers tests | Covered — core set |
| Finance operations | Registry + finance_known_answers (compound interest 1628.89 vector etc.) | Covered — core set |
| Health estimates | Registry + health_known_answers (BMI, BMR, due date) | Covered — core set, estimates labelled |
| Physics | Registry + physics_known_answers | Covered — core set |
| Chemistry | Registry + chemistry_known_answers (pH, gas law, dilution, half-life) | Covered — core set |
| Construction (Nutilz strength) | Registry + construction_known_answers (concrete bags, paint, stairs) | Covered — core set |
| Conversion (all Omni pair pages) | Parametric converters + conversion_known_answers + v040 compound vectors (density, fuel, data rate, concentration, energy density) | Covered by collapsing (see COVERAGE.md); +5 compound-unit converters at v0.4.0 (#8) |
| Statistics | List/pair registry + stats_known_answers + v030 regression vectors (textbook dataset m=0.6, b=2.2, R²=0.6) | Covered — core set, incl. Linear Regression & Calibration (v0.3.0 #34); two-series stats beyond regression = roadmap |
| Date & Time | Real calendar algorithm + dates_known_answers + v011 tests (MM/DD/YYYY round-trip, CLI date-arg equivalence) | Covered — merged date fields, Today? button, Birth/End Date wording |
| Sports/Food/Ecology/Other | Registry + sports_food_eco_known_answers | Covered — representative set |
| Error handling (safety) | errors_are_clear_and_never_panic test + v030 teaching-error tests (vary/batch) | Covered for tested paths |
| Batch-1 features (#1/#11/#16/#20/#31/#46/#47/#48/#50) | features.rs / meta.rs / settings.rs + tests/v030.rs (11 tests incl. registry-wide example/default/hint coverage over all 300 calculators) + CLI smoke | Covered by construction + tests; new GUI panels Unverified on Windows until a field run |
| Rigour features (#2/#3/#4/#5/#6/#7/#8/#9/#10/#37) | rigour.rs (display formats + rounding, copy formatters, expression parser, quantity parser, spec check) + meta.rs unit metadata + tests/v040.rs (15 tests: formatter boundaries, parser precedence/errors, quantity conversions, dimension warnings, compound vectors, spec verdicts, CLI end-to-end) | Covered by construction + tests; registry 306, all with examples/hints/vectors; new GUI rows Unverified on Windows until a field run |
| Students features (#12/#13/#14/#15/#17/#18/#19) | meta.rs FORMULAS/GLOSSARY (306/306 rows, registry-wide test) + students.rs engines (rational type, slip detectors, seeded practice, shared plot sampler, solution writer) + tests/v050.rs (19 tests: registry-wide formula/glossary presence, deeper stages across categories, exact evaluator incl. fallbacks, slip table, fraction-exact checks, RNG determinism, sampler gaps, CLI end-to-end) | Covered by construction + tests; new GUI panels (FORMULA & SYMBOLS, CHECK, PRACTICE, PLOT, EXPORT, STEPS toggle) Unverified on Windows until a field run |
| Teachers features (#21–#30) | teachers.rs (set format + parser, worksheet/key renderers, seeded generation over the practice engine, marking over the #13 checker, formula sheets over the #19 metadata, examples library) + meta.rs COURSE_PACKS (GCSE 115 / A-Level 161 / AP 121 / Undergrad 210) + tests/v060.rs (15 tests: set round trip + no-answers-in-file, worksheet/key contents, generation determinism byte-for-byte, 3-student marking fixture with exact totals, formula-sheet coverage ≥ category count, pack validity + spot checks, examples round trip with escapes, demo masking, projection settings round trip + legacy load, CLI end-to-end for every verb) | Covered by construction + tests; the SETS view, PROJECTION and DEMO toggles and COURSE PACKS rail section are Unverified on Windows until a field run |
| Scientists features (#32/#33/#35/#36/#38/#39/#40) | lib.rs (one shared parse_value_list, t_crit_95 table, 8-field history + annotate/lab-record) + rigour.rs (CONSTANTS CODATA table + custom-constant store, ± uncertainty evaluator) + stats.rs replicates-summary/outlier-check + chemistry.rs serial-dilution + tests/v070.rs (14 tests: parser separators/bad-token, replicates known answers vs independent computation, t-table spots, Tukey outlier vectors, dilution both modes + validation, CODATA values + photon-energy identity to 1e-30, constant validation/persistence, every propagation rule + grammar errors, lab round trip + legacy 6-field parse + backup-first annotate, CLI end-to-end incl --constants/--define/--log-tag/--log-note/--lab-export) | Covered by construction + tests; registry 309, all with examples/hints/formulas; the CONSTANTS view and the LOG tag/note editor are Unverified on Windows until a field run |
| Engineers features (#41/#42/#43/#44/#45) | engineers.rs (MATERIALS table of 12 typical-value rows + resistor colour decode/encode engine) + calcs/engineering.rs (resistor-colour-code text calc, resistors-series-parallel, voltage-divider, tolerance-stack-up pair calc, safety-factor mode calc, bolt-torque, beam-bending with material codes) + meta.rs examples/hints/formulas/glossary rows + main.rs --materials + MATERIALS view with USE E IN BEAM + tests/v080.rs (15 tests: decode/encode known answers incl. the brief's 220 Ω and 4,700 Ω vectors and the 5-band fallthrough, divider/series-parallel vectors, stack-up worst-case 35 ± 0.35 and RSS 0.229129, FoS both modes + the below-1 failure wording, bolt torque 20 N·m vector, beam I/σ/δ vectors + the material-code path byte-equal to the typed-E path, table value pins: copper 8960 / steel E 200 / stainless 7.2e-7 / glass resistivity honestly absent, CLI end-to-end) | Covered by construction + tests; registry 316, all with examples/hints/formulas; the MATERIALS view and the beam material fill are Unverified on Windows until a field run |
| Log polish (#49) | lib.rs (LogEntry pin flag with a 9th field written only on pinned lines, cap counting unpinned entries only, LogFilter + filter_entries with global numbers, log_export_text, backup-first set_entry_pinned_at) + main.rs --log-search/--log-filter/--log-pin/--log-unpin/--log-export + the log view's search/filter/pin/export controls + tests/v080.rs (pin round trip, legacy 6- and 8-field lines parsing unpinned, cap survival with 500 unpinned + 1 pinned fixture, backup-first pin edit, filter semantics + global numbers, filtered export contents, CLI end-to-end incl. bad-key and out-of-range refusals) | Covered by construction + tests; the log view's new controls are Unverified on Windows until a field run |
| Standing-rule features (help, log undo, batch overwrite, drag-drop, update check) | HELP_TEXT compiled in (same text for window + `--help-guide`); lib log backup/restore functions + v040 round-trip test; batch overwrite refusal tested end-to-end via the CLI binary; version compare + tag parse tested pure; the update FETCH is network-dependent | Covered by construction + tests, except the update fetch itself — Unverified by nature (offline build machine); drag-drop uses the egui dropped-files API, construction-verified |
| Tooltips (standing rule) | tip_* helper constructors in main.rs + tests/tooltips.rs source scan | Covered by construction + gate test |
| Calculation log | lib history functions + tests/v011 + CLI --log/--clear-log | Covered — FIELD-VERIFIED at v0.1.1 |
| Live-data calculators | Not implemented (offline design) | Gap — ROADMAP |
| Matrix / graphing | Not implemented | Gap — ROADMAP |
| Jurisdiction tax tables | Not implemented (rate-as-input versions exist) | Gap — ROADMAP |

## GUI vs CLI parity (standing rule: full command-line support)

Every capability in the window has a command-line counterpart
below. The settings file (%APPDATA%\CalcDex\settings.txt) and the
history file are shared by both — they are the single source of
truth, not two copies.

| Window capability | Command line |
|---|---|
| Pick a mode (category) and a calculator | `CalcDex --list`, `CalcDex --search <words>`, then the calculator id |
| Search with synonyms + ranking | `--search` (same `search_ranked` engine; Ctrl+K/`/` only move focus) |
| Fill fields and press CALCULATE | `CalcDex <id> <numbers...>` (same registry dispatch, identical output) |
| List field accepts a pasted column (#32: comma/semicolon/tab/space/new-line separated) | The same `parse_value_list` parses CLI arguments — one quoted argument may carry a whole newline-separated column; bad tokens are named in the error on both surfaces |
| CONSTANTS view (#38: CODATA listing + INSERT into the expression field) | `CalcDex --constants` prints the same table (INSERT is the window's way of typing the symbol — the CLI types it directly); user constants: `CalcDex --define <name>=<value>` writes the same settings `constant.` lines the view edits |
| Expression with constants / uncertainties (#38/#40) | `CalcDex expr "h*c/500e-9"` and `CalcDex expr "(10 ± 0.2) * (5 ± 0.1)"` — same parser, same propagation, same output |
| LOG view tag/note per entry + editor (#39) | `CalcDex --log-tag <n> <tag>` / `--log-note <n> <note>` edit the same history file with the same #1 = newest numbering and the same backup-first write; `--log` prints tags and notes |
| Lab record export (#39) | `CalcDex --lab-export <file>` writes the same chronological record the library builds (`lab_record_text`) |
| Today? button beside a date field | Pass today's date as one MM/DD/YYYY argument |
| TRY AN EXAMPLE | `CalcDex <id> --example` |
| RESET (smart defaults) | The defaults ARE the example values — `CalcDex <id> --example` runs them; `--describe` shows them |
| Hints under fields | `CalcDex --describe <id>` prints every field + hint |
| FORMAT selector (Off / Sig figs / Decimals / Engineering / Auto / Exact) | `--format <mode[:n]>` (incl. `exact`), `--decimals N`, `--sig-figs N` (same single annotation; log keeps full precision both ways) |
| FORMULA & SYMBOLS section under the form (#19) | `CalcDex --describe <id>` prints the formula + every symbol's meaning (same meta.rs data) |
| STEPS Standard / Deeper header toggle (#12) | `CalcDex <id> <args> --steps deeper` prints the same four labelled stages; the choice is the shared `steps=` settings key |
| CHECK MY ANSWER panel (#13) | `CalcDex <id> <args> --check <answer>` prints the same verdict (exact for fractions, 0.5% otherwise, named likely slip) |
| PRACTICE view (#15: problem generation, check, reveal, tally, seed) | `CalcDex <id> --practice --seed N` prints the problem; `--answer X` re-makes it from the seed, checks it and prints the worked solution. The tally is a window-session display (memory only, labelled) — its CLI counterpart is the per-problem check itself |
| PLOT section (#17: Expression Calculator + quadratic) | `CalcDex expr "<expr in x>" --plot <from> <to>` prints the same samples as CSV x,y (blank y = undefined); documented as the CLI form of the plot; same library sampler |
| EXPORT SOLUTION button + path field (#18) | `CalcDex <id> <args> --export <path>` writes the identical solution text (same library writer) |
| VIEW override row + ROUNDING row under the display | The same `--format` / `--rounding` flags for that one run |
| COPY row (Plain / LaTeX / Scientific / CSV) | `CalcDex <id> args --as latex\|csv\|sci\|plain` prints the run in that format |
| SPEC CHECK row (Min / Max → PASS/FAIL lines) | `CalcDex <id> args --spec MIN MAX` prints the same verdict lines |
| Expression Calculator form (expression box + DEG/RAD) | `CalcDex expr "<expression>"` (also by id); runs log identically — the log is the paper tape |
| Typed quantities in unit-family fields | The same parser runs in CLI argument expansion (`CalcDex bmi "154 lb" "5 ft 9 in"`); dimension warnings refuse the run in both |
| VARY ONE INPUT panel | `CalcDex <id> --vary <field> <start> <end> <step> <others...>` |
| BATCH panel (incl. dropping a .csv on the window to fill the input path, and the OVERWRITE toggle) | `CalcDex --batch <id> <in.csv> <out.csv>` (same engine; the panel only adds typed path fields; the drop only fills the input path — the capability is the batch run itself; overwrite needs `--force`, which keeps a `.bak`) |
| Display + steps footer | The full result text is printed verbatim |
| History rail, RE-OPEN | `--log` prints entries with their inputs — re-running with those inputs is the restore |
| CLEAR LOG (backs up first) + RESTORE CLEARED LOG | `--clear-log` (same backup) + `--restore-log` |
| HELP button / F1 help panel | `CalcDex --help-guide` prints the identical compiled-in text |
| CHECK FOR UPDATES button + "at startup" toggle (HELP panel) | `CalcDex --check-update` prints the verdict; the toggle lives in the shared settings file both read |
| ★ FAVOURITE toggle + FAVOURITES mode | `--favourite <id>` / `--unfavourite <id>` / `--favourites` |
| RECENT chips | `--favourites` prints the recent list (CLI runs update it too) |
| Remembered inputs | The shared settings file: CLI runs write them, the window reads them (and vice versa); `--favourites` reports how many calculators have saved inputs |
| Mode rail + Alt+ shortcuts | Navigation only — the CLI names the calculator directly, which is the same capability in one step |
| PROJECTION header toggle (#21) | `CalcDex --projection [on|off]` reads/writes the same shared `projection=` settings key; the window applies zoom + contrast from it |
| COURSE PACKS rail section (#24) | `CalcDex --pack <GCSE|A-Level|AP|Undergrad> [--list]` prints the pack's calculators; `--describe <id>` names a calculator's packs (same meta.rs tables) |
| DEMO header toggle + REVEAL / HIDE AGAIN (#25) | `CalcDex <id> <args> --demo` prints the working with result values masked; `--demo --reveal` prints them. The log stores the real result on both surfaces (documented in DECISIONS.md) |
| SETS view: build a set (name, add current, remove, LOAD entry) (#22/#30) | `CalcDex set new <file> [name...]`, `CalcDex set add <file> <id> <inputs...>`, `CalcDex set open <file>` (lists problems). Removing an entry and loading one into the form are file/form editing of the same shared format — the CLI edits set files with new/add and prints entries with open |
| SETS view: SAVE SET / OPEN SET (#22) | The same verbs; both surfaces read and write the one documented set format (teachers.rs) |
| SETS view: EXPORT ANSWER KEY (#22) | `CalcDex set key <file> <key-file>` writes the identical key text (same library renderer) |
| SETS view: EXPORT WORKSHEET (#27) | `CalcDex set worksheet <file> <out>` writes the identical worksheet text |
| SETS view: GENERATE FROM A SEED (#23) | `CalcDex set generate <file> --seed N --count K <id>...` — same generator, byte-identical files for the same seed |
| SETS view: MARK A CLASS (#26) | `CalcDex --mark <set-file> <answers.csv> [marked.csv]` — same marking engine, same output CSV + summary lines |
| SETS view: FORMULA SHEET (#29) | `CalcDex --formula-sheet <Category|All> <file>` — same library renderer over the #19 metadata |
| SETS view: SAVED CLASS EXAMPLES (save / load / delete) (#28) | `CalcDex <id> <args> --save-example "<title>" [--note "<note>"]`, `CalcDex --examples`, `CalcDex --open-example "<title>"` — the same %APPDATA%\CalcDex\examples.txt library; deleting an entry is editing that shared file |

Window-only capabilities: **none**.

The v0.2.0 on-screen keypad was removed in v0.3.0 (it made the
window contents uneven); keyboard entry covers all input, in the
window and on the command line alike.

Audience fit (audience: students, teachers,
professors, scientists, engineers): formula + substituted values +
result in outputs; full precision; scientific-notation inputs;
constants named; estimates labelled. Implemented in the registry
output text and README — Verified by CLI output; GUI presentation
of the v0.4.0, v0.5.0, v0.6.0 and v0.7.0 panels is Unverified until a fresh field run.

## RPN mode (P1)

| Capability | Window | Command line |
|---|---|---|
| RPN stack machine (P1) | Not yet (phase P9) | `CalcDex rpn "<tokens>"` — Verified by tests/rpn_p1.rs |
