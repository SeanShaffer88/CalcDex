# CalcDex

## TL;DR

CalcDex is one free program for Windows that does almost every kind of
calculation in one place — like having a big box of hundreds of different
calculators (for money, health, school maths, building jobs, cooking,
sports, and much more) that all work with no internet, no ads, and no
sign-up. Pick a calculator, type your numbers, read the answer with the
working shown in plain words. It also works from the command line for
people who like typing commands.

Free forever. Optional donations are allowed but never required and
never unlock anything. No ads, no in-app purchases, no selling data.

## Who it is for

CalcDex is built for **students, teachers, professors, scientists
and engineers** — people who need a result they can check, not just a
number. That is why every calculator:

- shows its **formula, the substituted values, and the result**
  (worked steps you can follow or mark against),
- keeps **full precision** in results (rounded forms are labelled,
  never silently substituted),
- accepts **scientific notation** inputs (e.g. `6.022e23`, `1.5e-3`),
- names the **constants** it uses (g = 9.80665 m/s², R = 0.082057
  L·atm/(mol·K) = 8.314 J/(mol·K), Avogadro 6.02214076×10²³),
- uses correct **units and symbols**, and labels anything that is an
  approximation as an **ESTIMATE** (body-fat, BAC, race prediction,
  carbon footprint and similar),
- explains input errors in words that say *why* there is no answer
  (for example: log of a negative number has no real value).

**For students & teachers:** results read like worked solutions —
formula first, then the numbers put in, then the answer — so homework
and lab calculations can be verified line by line.

**For professionals:** the same registry runs from the command line,
so any calculation in the window can be scripted, repeated and logged
with identical output.

## What it does

CalcDex brings together the calculator sets of two big websites —
Omni Calculator (3,938 calculators) and Nutilz (637+ tools, calculator
subset only) — as one offline desktop program. Where those sites list
hundreds of near-identical pages (for example a separate page for every
unit pair), CalcDex uses one calculator where you pick the units —
see `docs/COVERAGE.md` for the honest mapping.

Categories: Math, Finance, Health, Physics, Chemistry, Construction,
Conversion, Statistics, Date & Time, Everyday, Sports, Food, Ecology,
Other.

Every calculator shows its formula and the steps in plain words, and
handles bad input (dividing by zero, square root of a negative, and so
on) with a clear message instead of crashing.

## The look

Dark "Terminal" theme, exactly SEAN's Purgeon Terminal colours:
near-black background `#0c0c0c`, panels `#121212`, phosphor-green
accent `#4ade80`, Cascadia Mono type, sharp corners everywhere.

## How to use it (window)

Since v0.2.0 the window uses the **Design 3** layout SEAN picked
from the eight research mockups (mode rail / centre work area /
history rail — the pattern Microsoft's Calculator app uses):

1. Run `CalcDex.exe`.
2. **Mode rail (far left):** the categories are *modes* — ALL,
   MATH, FINANCE, HEALTH, and so on. Click a mode to see its
   calculators, or hold **Alt** and press the key printed on the
   mode's row. The full working shortcut set is Alt+1 … Alt+9,
   Alt+0 for the first ten modes, then Alt+Q, Alt+W, Alt+E,
   Alt+R, Alt+T for the last five, in rail order. The **Search**
   box at the top of the rail filters every calculator by name.
3. **Centre:** click a calculator in the chip strip under the mode
   heading, then fill in the labelled boxes (list calculators take
   numbers separated by commas or spaces in one big box). Dates go
   in ONE box per date, in the format the hint under the box
   shows — MM/DD/YYYY, so 04/23/1988 means April 23, 1988. Press
   the green **Today?** button beside any date box to fill in
   today's date (the Age calculator's End Date and the Countdown's
   From Date already start filled with today).
4. Press **CALCULATE** (or Enter). The
   result appears in the **Display** at the top of the centre in
   large green type, with the full formula and worked steps in the
   footer block underneath — nothing is cut down to one line.
5. **History rail (right):** every result is saved there at once
   (see Calculation log below). RE-OPEN on an entry restores its
   calculator and inputs. On a narrow window the rail collapses —
   below 1100 points wide the header **LOG** button shows the same
   history in the centre instead.

## How to use it (command line)

Every calculator works the same from the command line — nothing is
window-only:

