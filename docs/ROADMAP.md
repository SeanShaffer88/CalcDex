# Roadmap — what CalcDex does NOT do yet (honest list)

The RPN / programmable mode (Free42 + Plus42 functionality) is
planned in [RPN_PLAN.md](RPN_PLAN.md). The lists below date from
v0.1.1 and have not all been re-checked against v0.9.0; items
shipped since then (dates, log, engineering, units in expressions)
may still appear.

Done since v0.1 (removed from the gap list): merged MM/DD/YYYY date
fields with Today? buttons and end-date wording, CLI one-argument
dates, and the persistent calculation log (v0.1.1, from SEAN's field
test).

Nothing below was silently dropped. Each item is a real Omni/Nutilz
calculator family that needs more than a closed-form formula, or a
deliberate v0.2 candidate.

## Needs live data (conflicts with fully-offline design — decision needed)
- Currency converter with real exchange rates; gold/silver prices;
  crypto converters; stock/investment live prices.
- Weather-dependent calculators (wind chill from live weather, etc. —
  the formula version CAN be added offline; see below).

## Needs big lookup tables / jurisdiction data
- Income tax by country/state/year; US federal/state tax brackets;
  UK tax/National Insurance; sales-tax-by-state auto lookup (the
  rate-as-input calculators ARE in v0.1).
- Mortgage with PMI drop-off schedules and full amortization *tables*
  (v0.1 computes payments/totals; a printable month-by-month schedule
  view is a v0.2 GUI/CLI report).
- Zodiac, Chinese calendar, Hebrew/Islamic calendar conversions.

## Needs multi-list / graphical input the current signature can't do honestly
- Correlation & linear regression (two paired lists) — v0.2 signature
  extension (pair-lists exist; a two-series variant is planned).
- Matrix calculators (add/multiply/inverse/determinant) — needs a
  matrix input convention.
- Graphing / plotting calculators; geometry with drawn shapes.
- Triangle solver from any 3 known parts (law of sines/cosines with
  case selection) — planned as mode-input calculators in v0.2.
- Complex-number arithmetic suite; polynomial factoring beyond
  quadratic formula roots; systems of equations (2x2/3x3) — formulaic,
  v0.2 Math slice.
- Fraction↔decimal with repeating-decimal detection (v0.1 converts
  plainly).

## Formulaic, planned for v0.2 slices (no blocker, just not in v0.1)
- Wind chill & heat index (formulas), dew point.
- More chemistry: molality, normality, ppm↔molarity with molar mass,
  titration, gas density, STP conversions, Avogadro variants.
- More physics: Doppler effect, Snell's law, lens/mirror, buoyancy,
  hydraulic pressure, sound dB (already partly: see Everyday? no —
  add), escape velocity, relativistic KE.
- More construction: concrete block mortar detail, paver calculator,
  topsoil by bag size, soil compaction, stair with landing, roof pitch
  from rise/run as its own calc, paint for ceilings as separate input.
- Date: Easter computus, ISO week number, Julian day number,
  time-zone conversion (table-based, offline-safe).
- Statistics: t-test & chi-square p-values (need distribution
  functions), ANOVA, percentiles of a dataset by rank method choice.
- Health: FFMI, adjusted body weight, pregnancy weight-gain ranges,
  conception date from due date, child growth percentiles (tables).

## Out of scope permanently (SEAN's pick)
- All non-calculator tools: PDF, image, text, developer, generator
  tools from Nutilz (password/UUID/QR generators etc.) and the text
  side of other suites — DevTextTools already covers that family.