```
CalcDex --list
CalcDex --search loan
CalcDex compound-interest 1000 5 10
CalcDex bmi 70 175
CalcDex mean 1 2 3 4
CalcDex convert-length 1 7 3
CalcDex age 04/23/1988 10/25/2026
```

Dates on the command line are one argument in MM/DD/YYYY form, just
like the window (the old six-number form `1988 4 23 2026 10 25`
still works and gives the identical answer).

More command-line tools (v0.3.0 — everything in the window has a
command-line counterpart; see `docs/PARITY.md` for the full table):

```
CalcDex --describe bmi
CalcDex bmi --example
CalcDex compound-interest --vary 1 0 10 2.5 1000 10
CalcDex --batch compound-interest in.csv out.csv
CalcDex --sig-figs 3 compound-interest 1000 5 10
CalcDex --favourite bmi
CalcDex --favourites
```

More command-line tools (v0.4.0 — display formats, copy formats,
expressions, typed quantities, spec check, help, log undo,
update check):

```
CalcDex --format engineering compound-interest 1000 5 10
CalcDex --format auto compound-interest 1000 5 10
CalcDex bmi 70 175 --as csv
CalcDex bmi 70 175 --spec 18.5 25
CalcDex expr "2*(3+4)^2"
CalcDex bmi "154 lb" "5 ft 9 in"
CalcDex --help-guide
CalcDex --restore-log
CalcDex --check-update
```

RPN mode (v0.9.x, command line only for now; window comes later):

```
CalcDex rpn "2 3 + 4 *"           prints 20.0000
CalcDex rpn "1 3 / ALL"           prints 0.333333333333
CalcDex rpn "5 ENTER 3 -" --stack shows T/Z/Y/X
CalcDex rpn "12 STO A" --state work    keep variables between runs
```

Tokens are separated by spaces: numbers (`1.5e-3`), operators,
and HP-42S names (ENTER, SIN, STO, RCL, FIX 4, ...). Errors name a
stable code such as `[RPN-103]`. See `docs/RPN_PLAN.md`.

Numbers go in the same order as the labelled boxes in the window.
For unit converters the two extra numbers pick the units from the
numbered list shown in that calculator's description.
`CalcDex --help` explains the rest.

## New in v0.8.0 — the engineers batch

Electronics and mechanical tools, a materials table, and a
polished calculation log — in the window and on the command
line alike. Registry now **316 calculators**. This batch
completes the 50-enhancement program (items 41–45 and 49).

- **Electronics pack (#41, in Physics).** Resistor Colour Code:
  type the band colours (`red red brown gold`) to read a
  resistor, or type a value (`4700`, `4.7k`) to get its bands.
  Resistors in Series & Parallel gives both totals for one list.
  Voltage Divider gives Vout, the current, and each resistor's
  power.
- **Tolerance Stack-Up (#42, in Construction).** Dimensions as
  (nominal, tolerance) pairs → the total with its worst-case
  range and its RSS (root-sum-square) range.
- **Safety Factor & Margin of Safety (#43).** Strength against
  an applied stress (or a load over an area): FoS, MoS, and a
  plain-language reading — below 1 is called a predicted
  failure.
- **Materials table (#44).** Twelve common materials with
  typical density, resistivity and Young's modulus values,
  labelled as typical values (resistivity is marked
  not-applicable for insulators). `CalcDex --materials` prints
  the table; the header's MATERIALS view is the same table with
  a USE E IN BEAM button per row.
- **Mechanical pack (#45, in Construction).** Bolt Torque
  (T = K × D × F, N·m and ft·lbf) and Beam — Stress &
  Deflection (simply supported, central load, rectangle or
  circle section; E typed in or taken from a materials-table
  code).
- **Log polish (#49).** Search the log, filter it by
  calculator, tag or date, pin entries so the 500-entry trim
  never drops them, and export the filtered set to a text file:
  `CalcDex --log-search`, `--log-filter`, `--log-pin`,
  `--log-export` — and the same controls in the window's log
  view.

## New in v0.7.0 — the scientists batch

Tools for lab and research work, in the window and on the command
line alike. Registry now **309 calculators**.

- **Paste a column (#32).** List calculators (Mean, Standard
  Deviation, Linear Regression and the rest) accept numbers
  separated by commas, spaces, tabs, semicolons or new lines — a
  copied spreadsheet column pastes straight in, in the window
  field and as command-line arguments. A token that is not a
  number is named in the error.
- **Replicates Summary (#33).** One step from a set of repeated
  measurements to the mean, sample standard deviation, standard
  error of the mean, and the 95% confidence interval (Student's
  t for small samples, the normal value beyond 30 degrees of
  freedom).
- **Outlier Check (#36).** Quartiles (Q1, median, Q3), the
  interquartile range, and the 1.5×IQR fences, with any outlying
  values listed — or a plain statement that there are none.
- **Serial Dilution (#35).** A dilution series planned step by
  step: starting concentration, then either transfer + diluent
  volumes or a plain factor per step, giving the concentration
  after every step.
- **Constants library (#38).** Fifteen CODATA 2018 constants
  (c, h, ħ, e, k_B, N_A, R, G, ε0, μ0, m_e, m_p, u, F, σ) usable
  by name in the Expression Calculator — `h*c/500e-9` is the
  energy of a 500 nm photon. The CONSTANTS view lists them with
  an INSERT button; `CalcDex --constants` prints them. Your own
  constants (`CalcDex --define name=value`) are stored in the
  shared settings file and can never replace a built-in.
- **Uncertainty propagation (#40).** In the Expression
  Calculator, `10 ± 0.2` (or `10 +/- 0.2`) carries its
  uncertainty through + − × ÷ ^ and functions by the first-order
  lab rules, and the result prints as value ± uncertainty with
  the percentage — each step names the rule used.
- **Lab notebook (#39).** Any log entry can carry a tag and a
  note (LOG view, or `--log-tag` / `--log-note`); edits back the
  log up first. `--lab-export <file>` writes the log as a plain
  text lab record, oldest first, with the timestamps stated as
  UTC. History files from earlier versions read unchanged.

## New in v0.6.0 — the teachers batch

Tools for the front of the classroom, in the window and on the
command line alike:

- **Projection mode** — a header toggle rendering the window at
  about 1.5x with higher contrast, saved between runs
  (`CalcDex --projection on|off`).
- **Problem sets** — build a set of problems (calculator +
  values) in the SETS view or with `CalcDex set new|add|open`;
  the plain-text set file holds problems only, so it doubles as
  the student assignment. Sets can be generated from a seed
  (`set generate --seed N --count K`) — the same seed always
  makes the identical set.
- **Worksheets and answer keys** — export a printable
  plain-text worksheet (name/date header, working space, no
  answers) and a separate teacher answer key with answers and
  one-line working.
- **Bulk marking** — `CalcDex --mark <set> <answers.csv>` marks
  a whole class CSV (fractions checked exactly) and prints a
  score line per student.
- **Course packs** — the rail filters the calculators to GCSE
  (115), A-Level (161), AP (121) or Undergrad (210) by real
  curriculum fit (`CalcDex --pack GCSE --list`).
- **Demonstration mode** — DEMO hides a run's result values
  until REVEAL is pressed; the log keeps the real results.
- **Formula sheets and class examples** — printable per-category
  formula sheets (`--formula-sheet Math out.txt`) and a saved
  examples library (`--save-example`, `--examples`,
  `--open-example`).

## New in v0.5.0 — the students batch

Tools for learning and teaching, in the window and on the command
line alike:

- **Formula & symbols (#19).** Every calculator shows its formula
  and what each symbol means, in a FORMULA & SYMBOLS section
  under its form. `CalcDex --describe <id>` prints them too.
- **Deeper steps (#12).** The STEPS header selector's Deeper mode
  shows the formula, the formula with your values in it, the
  working, and the result, as four labelled stages.
  Command line: `--steps deeper`.
- **Check my answer (#13).** Type your own answer and be checked:
  exactly for the fraction calculators, within 0.5% for the rest.
  A wrong answer that matches a common slip (100x, 4x, squared,
  sign flipped, ...) names the likely slip. Command line:
  `CalcDex <id> <args> --check <answer>`.
- **Exact mode (#14).** A display format alongside Off / Sig
  figs / Decimals / Engineering / Auto: fraction results are
  labelled exact, pi geometry also shows its exact form (25*pi),
  and the Expression Calculator works rational expressions out
  exactly (1/3 + 1/6 = 1/2). Anything outside that scope keeps
  the decimal answer and says so. Command line: `--format exact`.
- **Practice mode (#15).** The PRACTICE header toggle opens a
  view that generates problems (answer hidden) for the picked
  calculator or a random one, checks your answer, and can reveal
  the worked solution. A seed field makes problem sequences
  reproducible. Date, list, unit-code and lookup-table
  calculators are excluded — the view says so. Command line:
  `CalcDex <id> --practice --seed N [--answer X]`.
- **Graph plot (#17).** The Expression Calculator plots its
  expression in x (range fields included); the quadratic solver
  plots its parabola. Command line: `CalcDex expr "x^2" --plot
  -5 5` prints the plot as CSV sample points.
- **Export worked solution (#18).** EXPORT SOLUTION under the
  steps saves the name, formula, inputs, steps and result as a
  plain-text file. Command line: `CalcDex <id> <args> --export
  <path>`.

## New in v0.4.0 — the rigour batch

Program items 2, 3, 4, 5, 6, 7, 8, 9, 10 and 37, plus four
standing-rule features. Registry now **306 calculators**.
Everything is in the window AND on the command line:

- **One display-format setting.** The header FORMAT selector
  (Off / Sig figs / Decimals / Engineering / Auto) replaces the
  v0.3.0 SIG FIGS selector. Decimals shows a fixed number of
  decimal places; Engineering shows the headline with SI
  prefixes (1.63 k, 4.2 M, 350 m, down to pico); Auto works out
  the significant figures from the numbers you typed and says
  why ("Inputs carry 3 significant figures"). Only one
  annotation line is ever added, a VIEW row under the display
  re-renders the current result in another format without
  changing the setting, and a Rounding choice (Half up /
  Banker's / Truncate) controls how halves round inside every
  format. All of it is display only — full precision is always
  kept and logged. CLI: `--format <mode[:n]>`, `--decimals N`,
  `--rounding half-up|bankers|truncate`.
- **Copy the result four ways.** Buttons under the display
  copy the result as plain text, LaTeX (the printed values and
  lines with the text escaped and numbers in $...$ — an export
  of what CalcDex printed, not a re-derived formula), scientific
  notation, or CSV (label,value rows). CLI: `--as
  latex|csv|sci|plain`.
- **Expression Calculator.** A new Math calculator that takes a
  whole expression — `2*(3+4)^2`, `sin(30)`, `sqrt(16)+abs(-3)`
  — with + − * / ^, parentheses, the functions sin cos tan asin
  acos atan sqrt ln log10 exp abs, constants pi/e/tau and a
  degrees/radians switch. Mistakes get teaching errors (where
  in the expression, and what was expected). Every run is saved
  in the calculation log, which is its paper tape. CLI:
  `CalcDex expr "2*(3+4)^2"`.
- **Typed quantities.** Fields that carry a unit family accept
  amounts typed the way people write them: `5 ft 3 in`,
  `1.2 kg`, `70 F`, `2 h 30 min`, `3 lb 4 oz`. Compound amounts
  add up and are converted into the unit the field expects
  (converters: into the FROM unit you picked). Coverage: all
  converter calculators, the physics set (speed, force,
  energy, power, pressure, Ohm's law, projectile, pendulum,
  wave, torque, momentum and more) and the BMI/BMR/water/
  protein health calculators. Other fields keep plain-number
  entry. If a typed amount's dimension does not match the
  field — a length typed into a mass field — the run stops
  with a plain warning naming both. Plain numbers never warn.
- **Five compound-unit converters.** Density, Fuel Consumption
  (L/100km ↔ mpg US/UK ↔ km/L), Data Rate (Mbps ↔ MB/s and
  more), Concentration (mg/mL ↔ g/L ↔ % w/v) and Energy
  Density — in the Conversion category, with examples, hints
  and known-answer checks like the other converters.
- **Spec check.** Enter Min/Max under the display and each
  numeric result line is judged PASS (inside the limits —
  the limits themselves count as inside) or FAIL, saying
  which limit and by how much, with a ✓/✗ symbol and a word
  as well as colour. CLI: `--spec MIN MAX`.
- **Built-in help.** The header HELP button (or F1) opens a
  plain-words guide inside the program — what CalcDex is,
  the five things people do most, the keyboard shortcuts,
  where the files live, and the command line in 3 lines.
  `CalcDex --help-guide` prints the same text.
- **Undo for clearing the log.** CLEAR LOG saves the log to
  `history-backup.txt` first; RESTORE CLEARED LOG in the
  history view (or `--restore-log`) puts it back. Batch runs
  refuse to overwrite an existing output file unless you
  allow it (OVERWRITE toggle in the window, `--force` on the
  CLI) — and then the old file is kept as a `.bak` copy.
- **Drag and drop.** Drop a .csv file on the window and the
  BATCH panel opens with the input path filled in.

## New in v0.3.0 — Batch 1 of the 50-enhancement program

Ten enhancements (program items 1, 11, 16, 20, 31, 34, 46, 47, 48,
50), every one of them in the window AND on the command line:

- **Significant-figures mode (1).** The header's SIG FIGS selector
  (Off / 2 / 3 / 4 / 5 / 6) adds a rounded showing of the result
  headline next to the full-precision answer — counted correctly
  (0.00456 to 3 figures stays 0.00456; 1628.89 becomes 1.63e3, so
  the count is never ambiguous). Display only: the stored result
  and the log always keep full precision. CLI: `--sig-figs 3`.
- **Try an example (11).** Every calculator has a TRY AN EXAMPLE
  button that fills in a worked teaching example and runs it at
  once. CLI: `CalcDex bmi --example`.
- **Vary one input (16).** The VARY section under the form sweeps
  one input across start/end/step and tables the results (200-row
  cap, stated in the panel). CLI:
  `CalcDex compound-interest --vary 1 0 10 2.5 1000 10` (the field
  is numbered from 0, as in `--describe`; the remaining numbers are
  the other inputs in order).
- **Hints under every input (20).** Every field shows what format
  and unit it wants — and what the codes mean where a field takes
  codes (sex codes, unit numbers, activity factors). CLI:
  `CalcDex --describe bmi` prints them too.
- **Batch from CSV (31).** The header BATCH panel (or
  `CalcDex --batch <id> <input.csv> <output.csv>`) runs one
  calculator over a whole file. The header row names the fields in
  order — for Compound Interest that is:

  ```
  Principal P,Annual rate %,Years
  1000,5,10
  2000,10,5
  ```

  Each output row is the inputs plus the result headline, or
  `ERROR: ...` — one bad row never stops the batch. (In the window
  the file paths are typed: this toolkit has no file-picker
  dialog.) For list calculators the header is the single list
  label and each row's cells are the list values.
- **Linear Regression & Calibration (34).** A new Statistics
  calculator: enter (x, y) pairs and get slope, intercept, r, R²,
  means, residuals, and the working (n, Σx, Σy, Σxy, Σx²). Add one
  extra value at the end — a measured y — and it predicts the x
  that produced it, x = (y − b) / m. Registry now 300 calculators.
- **Favourites + Recent (46).** The ★ FAVOURITE button on any
  calculator stars it into the FAVOURITES mode at the top of the
  rail (Alt+F), which also shows your last 8 calculators as Recent
  chips. CLI: `--favourite bmi`, `--unfavourite bmi`,
  `--favourites`.
- **Ctrl+K quick search with synonyms (47).** Ctrl+K (or `/`)
  jumps to search from anywhere; search understands everyday words
  — "home loan" finds the mortgage calculator, "bmi" finds Body
  Mass Index — and ranks exact names first. 40+ synonym pairs.
- **Remembered inputs (48).** Reopening a calculator restores the
  values you last used in it; RESET puts the smart defaults back.
- **Smart defaults (50).** Every field opens on a realistic,
  visible, editable default (the teaching examples double as the
  defaults): today in date fields, standard temperature/pressure
  in the gas laws (273.15 K, 1 atm, 22.414 L), g = 9.80665 m/s²
  inside the physics formulas, sane 0%/100% bounds elsewhere. The
  notable ones are listed in `docs/AUDIT.md`.

**Tooltips:** every button, toggle, field and key in the window
has a plain-words tooltip saying what it does — no exceptions
(enforced by
construction in the code and checked by a test).

**Settings file:** favourites, recent, remembered inputs and the
display-format choice live in `data\settings.txt` beside the
program (plain text, format documented in the file's code).
On the first run of v0.9.0 or later, a settings file in the old
`%APPDATA%\CalcDex` folder is copied into `data\` automatically;
the original is left untouched as the backup of the move. The window and the
command line read and write the SAME file — it is the shared
source of truth, so a calculator you star or run on the command
line is starred and remembered in the window too.

## Calculation log

Every successful calculation is saved automatically — whether you
ran it in the window or on the command line — so classwork, lab
results and estimates (SEAN's original request was for age
calculations) can be found again later:

- The saved entries live in the **History rail** docked on the
  right of the window, newest first (header **LOG** shows/hides
  it; on narrow windows the same history opens in the centre).
  **RE-OPEN** on an entry loads that calculator with its inputs
  restored; **CLEAR LOG** empties the log — but first saves it
  to `history-backup.txt`, and **RESTORE CLEARED LOG** (shown
  when a backup exists) undoes the clear.
- Or from the command line: `CalcDex --log` prints the log,
  `CalcDex --clear-log` empties it (with the same backup) and
  `CalcDex --restore-log` undoes the clear.
- Entries are kept in `data\history.txt` beside the program (plain text:
  time, calculator, inputs — dates in MM/DD/YYYY form — and result).
  Only the newest 500 entries are kept; log timestamps are UTC.
  Failed calculations are never logged.

## Privacy & network

CalcDex works fully offline and collects nothing. It makes **no
network connection at all** unless you press **Check for
updates** in the HELP panel (or run `CalcDex --check-update`),
or switch on "Check for updates at startup" in the HELP panel
(default OFF). The check only reads the newest release version
from github.com and shows you the release page if it is newer —
it never downloads or installs anything by itself.

## Requirements & building

- Windows 10/11 for the shipped program (it is a standard Windows
  .exe; it also builds and runs on Linux for development).
- To build from source: a recent Rust toolchain, then:
  `cargo build --release` — and the full check is
  `cargo fmt + cargo clippy --all-targets + cargo test --locked`.

## Install

Every release ships one 64-bit (x64) build in two forms, plus
a checksum file:

- `CalcDex-v{version}-x64-Setup.exe` — the installer (per-user,
  no admin; installs to %LOCALAPPDATA%\Programs\CalcDex).
- `CalcDex-v{version}-x64-Portable.zip` — the portable build:
  the .exe, this README, the license and a short note. Unzip
  anywhere and run; the program keeps its files in a `data`
  folder beside the .exe.
- `SHA256SUMS.txt` — the SHA-256 of both downloads; check your
  copy against it before running.

The program is unsigned (no code-signing certificate) — Windows may
warn on first run; choose "More info", then "Run anyway", if you trust
this build.

## Screenshots

Real captures from SEAN's machine, taken during the v0.1.1 field
test (2026-10-25). They show the OLD left column — v0.1.2 makes the
lists brighter, larger, and single-line; a fresh main-window capture
after the readability fix is still needed.

![Age Calculator in v0.1.1 — merged Birth Date and End Date fields with format hints and Today? buttons](docs/screenshots/age-calculator.png)

*Age Calculator (v0.1.1 capture): one MM/DD/YYYY box per date, the
format hint under each field, and a Today? button beside each date.
The result shown — 06/04/1977 to 10/26/2026 = 49 years / 18,041
days / 2,577 weeks — was independently re-checked and is exactly
right.*

![Calculation Log in v0.1.1](docs/screenshots/calculation-log.png)

*Calculation Log (v0.1.1 capture): every calculation is saved
automatically; RE-OPEN restores an entry's calculator and inputs,
CLEAR LOG empties the log. Timestamps are UTC.*

**Captures still needed (real ones only, no placeholders):** a fresh
main-window shot of **v0.4.0** showing the Design 3 layout with the
rigour features (mode rail, display with its format/copy/spec rows,
history rail, hints under the fields), ideally one at full width
and one narrow enough that the history rail has collapsed.

## Limitations (honest)

- Formula calculators only: the very specialised Omni pages that need
  live data (currency exchange rates, weather, stock prices), big
  lookup tables (tax brackets by country/year), medical diagnosis, or
  drawing graphs are not in v0.1 — they are listed in
  `docs/ROADMAP.md`, not silently dropped.
- Health and building results are estimates from the standard
  published formulas named in each calculator. They are not medical
  or engineering advice.
- See `docs/PARITY.md` for category-by-category scoring from test
  evidence.

## License

MIT — see `LICENSE`. Free software; use it, share it, change it.
